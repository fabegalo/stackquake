//! KPIs são arquivos .sql. Os embutidos ficam em `src-tauri/kpis/`; o usuário
//! pode criar os próprios (ou sobrescrever um embutido com o mesmo nome) na
//! pasta de KPIs do app, sem recompilar.
//!
//! Cabeçalho opcional:
//!     -- name: Nome exibido
//!     -- description: Texto de ajuda
//! Parâmetros disponíveis: `:package` (app selecionado) e `:days` (janela).
//!
//! Segurança: rodam numa conexão só de leitura, com autorizador que bloqueia
//! escrita/ATTACH/PRAGMA, limite de tempo (5 s) e de linhas (5.000).

use std::path::Path;
use std::time::{Duration, Instant};

use rusqlite::types::ValueRef;
use serde::Serialize;

use crate::db::Db;
use crate::error::{AppError, AppResult};

const BUILTIN: &[(&str, &str)] = &[
    ("01_resumo", include_str!("../kpis/01_resumo.sql")),
    (
        "02_saude_por_versao",
        include_str!("../kpis/02_saude_por_versao.sql"),
    ),
    (
        "03_tendencia_diaria",
        include_str!("../kpis/03_tendencia_diaria.sql"),
    ),
    (
        "04_top_problemas",
        include_str!("../kpis/04_top_problemas.sql"),
    ),
    (
        "05_novos_e_regressoes",
        include_str!("../kpis/05_novos_e_regressoes.sql"),
    ),
    (
        "06_cobertura_fontes",
        include_str!("../kpis/06_cobertura_fontes.sql"),
    ),
];

const MAX_USER_FILES: usize = 200;
const MAX_SQL_BYTES: u64 = 64 * 1024;
const MAX_ROWS: usize = 5000;
const TIMEOUT: Duration = Duration::from_secs(5);

const USER_README: &str = "KPIs personalizados do Stackquake\n\
================================\n\n\
Coloque aqui arquivos .sql (um KPI por arquivo). O nome do arquivo é o ID do KPI;\n\
um arquivo com o mesmo ID de um KPI embutido (ex.: 02_saude_por_versao.sql) o substitui.\n\n\
Cabeçalho opcional:\n  -- name: Meu KPI\n  -- description: O que ele mostra\n\n\
Parâmetros: :package (app selecionado) e :days (janela em dias).\n\
Tabelas: app, vitals_daily, play_issue, crashlytics_issue, crashlytics_issue_daily,\n\
crashlytics_version_daily, crashlytics_issue_version, problem.\n\
As consultas são somente leitura.\n";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KpiInfo {
    pub id: String,
    pub name: String,
    pub description: String,
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
    let header = |key: &str| {
        sql.lines()
            .map(str::trim)
            .take_while(|l| l.starts_with("--"))
            .find_map(|l| {
                l.trim_start_matches('-')
                    .trim()
                    .strip_prefix(key)
                    .map(|v| v.trim().to_string())
            })
    };
    KpiInfo {
        id: id.to_string(),
        name: header("name:").unwrap_or_else(|| id.to_string()),
        description: header("description:").unwrap_or_default(),
        source,
    }
}

pub fn ensure_user_dir(dir: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dir)?;
    let readme = dir.join("LEIA-ME.txt");
    if !readme.exists() {
        std::fs::write(readme, USER_README)?;
    }
    Ok(())
}

/// Todos os KPIs (embutidos + do usuário), com o SQL de cada um.
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
            // Ignora links simbólicos e arquivos grandes demais.
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
        .map_err(|e| AppError::Invalid(format!("KPI inválido ou bloqueado: {e}")))?;
    if !stmt.readonly() {
        return Err(AppError::Invalid("KPIs só podem ler dados".into()));
    }
    for i in 1..=stmt.parameter_count() {
        match stmt.parameter_name(i) {
            Some(":package") => stmt.raw_bind_parameter(i, package)?,
            Some(":days") => stmt.raw_bind_parameter(i, days)?,
            other => {
                return Err(AppError::Invalid(format!(
                    "parâmetro desconhecido no KPI: {} (use :package e :days)",
                    other.unwrap_or("?")
                )))
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
            AppError::Invalid("KPI excedeu o tempo limite de 5 s".into())
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
                ValueRef::Blob(_) => "<binário>".into(),
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
    fn parses_header() {
        let m = meta(
            "x",
            "-- name: Meu KPI\n-- description: Ajuda\nSELECT 1",
            "user",
        );
        assert_eq!(m.name, "Meu KPI");
        assert_eq!(m.description, "Ajuda");
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
