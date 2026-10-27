//! The app's single network egress point.
//!
//! Every request goes through `GoogleHttp`, which only accepts HTTPS to a fixed
//! list of Google hosts, each with its allowed HTTP methods, and never follows
//! redirects. The UI has no network access (CSP `connect-src` only allows IPC).
//!
//! The Crashlytics API has no read-only scope (it requires `firebase`), so the
//! Firebase hosts only accept GET here: the app cannot change anything in
//! Firebase, even if a bug tries to.

use std::time::Duration;

use serde::{de::DeserializeOwned, Serialize};
use url::Url;

use crate::error::{AppError, AppResult};

/// Host -> allowed methods.
const ALLOWED: &[(&str, &[&str])] = &[
    ("oauth2.googleapis.com", &["POST"]),
    // The Reporting API's `*:query` is a POST, but it only reads data.
    ("playdeveloperreporting.googleapis.com", &["GET", "POST"]),
    ("firebase.googleapis.com", &["GET"]),
    ("firebasecrashlytics.googleapis.com", &["GET"]),
];

/// Larger responses are refused (protection against memory exhaustion).
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

pub struct GoogleHttp {
    client: reqwest::Client,
}

impl GoogleHttp {
    pub fn new() -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(90))
            .user_agent(concat!("Stackquake/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client })
    }

    fn check(method: &str, url: &Url) -> AppResult<()> {
        let methods = url
            .host_str()
            .and_then(|h| ALLOWED.iter().find(|(host, _)| *host == h))
            .map(|(_, m)| *m);
        let Some(methods) = methods.filter(|_| url.scheme() == "https" && url.port().is_none())
        else {
            return Err(AppError::Network(format!(
                "destination blocked by the host allowlist: {}://{}",
                url.scheme(),
                url.host_str().unwrap_or("(no host)")
            )));
        };
        if !methods.contains(&method) {
            return Err(AppError::Network(format!(
                "method {method} blocked for {}",
                url.host_str().unwrap_or("?")
            )));
        }
        Ok(())
    }

    pub async fn get_json<T: DeserializeOwned>(&self, url: Url, token: &str) -> AppResult<T> {
        Self::check("GET", &url)?;
        let resp = self.client.get(url).bearer_auth(token).send().await?;
        Self::read_json(resp).await
    }

    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        url: Url,
        token: &str,
        body: &B,
    ) -> AppResult<T> {
        Self::check("POST", &url)?;
        let resp = self
            .client
            .post(url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await?;
        Self::read_json(resp).await
    }

    /// Form POST without a token (OAuth endpoints).
    pub async fn post_form<T: DeserializeOwned>(
        &self,
        url: Url,
        form: &[(&str, &str)],
    ) -> AppResult<T> {
        Self::check("POST", &url)?;
        let resp = self.client.post(url).form(form).send().await?;
        Self::read_json(resp).await
    }

    async fn read_json<T: DeserializeOwned>(resp: reqwest::Response) -> AppResult<T> {
        let status = resp.status();
        if resp
            .content_length()
            .is_some_and(|len| len as usize > MAX_BODY_BYTES)
        {
            return Err(AppError::Network("response too large".into()));
        }
        let bytes = resp.bytes().await?;
        if bytes.len() > MAX_BODY_BYTES {
            return Err(AppError::Network("response too large".into()));
        }
        if !status.is_success() {
            return Err(AppError::Api {
                status: status.as_u16(),
                message: google_error_message(&bytes),
            });
        }
        Ok(serde_json::from_slice(&bytes)?)
    }
}

/// Extracts the error message from Google's standard format (`{"error": {...}}`
/// or OAuth's `{"error": "...", "error_description": "..."}`), truncated.
fn google_error_message(body: &[u8]) -> String {
    let v: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
    let msg = match &v["error"] {
        serde_json::Value::Object(o) => o
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("erro desconhecido")
            .to_string(),
        serde_json::Value::String(code) => match v["error_description"].as_str() {
            Some(d) => format!("{code}: {d}"),
            None => code.clone(),
        },
        _ => "erro desconhecido".to_string(),
    };
    msg.chars().take(400).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_hosts_outside_allowlist() {
        for bad in [
            "http://firebase.googleapis.com/x",
            "https://evil.example.com/x",
            "https://firebase.googleapis.com.evil.com/x",
            "https://firebase.googleapis.com:8443/x",
            "https://bigquery.googleapis.com/x",
        ] {
            assert!(
                GoogleHttp::check("GET", &Url::parse(bad).unwrap()).is_err(),
                "{bad}"
            );
        }
        assert!(GoogleHttp::check(
            "GET",
            &Url::parse("https://firebase.googleapis.com/x").unwrap()
        )
        .is_ok());
    }

    #[test]
    fn firebase_hosts_are_read_only() {
        for host in [
            "firebase.googleapis.com",
            "firebasecrashlytics.googleapis.com",
        ] {
            let u = Url::parse(&format!("https://{host}/v1/x")).unwrap();
            assert!(GoogleHttp::check("GET", &u).is_ok());
            for m in ["POST", "PATCH", "PUT", "DELETE"] {
                assert!(GoogleHttp::check(m, &u).is_err(), "{m} {host}");
            }
        }
    }

    #[test]
    fn parses_google_errors() {
        assert_eq!(
            google_error_message(br#"{"error":{"code":403,"message":"denied"}}"#),
            "denied"
        );
        assert_eq!(
            google_error_message(br#"{"error":"invalid_grant","error_description":"expired"}"#),
            "invalid_grant: expired"
        );
    }
}
