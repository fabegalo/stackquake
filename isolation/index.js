// Padrão Isolation do Tauri: toda chamada IPC da interface passa por aqui,
// num iframe isolado, antes de chegar ao núcleo Rust. Mantenha esta lista
// igual a COMMANDS em src-tauri/build.rs.
const ALLOWED = new Set([
  "get_setup_status",
  "sign_in",
  "sign_out",
  "list_apps",
  "refresh_apps",
  "update_app_settings",
  "sync_app",
  "list_kpis",
  "run_kpi",
  "list_problems",
  "get_problem",
  "update_problem",
  "fetch_problem_sample",
  "open_kpi_folder",
  "reset_local_data",
]);

window.__TAURI_ISOLATION_HOOK__ = (payload) => {
  if (!payload || typeof payload.cmd !== "string" || !ALLOWED.has(payload.cmd)) {
    throw new Error("Stackquake: comando IPC bloqueado");
  }
  return payload;
};
