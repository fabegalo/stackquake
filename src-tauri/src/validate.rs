//! Validation of everything that comes from the UI or an API and becomes part of a URL.

use std::sync::LazyLock;

use regex::Regex;

use crate::error::{AppError, AppResult};

static PACKAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$").unwrap());
// Google Cloud project IDs: 6–30 chars, lowercase letters, digits and hyphens.
static PROJECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]{4,28}[a-z0-9]$").unwrap());
// Firebase Android App ID: `1:<project number>:android:<hex>`.
static FIREBASE_APP_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^1:[0-9]{1,20}:android:[0-9a-f]{1,64}$").unwrap());
static ISSUE_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_-]{1,128}$").unwrap());

pub fn package(s: &str) -> AppResult<&str> {
    if s.len() <= 255 && PACKAGE.is_match(s) {
        Ok(s)
    } else {
        Err(AppError::invalid("invalid_package"))
    }
}

pub fn project(s: &str) -> AppResult<&str> {
    if PROJECT.is_match(s) {
        Ok(s)
    } else {
        Err(AppError::invalid("invalid_project"))
    }
}

pub fn firebase_app_id(s: &str) -> AppResult<&str> {
    if FIREBASE_APP_ID.is_match(s) {
        Ok(s)
    } else {
        Err(AppError::invalid("invalid_firebase_app_id"))
    }
}

pub fn issue_id(s: &str) -> AppResult<&str> {
    if ISSUE_ID.is_match(s) {
        Ok(s)
    } else {
        Err(AppError::invalid("invalid_issue_id"))
    }
}

pub fn status(s: &str) -> AppResult<&str> {
    match s {
        "open" | "investigating" | "resolved" | "ignored" | "regressed" => Ok(s),
        _ => Err(AppError::invalid("invalid_status")),
    }
}

pub fn text(s: Option<String>, max: usize, field: &'static str) -> AppResult<Option<String>> {
    match s {
        Some(v) if v.chars().count() > max => Err(AppError::invalid_with(
            "text_too_long",
            vec![("field", field.to_string()), ("max", max.to_string())],
        )),
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
