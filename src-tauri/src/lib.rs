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
    /// Database of the signed-in account (`None` when nobody is signed in).
    db: RwLock<Option<Arc<db::Db>>>,
    pub sync_lock: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn db(&self) -> AppResult<Arc<db::Db>> {
        self.db
            .read()
            .map_err(|_| AppError::Db("state unavailable".into()))?
            .clone()
            .ok_or(AppError::NotSignedIn)
    }

    /// Opens this account's database in `accounts/<id>/`, with its own key.
    pub fn open_account(&self, account: &auth::Account) -> AppResult<()> {
        let local_id = account.local_id();
        let dir = self.data_dir.join("accounts").join(&local_id);
        let db = db::Db::open(&dir, &secrets::db_key_name(&local_id))?;
        *self
            .db
            .write()
            .map_err(|_| AppError::Db("state unavailable".into()))? = Some(Arc::new(db));
        Ok(())
    }

    pub fn close_account(&self) {
        if let Ok(mut guard) = self.db.write() {
            *guard = None;
        }
    }
}

/// Removes the legacy database (from before one database per account).
fn remove_legacy_data(data_dir: &std::path::Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(data_dir.join(format!("stackquake.db{suffix}")));
    }
    let _ = secrets::delete("local-db-key");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Used only by the core (opening the sign-in browser and the KPI folder);
        // no opener permission is granted to the UI.
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
            // Reopens the database of the last signed-in account, if any.
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
        .expect("error while starting Stackquake");
}
