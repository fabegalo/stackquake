//! Google sign-in for desktop apps (RFC 8252):
//! system browser + loopback redirect on 127.0.0.1 + PKCE (S256) + `state`.
//!
//! - Users only click "Sign in with Google"; the OAuth client is embedded at build time.
//! - The refresh token goes to the OS keychain; the access token only lives in memory.
//! - No token is ever sent to the UI.

use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use url::Url;
use zeroize::Zeroizing;

use crate::error::{AppError, AppResult};
use crate::http::GoogleHttp;
use crate::secrets;

/// API scopes (required). The Crashlytics API only accepts `firebase` or
/// `cloud-platform` (there is no read-only variant); `firebase` is the
/// narrower one. To compensate, `http.rs` only allows GET to Firebase hosts.
pub const SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/playdeveloperreporting",
    "https://www.googleapis.com/auth/firebase",
];
/// Identify the account (to keep each account's data separate). Non-sensitive.
const IDENTITY_SCOPES: &[&str] = &["openid", "email"];

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// The app's OAuth client ("Desktop app"), embedded at build time by
/// `build.rs`. No user ever creates or pastes credentials.
///
/// For installed apps, Google and RFC 8252 (§8.5) do not treat the
/// `client_secret` as confidential: PKCE + loopback protect the flow.
/// It still stays out of the repository and is injected at build time.
pub struct OAuthClient {
    pub client_id: &'static str,
    pub client_secret: &'static str,
}

impl OAuthClient {
    pub fn embedded() -> AppResult<Self> {
        match (
            option_env!("STACKQUAKE_GOOGLE_CLIENT_ID"),
            option_env!("STACKQUAKE_GOOGLE_CLIENT_SECRET"),
        ) {
            (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => Ok(Self {
                client_id: id,
                client_secret: secret,
            }),
            _ => Err(AppError::NoOAuthClient),
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
    scope: Option<String>,
    id_token: Option<String>,
}

/// Signed-in Google account. `sub` is the account's stable Google ID.
#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
    pub sub: String,
    pub email: String,
}

impl Account {
    pub fn current() -> AppResult<Option<Self>> {
        match secrets::get(secrets::ACCOUNT)? {
            Some(raw) => Ok(serde_json::from_str(&raw).ok()),
            None => Ok(None),
        }
    }

    /// Identifier of this account's local folder/key (does not reveal the email).
    pub fn local_id(&self) -> String {
        Sha256::digest(self.sub.as_bytes())[..12]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Reads the `id_token` received directly from Google's token endpoint over
    /// TLS. In that case OIDC (Core §3.1.3.7) does not require validating the
    /// signature; we check issuer, audience and expiry.
    fn from_id_token(id_token: &str, client_id: &str) -> AppResult<Self> {
        let bad = || AppError::auth("auth_bad_identity");
        let payload = id_token.split('.').nth(1).ok_or_else(bad)?;
        let bytes = B64
            .decode(payload.trim_end_matches('='))
            .map_err(|_| bad())?;
        let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| bad())?;
        let iss_ok = matches!(
            v["iss"].as_str(),
            Some("https://accounts.google.com" | "accounts.google.com")
        );
        let aud_ok = v["aud"].as_str() == Some(client_id);
        let exp_ok = v["exp"]
            .as_i64()
            .is_some_and(|e| e > chrono::Utc::now().timestamp());
        let sub = v["sub"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 255);
        match (iss_ok && aud_ok && exp_ok, sub) {
            (true, Some(sub)) => Ok(Self {
                sub: sub.to_string(),
                email: v["email"]
                    .as_str()
                    .unwrap_or_default()
                    .chars()
                    .take(320)
                    .collect(),
            }),
            _ => Err(bad()),
        }
    }
}

struct CachedToken {
    token: Zeroizing<String>,
    expires_at: Instant,
}

#[derive(Default)]
pub struct Auth {
    cached: Mutex<Option<CachedToken>>,
}

impl Auth {
    pub fn is_signed_in(&self) -> AppResult<bool> {
        Ok(secrets::get(secrets::REFRESH_TOKEN)?.is_some() && Account::current()?.is_some())
    }

    /// Full sign-in; returns the account that signed in.
    pub async fn sign_in(&self, app: &AppHandle, http: &GoogleHttp) -> AppResult<Account> {
        let client = OAuthClient::embedded()?;

        let verifier = Zeroizing::new(B64.encode(*secrets::random_bytes::<32>()?));
        let challenge = B64.encode(Sha256::digest(verifier.as_bytes()));
        let state = Zeroizing::new(B64.encode(*secrets::random_bytes::<16>()?));

        // Random port, loopback interface only.
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let redirect_uri = format!("http://127.0.0.1:{}", listener.local_addr()?.port());

        let mut url = Url::parse(AUTH_URL).expect("valid constant URL");
        url.query_pairs_mut()
            .append_pair("client_id", client.client_id)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("response_type", "code")
            .append_pair(
                "scope",
                &SCOPES
                    .iter()
                    .chain(IDENTITY_SCOPES)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" "),
            )
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state)
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");

        app.opener()
            .open_url(url.as_str(), None::<&str>)
            .map_err(|e| AppError::auth_with("auth_browser", vec![("detail", e.to_string())]))?;

        let code = tokio::time::timeout(LOGIN_TIMEOUT, wait_for_code(&listener, &state))
            .await
            .map_err(|_| AppError::auth("auth_timeout"))??;
        drop(listener);

        let tr: TokenResponse = http
            .post_form(
                Url::parse(TOKEN_URL).expect("valid constant URL"),
                &[
                    ("code", code.as_str()),
                    ("client_id", client.client_id),
                    ("client_secret", client.client_secret),
                    ("redirect_uri", &redirect_uri),
                    ("grant_type", "authorization_code"),
                    ("code_verifier", verifier.as_str()),
                ],
            )
            .await?;

        // With granular consent, the user may uncheck permissions.
        let granted = tr.scope.as_deref().unwrap_or_default();
        let missing: Vec<&str> = SCOPES
            .iter()
            .copied()
            .filter(|s| !granted.split(' ').any(|g| g == *s))
            .collect();
        if !missing.is_empty() {
            return Err(AppError::auth_with(
                "auth_missing_scopes",
                vec![("scopes", missing.join(", "))],
            ));
        }
        let account = Account::from_id_token(
            tr.id_token
                .as_deref()
                .ok_or_else(|| AppError::auth("auth_no_identity"))?,
            client.client_id,
        )?;
        let refresh = Zeroizing::new(
            tr.refresh_token
                .ok_or_else(|| AppError::auth("auth_no_refresh_token"))?,
        );
        secrets::set(secrets::REFRESH_TOKEN, &refresh)?;
        secrets::set(secrets::ACCOUNT, &serde_json::to_string(&account)?)?;
        *self.cached.lock().await = Some(CachedToken {
            token: Zeroizing::new(tr.access_token),
            expires_at: Instant::now() + Duration::from_secs(tr.expires_in),
        });
        Ok(account)
    }

