//! Validação de tudo que vem da interface ou de uma API e vira parte de uma URL.

use std::sync::LazyLock;

use regex::Regex;

use crate::error::{AppError, AppResult};

static PACKAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$").unwrap());
// IDs de projeto do Google Cloud: 6–30 caracteres, minúsculas, dígitos e hífen.
static PROJECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]{4,28}[a-z0-9]$").unwrap());
// App ID do Firebase para Android: `1:<número do projeto>:android:<hex>`.
static FIREBASE_APP_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^1:[0-9]{1,20}:android:[0-9a-f]{1,64}$").unwrap());
static ISSUE_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_-]{1,128}$").unwrap());

pub fn package(s: &str) -> AppResult<&str> {
    if s.len() <= 255 && PACKAGE.is_match(s) {
        Ok(s)
    } else {
        Err(AppError::Invalid("nome de pacote inválido".into()))
    }
}

pub fn project(s: &str) -> AppResult<&str> {
    if PROJECT.is_match(s) {
        Ok(s)
    } else {
        Err(AppError::Invalid(
            "ID de projeto inválido (use o ID, não o nome; ex.: meu-app-1a2b3)".into(),
        ))
    }
}

pub fn firebase_app_id(s: &str) -> AppResult<&str> {
    if FIREBASE_APP_ID.is_match(s) {
        Ok(s)
    } else {
        Err(AppError::Invalid("App ID do Firebase inválido".into()))
    }
}

pub fn issue_id(s: &str) -> AppResult<&str> {
    if ISSUE_ID.is_match(s) {
        Ok(s)
    } else {
        Err(AppError::Invalid("ID de issue inválido".into()))
    }
}

pub fn status(s: &str) -> AppResult<&str> {
    match s {
        "open" | "investigating" | "resolved" | "ignored" | "regressed" => Ok(s),
        _ => Err(AppError::Invalid("status inválido".into())),
    }
}

pub fn text(s: Option<String>, max: usize, field: &str) -> AppResult<Option<String>> {
    match s {
        Some(v) if v.chars().count() > max => Err(AppError::Invalid(format!(
            "{field}: no máximo {max} caracteres"
        ))),
        Some(v) if v.trim().is_empty() => Ok(None),
        other => Ok(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_names() {
        assert!(package("com.example.app").is_ok());
        assert!(package("com.example.app_2").is_ok());
        assert!(package("app").is_err());
        assert!(package("com.example`; DROP").is_err());
        assert!(package("com..x").is_err());
    }

    #[test]
    fn project_ids() {
        assert!(project("meu-app-1a2b3").is_ok());
        assert!(project("Meu-App").is_err());
        assert!(project("x`.y").is_err());
    }

    #[test]
    fn firebase_app_ids() {
        assert!(firebase_app_id("1:123456:android:ab12cd").is_ok());
        assert!(firebase_app_id("1:123:ios:ab12").is_err());
        assert!(firebase_app_id("1:123:android:ab/../x").is_err());
    }
}
