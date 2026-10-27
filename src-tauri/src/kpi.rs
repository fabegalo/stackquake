//! KPIs are .sql files. Built-in ones live in `src-tauri/kpis/`; users can add
//! their own (or override a built-in one with the same name) in the app's KPI
//! folder, without recompiling.
//!
//! Optional header (translations via `name.<locale>` / `description.<locale>`):
//!     -- name: Display name
//!     -- name.pt-BR: Nome exibido
//!     -- description: Help text
//! Available parameters: `:package` (selected app) and `:days` (window).
//!
//! Security: they run on a read-only connection, with an authorizer that blocks
//! writes/ATTACH/PRAGMA, a time limit (5 s) and a row limit (5,000).

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use rusqlite::types::ValueRef;
use serde::Serialize;

use crate::db::Db;
use crate::error::{AppError, AppResult};

const BUILTIN: &[(&str, &str)] = &[
    ("01_summary", include_str!("../kpis/01_summary.sql")),
    (
        "02_version_health",
        include_str!("../kpis/02_version_health.sql"),
    ),
    ("03_daily_trend", include_str!("../kpis/03_daily_trend.sql")),
    (
        "04_top_problems",
        include_str!("../kpis/04_top_problems.sql"),
    ),
    (
        "05_new_and_regressed",
        include_str!("../kpis/05_new_and_regressed.sql"),
    ),
    (
        "06_source_coverage",
        include_str!("../kpis/06_source_coverage.sql"),
    ),
];

const MAX_USER_FILES: usize = 200;
const MAX_SQL_BYTES: u64 = 64 * 1024;
const MAX_ROWS: usize = 5000;
const TIMEOUT: Duration = Duration::from_secs(5);

const USER_README: &str = "Stackquake custom KPIs\n\
======================\n\n\
Put .sql files here (one KPI per file). The file name is the KPI ID; a file with\n\
the same ID as a built-in KPI (e.g. 02_version_health.sql) replaces it.\n\n\
Optional header:\n  -- name: My KPI\n  -- name.pt-BR: Meu KPI\n  -- description: What it shows\n\n\
Parameters: :package (selected app) and :days (window in days).\n\
Tables: app, vitals_daily, play_issue, crashlytics_issue, crashlytics_issue_daily,\n\
crashlytics_version_daily, crashlytics_issue_version, problem.\n\
Queries are read-only.\n";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KpiInfo {
    pub id: String,
    /// Default (English) name and description.
    pub name: String,
    pub description: String,
    /// Translations by locale, from `-- name.<locale>:` / `-- description.<locale>:`.
    pub names: BTreeMap<String, String>,
    pub descriptions: BTreeMap<String, String>,
    pub source: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KpiResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub truncated: bool,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn meta(id: &str, sql: &str, source: &'static str) -> KpiInfo {
    let mut info = KpiInfo {
        id: id.to_string(),
        name: id.to_string(),
        description: String::new(),
        names: BTreeMap::new(),
        descriptions: BTreeMap::new(),
        source,
    };
    let header = sql
        .lines()
        .map(str::trim)
        .take_while(|l| l.starts_with("--"));
    for line in header {
        let Some((key, value)) = line.trim_start_matches('-').trim().split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        let (field, locale) = match key.split_once('.') {
            Some((f, l)) if valid_locale(l) => (f, Some(l.to_string())),
            Some(_) => continue,
            None => (key, None),
        };
        match (field, locale) {
            ("name", None) => info.name = value,
            ("description", None) => info.description = value,
            ("name", Some(l)) => {
                info.names.insert(l, value);
            }
            ("description", Some(l)) => {
                info.descriptions.insert(l, value);
            }
            _ => {}
        }
    }
    info
}

/// BCP 47-ish tag such as `pt-BR` or `es`.
fn valid_locale(l: &str) -> bool {
    !l.is_empty() && l.len() <= 10 && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

pub fn ensure_user_dir(dir: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dir)?;
    let readme = dir.join("README.txt");
    if !readme.exists() {
        std::fs::write(readme, USER_README)?;
    }
    Ok(())
}

/// All KPIs (built-in + user), with each one's SQL.
pub fn all(user_dir: &Path) -> AppResult<Vec<(KpiInfo, String)>> {
    let mut out: Vec<(KpiInfo, String)> = BUILTIN
        .iter()
        .map(|(id, sql)| (meta(id, sql, "builtin"), sql.to_string()))
        .collect();
    if let Ok(entries) = std::fs::read_dir(user_dir) {
        for entry in entries.flatten().take(MAX_USER_FILES) {
            let path = entry.path();
            let Some(id) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|_| path.extension().is_some_and(|e| e == "sql"))
                .filter(|id| valid_id(id))
            else {
                continue;
            };
            // Skips symlinks and files that are too large.
            let Ok(md) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if !md.is_file() || md.len() > MAX_SQL_BYTES {
                continue;
            }
            let Ok(sql) = std::fs::read_to_string(&path) else {
                continue;
            };
            let info = meta(id, &sql, "user");
            out.retain(|(k, _)| k.id != info.id);
            out.push((info, sql));
        }
    }
    out.sort_by(|a, b| a.0.id.cmp(&b.0.id));
    Ok(out)
}

