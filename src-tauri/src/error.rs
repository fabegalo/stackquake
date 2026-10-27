//! Errors returned to the UI.
//!
//! The core never sends user-facing prose. Every error has a stable `code`
//! (plus optional `params`) that the UI translates; `message` is an English
//! fallback for logs. Messages never contain tokens, secrets or URLs with
//! query parameters.

use std::collections::BTreeMap;

use serde::{Serialize, Serializer};

pub type Params = Vec<(&'static str, String)>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not signed in")]
    NotSignedIn,
    #[error("this build has no embedded Google OAuth client")]
    NoOAuthClient,
    /// Invalid input or missing entity; `code` is translated by the UI.
    #[error("invalid input: {code}")]
    Invalid { code: &'static str, params: Params },
    /// Sign-in failure; `code` is translated by the UI.
    #[error("sign-in failed: {code}")]
    Auth { code: &'static str, params: Params },
    /// A known, translatable condition (e.g. an API that is not enabled).
    #[error("{code}")]
    Known { code: &'static str, params: Params },
    #[error("Google API error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("network error: {0}")]
    Network(String),
    #[error("local database error: {0}")]
    Db(String),
    #[error("system keychain error: {0}")]
    Keyring(String),
    #[error("internal error: {0}")]
    Other(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn invalid(code: &'static str) -> Self {
        Self::Invalid {
            code,
            params: vec![],
        }
    }

    pub fn invalid_with(code: &'static str, params: Params) -> Self {
        Self::Invalid { code, params }
    }

    pub fn auth(code: &'static str) -> Self {
        Self::Auth {
            code,
            params: vec![],
        }
    }

    pub fn auth_with(code: &'static str, params: Params) -> Self {
        Self::Auth { code, params }
    }

    pub fn known(code: &'static str) -> Self {
        Self::Known {
            code,
            params: vec![],
        }
    }

    pub fn known_with(code: &'static str, params: Params) -> Self {
        Self::Known { code, params }
    }

    /// Stable code + params, as sent to the UI.
    pub fn payload(&self) -> ErrorPayload {
        let (code, params): (&str, Params) = match self {
            Self::NotSignedIn => ("not_signed_in", vec![]),
            Self::NoOAuthClient => ("no_oauth_client", vec![]),
            Self::Invalid { code, params }
            | Self::Auth { code, params }
            | Self::Known { code, params } => (code, params.clone()),
            Self::Api { status, message } => (
                "google_api",
                vec![("status", status.to_string()), ("detail", message.clone())],
            ),
            Self::Network(d) => ("network", vec![("detail", d.clone())]),
            Self::Db(d) => ("database", vec![("detail", d.clone())]),
            Self::Keyring(d) => ("keychain", vec![("detail", d.clone())]),
            Self::Other(d) => ("internal", vec![("detail", d.clone())]),
        };
        ErrorPayload {
            code: code.to_string(),
            params: params.into_iter().collect(),
            message: self.to_string(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ErrorPayload {
    pub code: String,
    pub params: BTreeMap<&'static str, String>,
    pub message: String,
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.payload().serialize(s)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        // without_url: the URL may carry parameters that must not reach logs/UI.
        AppError::Network(e.without_url().to_string())
    }
}

impl From<keyring::Error> for AppError {
    fn from(e: keyring::Error) -> Self {
        AppError::Keyring(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Other(format!("I/O: {e}"))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Other(format!("JSON: {e}"))
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Other(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_code_and_params() {
        let v = serde_json::to_value(AppError::invalid_with(
            "text_too_long",
            vec![("field", "notes".into()), ("max", "10".into())],
        ))
        .unwrap();
        assert_eq!(v["code"], "text_too_long");
        assert_eq!(v["params"]["field"], "notes");
        let api = serde_json::to_value(AppError::Api {
            status: 403,
            message: "denied".into(),
        })
        .unwrap();
        assert_eq!(api["code"], "google_api");
        assert_eq!(api["params"]["status"], "403");
    }
}
