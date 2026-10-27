mod auth;
mod commands;
mod db;
mod error;
mod google;
mod http;
mod kpi;
mod secrets;
mod sync;
mod unify;
mod validate;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tauri::Manager;

use crate::error::{AppError, AppResult};

pub struct AppState {
    pub auth: auth::Auth,
    pub http: http::GoogleHttp,
    pub data_dir: PathBuf,
    pub kpi_dir: PathBuf,
    /// Banco da conta conectada (`None` quando ninguém está conectado).
    db: RwLock<Option<Arc<db::Db>>>,
    pub sync_lock: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn db(&self) -> AppResult<Arc<db::Db>> {
        self.db
            .read()
            .map_err(|_| AppError::Db("estado indisponível".into()))?
            .clone()
            .ok_or(AppError::NotSignedIn)
    }

    /// Abre o banco desta conta, em `accounts/<id>/`, com chave própria.
    pub fn open_account(&self, account: &auth::Account) -> AppResult<()> {
        let local_id = account.local_id();
        let dir = self.data_dir.join("accounts").join(&local_id);
        let db = db::Db::open(&dir, &secrets::db_key_name(&local_id))?;
        *self
            .db
            .write()
            .map_err(|_| AppError::Db("estado indisponível".into()))? = Some(Arc::new(db));
        Ok(())
    }

    pub fn close_account(&self) {
        if let Ok(mut guard) = self.db.write() {
            *guard = None;
        }
    }
}

/// Remove o banco do formato antigo (antes de existir um banco por conta).
fn remove_legacy_data(data_dir: &std::path::Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(data_dir.join(format!("stackquake.db{suffix}")));
    }
    let _ = secrets::delete("local-db-key");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Usado só pelo núcleo (abrir o navegador do login e a pasta de KPIs);
        // nenhuma permissão do opener é dada à interface.
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            db::private_dir(&data_dir)?;
            remove_legacy_data(&data_dir);
            let kpi_dir = data_dir.join("kpis");
            kpi::ensure_user_dir(&kpi_dir)?;
            let state = AppState {
                auth: auth::Auth::default(),
                http: http::GoogleHttp::new()?,
                data_dir,
                kpi_dir,
                db: RwLock::new(None),
                sync_lock: tokio::sync::Mutex::new(()),
            };
            // Reabre o banco da última conta conectada, se houver.
            if state.auth.is_signed_in().unwrap_or(false) {
                if let Ok(Some(account)) = auth::Account::current() {
                    let _ = state.open_account(&account);
                }
            }
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_setup_status,
            commands::sign_in,
            commands::sign_out,
            commands::list_apps,
            commands::refresh_apps,
            commands::update_app_settings,
            commands::sync_app,
            commands::list_kpis,
            commands::run_kpi,
            commands::list_problems,
            commands::get_problem,
            commands::update_problem,
            commands::fetch_problem_sample,
            commands::open_kpi_folder,
            commands::reset_local_data,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Stackquake");
}
