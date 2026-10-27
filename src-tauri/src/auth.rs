//! Login com Google para aplicativos desktop (RFC 8252):
//! navegador do sistema + redirecionamento loopback em 127.0.0.1 + PKCE (S256) + `state`.
//!
//! - O usuário só clica em "Entrar com Google"; o cliente OAuth vem embutido no build.
//! - O refresh token vai para o chaveiro do sistema; o access token só existe em memória.
//! - Nenhum token é enviado para a interface.

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

/// Escopos das APIs (obrigatórios). A API do Crashlytics só aceita `firebase`
/// ou `cloud-platform` (não existe versão somente leitura); `firebase` é o
/// menor. Para compensar, `http.rs` só permite GET nos hosts do Firebase.
pub const SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/playdeveloperreporting",
    "https://www.googleapis.com/auth/firebase",
];
/// Identificam a conta (para separar os dados de cada uma). Não confidenciais.
const IDENTITY_SCOPES: &[&str] = &["openid", "email"];

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// Cliente OAuth do app ("App para computador"), embutido na compilação por
/// `build.rs`. Nenhum usuário precisa criar ou colar credenciais.
///
/// Para apps instalados, o Google e a RFC 8252 (§8.5) não tratam o
/// `client_secret` como confidencial: quem protege o fluxo é o PKCE + loopback.
/// Mesmo assim ele não fica no repositório; entra só no build (CI/arquivo local).
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

/// Conta Google conectada. `sub` é o ID estável da conta no Google.
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

    /// Identificador da pasta/chave locais desta conta (não revela o e-mail).
    pub fn local_id(&self) -> String {
        Sha256::digest(self.sub.as_bytes())[..12]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Lê o `id_token` recebido diretamente do endpoint de token do Google via
    /// TLS. Nesse caso a OIDC (Core §3.1.3.7) dispensa validar a assinatura;
    /// conferimos emissor, público e expiração.
    fn from_id_token(id_token: &str, client_id: &str) -> AppResult<Self> {
        let bad = || AppError::Auth("resposta de identidade inválida do Google".into());
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

    /// Login completo; devolve a conta que entrou.
    pub async fn sign_in(&self, app: &AppHandle, http: &GoogleHttp) -> AppResult<Account> {
        let client = OAuthClient::embedded()?;

        let verifier = Zeroizing::new(B64.encode(*secrets::random_bytes::<32>()?));
        let challenge = B64.encode(Sha256::digest(verifier.as_bytes()));
        let state = Zeroizing::new(B64.encode(*secrets::random_bytes::<16>()?));

        // Porta aleatória, só na interface de loopback.
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let redirect_uri = format!("http://127.0.0.1:{}", listener.local_addr()?.port());

        let mut url = Url::parse(AUTH_URL).expect("URL fixa válida");
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
            .map_err(|e| AppError::Auth(format!("não foi possível abrir o navegador: {e}")))?;

        let code = tokio::time::timeout(LOGIN_TIMEOUT, wait_for_code(&listener, &state))
            .await
            .map_err(|_| AppError::Auth("tempo esgotado esperando o navegador".into()))??;
        drop(listener);

        let tr: TokenResponse = http
            .post_form(
                Url::parse(TOKEN_URL).expect("URL fixa válida"),
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

        // Com consentimento granular, o usuário pode desmarcar permissões.
        let granted = tr.scope.as_deref().unwrap_or_default();
        let missing: Vec<&str> = SCOPES
            .iter()
            .copied()
            .filter(|s| !granted.split(' ').any(|g| g == *s))
            .collect();
        if !missing.is_empty() {
            return Err(AppError::Auth(format!(
                "permissões não concedidas: {}. Entre de novo e marque todas.",
                missing.join(", ")
            )));
        }
        let account = Account::from_id_token(
            tr.id_token.as_deref().ok_or_else(|| {
                AppError::Auth("o Google não devolveu a identidade da conta".into())
            })?,
            client.client_id,
        )?;
        let refresh = Zeroizing::new(
            tr.refresh_token
                .ok_or_else(|| AppError::Auth("o Google não devolveu um refresh token".into()))?,
        );
        secrets::set(secrets::REFRESH_TOKEN, &refresh)?;
        secrets::set(secrets::ACCOUNT, &serde_json::to_string(&account)?)?;
        *self.cached.lock().await = Some(CachedToken {
            token: Zeroizing::new(tr.access_token),
            expires_at: Instant::now() + Duration::from_secs(tr.expires_in),
        });
        Ok(account)
    }

    /// Access token válido por pelo menos mais 60 s, renovando se preciso.
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
                Url::parse(TOKEN_URL).expect("URL fixa válida"),
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
            // Token revogado ou expirado: exige novo login.
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

    /// Revoga o token no Google e apaga do chaveiro.
    pub async fn sign_out(&self, http: &GoogleHttp) -> AppResult<()> {
        if let Some(rt) = secrets::get(secrets::REFRESH_TOKEN)? {
            // A resposta do revoke não é JSON útil; o importante é apagar localmente.
            let _ = http
                .post_form::<serde_json::Value>(
                    Url::parse(REVOKE_URL).expect("URL fixa válida"),
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

/// Espera o redirecionamento do navegador. Ignora requisições sem parâmetros
/// (ex.: /favicon.ico) e só aceita a que traz o `state` correto.
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
            return Err(AppError::Auth(
                "parâmetro state inválido (possível ataque CSRF)".into(),
            ));
        }
        if let Some(err) = error {
            respond(&mut stream, 200, PAGE_ERROR).await;
            // Só repassa códigos de erro OAuth conhecidos (sem refletir texto arbitrário).
            let safe: String = err
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                .take(64)
                .collect();
            return Err(AppError::Auth(format!("o Google recusou o login ({safe})")));
        }
        respond(&mut stream, 200, PAGE_OK).await;
        return code.ok_or_else(|| AppError::Auth("código ausente".into()));
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

// Páginas estáticas: nada vindo da requisição é refletido no HTML.
const PAGE_OK: &str = "<!doctype html><meta charset=utf-8><title>Stackquake</title>\
<body style=\"font-family:system-ui;padding:3rem;text-align:center\">\
<h2>Login concluído</h2><p>Pode fechar esta aba e voltar ao Stackquake.</p></body>";
const PAGE_ERROR: &str = "<!doctype html><meta charset=utf-8><title>Stackquake</title>\
<body style=\"font-family:system-ui;padding:3rem;text-align:center\">\
<h2>Não foi possível concluir o login</h2><p>Volte ao Stackquake e tente de novo.</p></body>";

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
        assert!(Account::from_id_token("lixo", "cid").is_err());
    }
}
