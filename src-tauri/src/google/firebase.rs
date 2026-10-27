//! Firebase Management API: descobre em qual projeto Firebase está cada app
//! Android da conta (e o App ID que a API do Crashlytics exige), para o usuário
//! não precisar digitar nada.

use std::collections::HashMap;

use serde::Deserialize;
use url::Url;

use crate::error::AppResult;
use crate::http::GoogleHttp;
use crate::validate;

const BASE: &str = "https://firebase.googleapis.com/v1beta1/";
const MAX_PROJECTS: usize = 200;

/// Concatena em vez de usar `Url::join`: com `join`, um caminho como
/// `apps:search` seria lido como uma URL absoluta de esquema `apps:`.
fn url(path: &str) -> Url {
    Url::parse(&format!("{BASE}{path}")).expect("URL da API válida")
}

#[derive(Debug, Clone, PartialEq)]
pub struct FirebaseApp {
    pub project_id: String,
    /// Ex.: `1:123456789:android:abc123`.
    pub app_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Project {
    project_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectsPage {
    #[serde(default)]
    results: Vec<Project>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AndroidApp {
    package_name: Option<String>,
    app_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AndroidAppsPage {
    #[serde(default)]
    apps: Vec<AndroidApp>,
    next_page_token: Option<String>,
}

async fn list_projects(http: &GoogleHttp, token: &str) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    while out.len() < MAX_PROJECTS {
        let mut u = url("projects");
        u.query_pairs_mut().append_pair("pageSize", "100");
        if let Some(p) = &page {
            u.query_pairs_mut().append_pair("pageToken", p);
        }
        let resp: ProjectsPage = http.get_json(u, token).await?;
        out.extend(
            resp.results
                .into_iter()
                .map(|p| p.project_id)
                .filter(|id| validate::project(id).is_ok()),
        );
        match resp.next_page_token.filter(|t| !t.is_empty()) {
            Some(t) => page = Some(t),
            None => break,
        }
    }
    out.truncate(MAX_PROJECTS);
    Ok(out)
}

/// `(pacote, app_id)` dos apps Android de um projeto.
async fn android_apps(
    http: &GoogleHttp,
    token: &str,
    project: &str,
) -> AppResult<Vec<(String, String)>> {
    let project = validate::project(project)?;
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    for _ in 0..10 {
        let mut u = url(&format!("projects/{project}/androidApps"));
        u.query_pairs_mut().append_pair("pageSize", "100");
        if let Some(p) = &page {
            u.query_pairs_mut().append_pair("pageToken", p);
        }
        let resp: AndroidAppsPage = http.get_json(u, token).await?;
        out.extend(resp.apps.into_iter().filter_map(|a| {
            let pkg = a.package_name.filter(|p| validate::package(p).is_ok())?;
            let app_id = a
                .app_id
                .filter(|id| validate::firebase_app_id(id).is_ok())?;
            Some((pkg, app_id))
        }));
        match resp.next_page_token.filter(|t| !t.is_empty()) {
            Some(t) => page = Some(t),
            None => break,
        }
    }
    Ok(out)
}

/// Mapa `pacote -> app Firebase` para os pacotes pedidos.
/// Para assim que todos forem encontrados. Projetos sem permissão são ignorados.
pub async fn apps_for(
    http: &GoogleHttp,
    token: &str,
    packages: &[String],
) -> AppResult<HashMap<String, FirebaseApp>> {
    let mut found = HashMap::new();
    if packages.is_empty() {
        return Ok(found);
    }
    for project in list_projects(http, token).await? {
        let Ok(apps) = android_apps(http, token, &project).await else {
            continue;
        };
        for (pkg, app_id) in apps {
            if packages.contains(&pkg) {
                found.entry(pkg).or_insert_with(|| FirebaseApp {
                    project_id: project.clone(),
                    app_id,
                });
            }
        }
        if found.len() == packages.len() {
            break;
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pages() {
        let p: ProjectsPage = serde_json::from_str(
            r#"{"results":[{"projectId":"meu-app-1a2b3","displayName":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(p.results[0].project_id, "meu-app-1a2b3");
        let a: AndroidAppsPage = serde_json::from_str(
            r#"{"apps":[{"packageName":"com.acme.app","appId":"1:2:android:3f"}]}"#,
        )
        .unwrap();
        assert_eq!(a.apps[0].package_name.as_deref(), Some("com.acme.app"));
        assert_eq!(a.apps[0].app_id.as_deref(), Some("1:2:android:3f"));
    }

    #[test]
    fn urls_stay_on_the_api_host() {
        for path in ["projects", "projects/meu-app-1a2b3/androidApps"] {
            let u = url(path);
            assert_eq!(u.host_str(), Some("firebase.googleapis.com"), "{path}");
            assert!(u.path().ends_with(path), "{path} -> {u}");
        }
    }
}