pub fn run(db: &Db, sql: &str, package: &str, days: i64) -> AppResult<KpiResult> {
    let mut conn = db.open_kpi_connection()?;
    let deadline = Instant::now() + TIMEOUT;
    conn.progress_handler(10_000, Some(move || Instant::now() > deadline))?;

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| AppError::invalid_with("kpi_invalid", vec![("detail", e.to_string())]))?;
    if !stmt.readonly() {
        return Err(AppError::invalid("kpi_read_only"));
    }
    for i in 1..=stmt.parameter_count() {
        match stmt.parameter_name(i) {
            Some(":package") => stmt.raw_bind_parameter(i, package)?,
            Some(":days") => stmt.raw_bind_parameter(i, days)?,
            other => {
                return Err(AppError::invalid_with(
                    "kpi_unknown_param",
                    vec![("name", other.unwrap_or("?").to_string())],
                ))
            }
        }
    }
    let columns: Vec<String> = stmt.column_names().map(String::from).collect();
    let n = columns.len();
    let mut rows = Vec::new();
    let mut truncated = false;
    let mut cursor = stmt.raw_query();
    while let Some(row) = cursor.next().map_err(|e| match e {
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::OperationInterrupted =>
        {
            AppError::invalid("kpi_timeout")
        }
        other => AppError::from(other),
    })? {
        if rows.len() >= MAX_ROWS {
            truncated = true;
            break;
        }
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push(match row.get_ref(i)? {
                ValueRef::Null => serde_json::Value::Null,
                ValueRef::Integer(v) => v.into(),
                ValueRef::Real(v) => serde_json::Number::from_f64(v)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),
                ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned().into(),
                ValueRef::Blob(_) => "<binary>".into(),
            });
        }
        rows.push(out);
    }
    Ok(KpiResult {
        columns,
        rows,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_with_translations() {
        let m = meta(
            "x",
            "-- name: My KPI\n-- name.pt-BR: Meu KPI\n-- description: Help\n-- name.bad/locale: x\nSELECT 1",
            "user",
        );
        assert_eq!(m.name, "My KPI");
        assert_eq!(m.description, "Help");
        assert_eq!(m.names.get("pt-BR").map(String::as_str), Some("Meu KPI"));
        assert_eq!(m.names.len(), 1);
    }

    #[test]
    fn rejects_bad_ids() {
        assert!(valid_id("02_saude"));
        assert!(!valid_id("../etc"));
        assert!(!valid_id(""));
    }

    #[test]
    fn builtin_kpis_prepare_against_schema() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../sql/schema.sql"))
            .unwrap();
        for (id, sql) in BUILTIN {
            conn.prepare(sql).unwrap_or_else(|e| panic!("{id}: {e}"));
        }
    }
}
