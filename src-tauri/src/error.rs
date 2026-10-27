use serde::{Serialize, Serializer};

/// Erros que podem chegar à interface. As mensagens são pensadas para o
/// usuário e nunca incluem tokens, segredos ou URLs com parâmetros.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Você não está conectado. Entre com sua conta Google.")]
    NotSignedIn,
    #[error("Este build do Stackquake não tem o cliente OAuth do Google embutido. Quem compila precisa seguir docs/DESENVOLVIMENTO.md.")]
    NoOAuthClient,
    #[error("Entrada inválida: {0}")]
    Invalid(String),
    #[error("Falha no login: {0}")]
    Auth(String),
    #[error("Erro da API Google ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("Erro de rede: {0}")]
    Network(String),
    #[error("Erro no banco local: {0}")]
    Db(String),
    #[error("Erro no chaveiro do sistema: {0}")]
    Keyring(String),
    #[error("{0}")]
    Other(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        // without_url: a URL pode conter parâmetros que não devem ir para logs/UI.
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
        AppError::Other(format!("E/S: {e}"))
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
