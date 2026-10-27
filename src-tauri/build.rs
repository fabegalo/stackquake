// Declara os comandos do app para que o Tauri gere uma permissão por comando.
// Comandos não listados em capabilities/default.json ficam bloqueados para a interface.
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

/// Embute o cliente OAuth do app no binário, para o usuário final só clicar em
/// "Entrar com Google". Fontes, em ordem:
/// 1. variáveis STACKQUAKE_GOOGLE_CLIENT_ID / STACKQUAKE_GOOGLE_CLIENT_SECRET (CI);
/// 2. o arquivo src-tauri/google-oauth-client.json baixado do Google Cloud
///    (fica fora do git, ver .gitignore).
fn embed_oauth_client() {
    println!("cargo:rerun-if-env-changed=STACKQUAKE_GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=STACKQUAKE_GOOGLE_CLIENT_SECRET");
    println!("cargo:rerun-if-changed={CLIENT_FILE}");

    if std::env::var("STACKQUAKE_GOOGLE_CLIENT_ID").is_ok() {
        return; // option_env! lê direto do ambiente
    }
    let Ok(raw) = std::fs::read_to_string(CLIENT_FILE) else {
        println!(
            "cargo:warning=Sem cliente OAuth do Google: o login ficará desativado neste build. Veja docs/DESENVOLVIMENTO.md"
        );
        return;
    };
    let v: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| panic!("{CLIENT_FILE} não é um JSON válido"));
    let c = v.get("installed").unwrap_or_else(|| {
        panic!("{CLIENT_FILE}: o cliente precisa ser do tipo 'App para computador' (chave \"installed\")")
    });
    let field = |k: &str| -> String {
        let s = c[k].as_str().unwrap_or_default();
        // Só caracteres seguros: o valor vira uma variável de compilação.
        assert!(
            !s.is_empty() && s.len() < 200 && s.chars().all(|ch| ch.is_ascii_graphic()),
            "{CLIENT_FILE}: campo {k} ausente ou inválido"
        );
        s.to_string()
    };
    let id = field("client_id");
    assert!(
        id.ends_with(".apps.googleusercontent.com"),
        "{CLIENT_FILE}: client_id inválido"
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
    .expect("falha ao gerar o contexto do Tauri");
}
