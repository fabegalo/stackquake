//! Comandos IPC: a única interface entre a UI e o núcleo. Toda entrada é
//! validada aqui; nenhum comando devolve tokens ou segredos.

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::auth::{Account, OAuthClient};
use crate::db::{AppRow, ProblemDetail, ProblemFilter, ProblemRow};
use crate::error::{AppError, AppResult};
use crate::google::play;
use crate::kpi::{self, KpiInfo, KpiResult};
use crate::sync::{self, Ctx, SyncReport};
use crate::{validate, AppState};

fn days(d: i64) -> AppResult<i64> {
    if (1..=365).contains(&d) {
        Ok(d)
    } else {
        Err(AppError::Invalid(
            "janela deve ter entre 1 e 365 dias".into(),
        ))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    has_client: bool,
    signed_in: bool,
    email: Option<String>,
}

#[tauri::command]
pub async fn get_setup_status(state: State<'_, AppState>) -> AppResult<SetupStatus> {
    let signed_in = state.auth.is_signed_in()? && state.db().is_ok();
    Ok(SetupStatus {
        has_client: OAuthClient::embedded().is_ok(),
        signed_in,
        email: signed_in
            .then(Account::current)
            .transpose()?
            .flatten()
            .map(|a| a.email),
    })
}

#[tauri::command]
pub async fn sign_in(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let account = state.auth.sign_in(&app, &state.http).await?;
    // Cada conta abre o próprio banco: dados de contas diferentes nunca se misturam.
    state.open_account(&account)
}

#[tauri::command]
pub async fn sign_out(state: State<'_, AppState>) -> AppResult<()> {
    state.close_account();
    state.auth.sign_out(&state.http).await
}

#[tauri::command]
pub async fn list_apps(state: State<'_, AppState>) -> AppResult<Vec<AppRow>> {
    state.db()?.list_apps()
}

#[tauri::command]
pub async fn refresh_apps(state: State<'_, AppState>) -> AppResult<Vec<AppRow>> {
    let db = state.db()?;
    let token = state.auth.access_token(&state.http).await?;
    let apps = play::search_apps(&state.http, &token).await?;
    let rows: Vec<(String, Option<String>)> = apps
        .into_iter()
        .map(|a| {
            (
                a.package_name,
                a.display_name.map(|n| n.chars().take(200).collect()),
            )
        })
        .collect();
    // Remove da lista local apps que não estão mais nesta conta.
    db.replace_apps(&rows)?;
    // Encontra sozinho o projeto Firebase de cada app (sem digitar ID).
    let ctx = Ctx {
        http: &state.http,
        db: &db,
        token: &token,
    };
    let _ = sync::discover_firebase_apps(&ctx).await;
    db.list_apps()
}

#[tauri::command]
pub async fn update_app_settings(
    state: State<'_, AppState>,
    package_name: String,
    enabled: bool,
) -> AppResult<()> {
    state
        .db()?
        .set_app_enabled(validate::package(&package_name)?, enabled)
}

#[tauri::command]
pub async fn sync_app(state: State<'_, AppState>, package_name: String) -> AppResult<SyncReport> {
    let _guard = state
        .sync_lock
        .try_lock()
        .map_err(|_| AppError::Invalid("já existe uma sincronização em andamento".into()))?;
    let db = state.db()?;
    let token = state.auth.access_token(&state.http).await?;
    let ctx = Ctx {
        http: &state.http,
        db: &db,
        token: &token,
    };
    sync::sync_app(&ctx, &package_name).await
}

#[tauri::command]
pub async fn list_kpis(state: State<'_, AppState>) -> AppResult<Vec<KpiInfo>> {
    Ok(kpi::all(&state.kpi_dir)?
        .into_iter()
        .map(|(info, _)| info)
        .collect())
}

#[tauri::command]
pub async fn run_kpi(
    state: State<'_, AppState>,
    id: String,
    package_name: String,
    days: i64,
) -> AppResult<KpiResult> {
    let pkg = validate::package(&package_name)?.to_string();
    let days = self::days(days)?;
    let sql = kpi::all(&state.kpi_dir)?
        .into_iter()
        .find(|(info, _)| info.id == id)
        .map(|(_, sql)| sql)
        .ok_or_else(|| AppError::Invalid("KPI não encontrado".into()))?;
    let db = state.db()?;
    tauri::async_runtime::spawn_blocking(move || kpi::run(&db, &sql, &pkg, days))
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
}

#[tauri::command]
pub async fn list_problems(
    state: State<'_, AppState>,
    package_name: String,
    days: i64,
    status: Option<String>,
    kind: Option<String>,
) -> AppResult<Vec<ProblemRow>> {
    let pkg = validate::package(&package_name)?;
    if let Some(s) = status.as_deref() {
        validate::status(s)?;
    }
    if let Some(k) = kind.as_deref() {
        if !matches!(k, "crash" | "anr" | "nonfatal") {
            return Err(AppError::Invalid("tipo inválido".into()));
        }
    }
    state.db()?.list_problems(
        pkg,
        &ProblemFilter {
            days: self::days(days)?,
            status,
            kind,
        },
    )
}

#[tauri::command]
pub async fn get_problem(state: State<'_, AppState>, id: i64) -> AppResult<ProblemDetail> {
    state.db()?.get_problem(id)
}

#[tauri::command]
pub async fn update_problem(
    state: State<'_, AppState>,
    id: i64,
    status: String,
    assignee: Option<String>,
    notes: Option<String>,
    resolved_in_version: Option<String>,
) -> AppResult<()> {
    let status = validate::status(&status)?;
    let assignee = validate::text(assignee, 200, "responsável")?;
    let notes = validate::text(notes, 20_000, "notas")?;
    let resolved = validate::text(resolved_in_version, 20, "versão")?;
    if resolved
        .as_deref()
        .is_some_and(|v| !v.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(AppError::Invalid(
            "versão da correção deve ser o versionCode (só números)".into(),
        ));
    }
    state
        .db()?
        .update_problem(id, status, assignee, notes, resolved)
}

#[tauri::command]
pub async fn fetch_problem_sample(state: State<'_, AppState>, id: i64) -> AppResult<ProblemDetail> {
    let db = state.db()?;
    let token = state.auth.access_token(&state.http).await?;
    let ctx = Ctx {
        http: &state.http,
        db: &db,
        token: &token,
    };
    sync::fetch_problem_details(&ctx, id).await
}

#[tauri::command]
pub async fn open_kpi_folder(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    kpi::ensure_user_dir(&state.kpi_dir)?;
    app.opener()
        .open_path(state.kpi_dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::Other(format!("não foi possível abrir a pasta: {e}")))
}

/// Apaga todos os dados locais desta conta (banco + chave) e sai da conta.
#[tauri::command]
pub async fn reset_local_data(state: State<'_, AppState>) -> AppResult<()> {
    let _guard = state
        .sync_lock
        .try_lock()
        .map_err(|_| AppError::Invalid("aguarde a sincronização terminar".into()))?;
    state.db()?.wipe()?;
    state.close_account();
    state.auth.sign_out(&state.http).await
}