    /// Access token valid for at least another 60 s, refreshing if needed.
    pub async fn access_token(&self, http: &GoogleHttp) -> AppResult<Zeroizing<String>> {
        let mut guard = self.cached.lock().await;
        if let Some(t) = guard.as_ref() {
            if t.expires_at > Instant::now() + Duration::from_secs(60) {
                return Ok(t.token.clone());
            }
        }
        let refresh = secrets::get(secrets::REFRESH_TOKEN)?.ok_or(AppError::NotSignedIn)?;
        let client = OAuthClient::embedded()?;
        let result: AppResult<TokenResponse> = http
            .post_form(
                Url::parse(TOKEN_URL).expect("valid constant URL"),
                &[
                    ("client_id", client.client_id),
                    ("client_secret", client.client_secret),
                    ("refresh_token", refresh.as_str()),
                    ("grant_type", "refresh_token"),
                ],
            )
            .await;
        let tr = match result {
            Ok(tr) => tr,
            // Revoked or expired token: requires signing in again.
            Err(AppError::Api {
                status: 400 | 401, ..
            }) => {
                secrets::delete(secrets::REFRESH_TOKEN)?;
                return Err(AppError::NotSignedIn);
            }
            Err(e) => return Err(e),
        };
        let token = Zeroizing::new(tr.access_token);
        *guard = Some(CachedToken {
            token: token.clone(),
            expires_at: Instant::now() + Duration::from_secs(tr.expires_in),
        });
        Ok(token)
    }

    /// Revokes the token at Google and deletes it from the keychain.
    pub async fn sign_out(&self, http: &GoogleHttp) -> AppResult<()> {
        if let Some(rt) = secrets::get(secrets::REFRESH_TOKEN)? {
            // The revoke response is not useful JSON; what matters is deleting locally.
            let _ = http
                .post_form::<serde_json::Value>(
                    Url::parse(REVOKE_URL).expect("valid constant URL"),
                    &[("token", rt.as_str())],
                )
                .await;
        }
        secrets::delete(secrets::REFRESH_TOKEN)?;
        secrets::delete(secrets::ACCOUNT)?;
        *self.cached.lock().await = None;
        Ok(())
    }
}

