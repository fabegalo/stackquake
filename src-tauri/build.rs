// Declares the app commands so Tauri generates one permission per command.
// Commands not listed in capabilities/default.json are blocked for the UI.
const COMMANDS: &[&str] = &[
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
];

const CLIENT_FILE: &str = "google-oauth-client.json";

/// Embeds the app's OAuth client in the binary, so end users only click
/// "Sign in with Google". Sources, in order:
/// 1. STACKQUAKE_GOOGLE_CLIENT_ID / STACKQUAKE_GOOGLE_CLIENT_SECRET env vars (CI);
/// 2. src-tauri/google-oauth-client.json downloaded from Google Cloud
///    (kept out of git, see .gitignore).
fn embed_oauth_client() {
    println!("cargo:rerun-if-env-changed=STACKQUAKE_GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=STACKQUAKE_GOOGLE_CLIENT_SECRET");
    println!("cargo:rerun-if-changed={CLIENT_FILE}");

    if std::env::var("STACKQUAKE_GOOGLE_CLIENT_ID").is_ok() {
        return; // option_env! reads straight from the environment
    }
    let Ok(raw) = std::fs::read_to_string(CLIENT_FILE) else {
        println!(
            "cargo:warning=No Google OAuth client: sign-in is disabled in this build. See docs/DEVELOPMENT.md"
        );
        return;
    };
    let v: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| panic!("{CLIENT_FILE} is not valid JSON"));
    let c = v.get("installed").unwrap_or_else(|| {
        panic!("{CLIENT_FILE}: the client must be of type 'Desktop app' (\"installed\" key)")
    });
    let field = |k: &str| -> String {
        let s = c[k].as_str().unwrap_or_default();
        // Safe characters only: the value becomes a compile-time variable.
        assert!(
            !s.is_empty() && s.len() < 200 && s.chars().all(|ch| ch.is_ascii_graphic()),
            "{CLIENT_FILE}: field {k} missing or invalid"
        );
        s.to_string()
    };
    let id = field("client_id");
    assert!(
        id.ends_with(".apps.googleusercontent.com"),
        "{CLIENT_FILE}: invalid client_id"
    );
    println!("cargo:rustc-env=STACKQUAKE_GOOGLE_CLIENT_ID={id}");
    println!(
        "cargo:rustc-env=STACKQUAKE_GOOGLE_CLIENT_SECRET={}",
        field("client_secret")
    );
}

fn main() {
    embed_oauth_client();
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to generate the Tauri context");
}
