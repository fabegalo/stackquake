//! Único ponto de saída de rede do aplicativo.
//!
//! Toda requisição passa por `GoogleHttp`, que só aceita HTTPS para uma lista
//! fixa de hosts do Google, cada um com os métodos HTTP permitidos, e não segue
//! redirecionamentos. A interface não tem acesso à rede (CSP `connect-src` só
//! permite IPC).
//!
//! A API do Crashlytics não tem escopo somente leitura (exige `firebase`); por
//! isso os hosts do Firebase só aceitam GET aqui: o app não consegue alterar
//! nada no Firebase, mesmo que um bug tente.

use std::time::Duration;

use serde::{de::DeserializeOwned, Serialize};
use url::Url;

use crate::error::{AppError, AppResult};

/// Host -> métodos permitidos.
const ALLOWED: &[(&str, &[&str])] = &[
    ("oauth2.googleapis.com", &["POST"]),
    // `*:query` da Reporting API é POST, mas só lê dados.
    ("playdeveloperreporting.googleapis.com", &["GET", "POST"]),
    ("firebase.googleapis.com", &["GET"]),
    ("firebasecrashlytics.googleapis.com", &["GET"]),
];

/// Respostas maiores que isso são recusadas (proteção contra consumo de memória).
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
                "destino bloqueado pela lista de hosts permitidos: {}://{}",
                url.scheme(),
                url.host_str().unwrap_or("(sem host)")
            )));
        };
        if !methods.contains(&method) {
            return Err(AppError::Network(format!(
                "método {method} bloqueado para {}",
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

    /// POST de formulário sem token (endpoints OAuth).
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
            return Err(AppError::Network("resposta grande demais".into()));
        }
        let bytes = resp.bytes().await?;
        if bytes.len() > MAX_BODY_BYTES {
            return Err(AppError::Network("resposta grande demais".into()));
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

/// Extrai a mensagem de erro do formato padrão do Google (`{"error": {...}}`
/// ou `{"error": "...", "error_description": "..."}` do OAuth), truncada.
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