/// Waits for the browser redirect. Ignores requests without parameters
/// (e.g. /favicon.ico) and only accepts the one carrying the right `state`.
async fn wait_for_code(
    listener: &TcpListener,
    expected_state: &str,
) -> AppResult<Zeroizing<String>> {
    loop {
        let (mut stream, peer) = listener.accept().await?;
        if !peer.ip().is_loopback() {
            continue;
        }
        let mut buf = Zeroizing::new(vec![0u8; 8192]);
        let n = match tokio::time::timeout(Duration::from_secs(10), stream.read(&mut buf)).await {
            Ok(Ok(n)) => n,
            _ => continue,
        };
        let request = String::from_utf8_lossy(&buf[..n]);
        let target = request
            .lines()
            .next()
            .and_then(|l| l.strip_prefix("GET "))
            .and_then(|l| l.split(' ').next())
            .unwrap_or("/");
        let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
            respond(&mut stream, 400, PAGE_ERROR).await;
            continue;
        };
        let get = |k: &str| {
            url.query_pairs()
                .find(|(key, _)| key == k)
                .map(|(_, v)| Zeroizing::new(v.into_owned()))
        };
        let (code, state, error) = (get("code"), get("state"), get("error"));
        if code.is_none() && error.is_none() {
            respond(&mut stream, 404, PAGE_ERROR).await;
            continue;
        }
        let state_ok = state
            .as_ref()
            .is_some_and(|s| secrets::ct_eq(s.as_bytes(), expected_state.as_bytes()));
        if !state_ok {
            respond(&mut stream, 400, PAGE_ERROR).await;
            return Err(AppError::auth("auth_csrf"));
        }
        if let Some(err) = error {
            respond(&mut stream, 200, PAGE_ERROR).await;
            // Only forwards OAuth error codes (never reflects arbitrary text).
            let safe: String = err
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                .take(64)
                .collect();
            return Err(AppError::auth_with("auth_denied", vec![("reason", safe)]));
        }
        respond(&mut stream, 200, PAGE_OK).await;
        return code.ok_or_else(|| AppError::auth("auth_no_code"));
    }
}

async fn respond(stream: &mut tokio::net::TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Bad Request",
    };
    let resp = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\n\
         Referrer-Policy: no-referrer\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\
         Content-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.shutdown().await;
}

// Static pages (English + Portuguese): nothing from the request is reflected.
const PAGE_OK: &str = "<!doctype html><meta charset=utf-8><title>Stackquake</title>\
<body style=\"font-family:system-ui;padding:3rem;text-align:center\">\
<h2>Signed in</h2><p>You can close this tab and return to Stackquake.</p>\
<p lang=pt-BR>Login concluído. Pode fechar esta aba e voltar ao Stackquake.</p></body>";
const PAGE_ERROR: &str = "<!doctype html><meta charset=utf-8><title>Stackquake</title>\
<body style=\"font-family:system-ui;padding:3rem;text-align:center\">\
<h2>Sign-in failed</h2><p>Return to Stackquake and try again.</p>\
<p lang=pt-BR>Não foi possível concluir o login. Volte ao Stackquake e tente de novo.</p></body>";

#[cfg(test)]
mod tests {
    use super::*;

    fn token(claims: serde_json::Value) -> String {
        format!("x.{}.sig", B64.encode(claims.to_string()))
    }

    #[test]
    fn reads_account_from_id_token() {
        let exp = chrono::Utc::now().timestamp() + 3600;
        let ok = token(serde_json::json!({
            "iss": "https://accounts.google.com", "aud": "cid", "exp": exp,
            "sub": "1234567890", "email": "a@b.com"
        }));
        let a = Account::from_id_token(&ok, "cid").unwrap();
        assert_eq!(
            (a.sub.as_str(), a.email.as_str()),
            ("1234567890", "a@b.com")
        );
        assert_eq!(a.local_id().len(), 24);

        let wrong_aud = token(serde_json::json!({
            "iss": "https://accounts.google.com", "aud": "other", "exp": exp, "sub": "1"
        }));
        assert!(Account::from_id_token(&wrong_aud, "cid").is_err());
        let expired = token(serde_json::json!({
            "iss": "accounts.google.com", "aud": "cid", "exp": 1, "sub": "1"
        }));
        assert!(Account::from_id_token(&expired, "cid").is_err());
        assert!(Account::from_id_token("garbage", "cid").is_err());
    }
}
