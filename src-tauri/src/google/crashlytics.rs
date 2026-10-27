//! Firebase Crashlytics API v1alpha: a mesma que o Android Studio (App Quality
//! Insights) e a CLI do Firebase usam. Não precisa de BigQuery, então funciona
//! no plano Spark sem configurar nada.
//!
//! Limite da API: só os últimos 90 dias. O histórico mais antigo fica no banco local.
//! Só usamos leitura (GET); `http.rs` bloqueia qualquer outro método para este host.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use url::Url;

use crate::error::{AppError, AppResult};
use crate::http::GoogleHttp;
use crate::validate;

const BASE: &str = "https://firebasecrashlytics.googleapis.com/v1alpha/";
/// A API aceita intervalos de até 90 dias atrás; 89 dá margem para fuso/relógio.
pub const MAX_DAYS: i64 = 89;
const MAX_PAGES: usize = 10;

fn app_path(app_id: &str) -> AppResult<String> {
    let app_id = validate::firebase_app_id(app_id)?;
    let number = app_id.split(':').nth(1).unwrap_or_default();
    Ok(format!("projects/{number}/apps/{app_id}"))
}

fn url(path: &str) -> Url {
    Url::parse(&format!("{BASE}{path}")).expect("URL da API válida")
}

fn int(v: &Option<String>) -> i64 {
    v.as_deref().and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// A API devolve "percentage"; normalizamos para fração (0–1), como as taxas do Play.
fn fraction(v: Option<f64>) -> Option<f64> {
    v.map(|p| if p > 1.0001 { p / 100.0 } else { p })
}

fn day_of(ts: &Option<String>) -> Option<String> {
    let t = DateTime::parse_from_rfc3339(ts.as_deref()?).ok()?;
    Some(t.with_timezone(&Utc).format("%Y-%m-%d").to_string())
}

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub id: Option<String>,
    pub error_type: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub first_seen_version: Option<String>,
    pub last_seen_version: Option<String>,
    pub first_seen_time: Option<String>,
    pub last_seen_time: Option<String>,
    pub uri: Option<String>,
    pub sample_event: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Version {
    build_version: Option<String>,
    display_version: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntervalMetrics {
    start_time: Option<String>,
    events_count: Option<String>,
    impacted_users_count: Option<String>,
    crash_free_users_percentage: Option<f64>,
    crash_free_sessions_percentage: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportGroup {
    version: Option<Version>,
    issue: Option<Issue>,
    #[serde(default)]
    metrics: Vec<IntervalMetrics>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    #[serde(default)]
    groups: Vec<ReportGroup>,
    next_page_token: Option<String>,
}

pub struct DayMetric {
    /// `YYYY-MM-DD` (UTC), ou `None` quando o relatório não é diário.
    pub day: Option<String>,
    pub events: i64,
    pub users: i64,
    pub crash_free_users: Option<f64>,
    pub crash_free_sessions: Option<f64>,
}

impl From<&IntervalMetrics> for DayMetric {
    fn from(m: &IntervalMetrics) -> Self {
        Self {
            day: day_of(&m.start_time),
            events: int(&m.events_count),
            users: int(&m.impacted_users_count),
            crash_free_users: fraction(m.crash_free_users_percentage),
            crash_free_sessions: fraction(m.crash_free_sessions_percentage),
        }
    }
}

pub struct IssueStats {
    pub issue: Issue,
    pub metrics: Vec<DayMetric>,
}

pub struct VersionStats {
    pub build_version: String,
    pub display_version: Option<String>,
    pub metrics: Vec<DayMetric>,
}

pub struct Filter<'a> {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub error_type: Option<&'a str>,
    pub issue_id: Option<&'a str>,
    pub daily: bool,
}

async fn report(
    http: &GoogleHttp,
    token: &str,
    app_id: &str,
    name: &str,
    f: &Filter<'_>,
) -> AppResult<Vec<ReportGroup>> {
    let path = format!("{}/reports/{name}", app_path(app_id)?);
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let mut u = url(&path);
        {
            let mut q = u.query_pairs_mut();
            q.append_pair(
                "filter.interval.start_time",
                &f.start.to_rfc3339_opts(SecondsFormat::Secs, true),
            )
            .append_pair(
                "filter.interval.end_time",
                &f.end.to_rfc3339_opts(SecondsFormat::Secs, true),
            )
            .append_pair("page_size", "100");
            if f.daily {
                q.append_pair("granularity", "TIME_GRANULARITY_DAY");
            }
            if let Some(t) = f.error_type {
                q.append_pair("filter.issue.error_types", t);
            }
            if let Some(id) = f.issue_id {
                q.append_pair("filter.issue.id", validate::issue_id(id)?);
            }
            if let Some(p) = &page {
                q.append_pair("page_token", p);
            }
        }
        let r: Report = http.get_json(u, token).await?;
        out.extend(r.groups);
        match r.next_page_token.filter(|t| !t.is_empty()) {
            Some(t) => page = Some(t),
            None => break,
        }
    }
    Ok(out)
}

/// Issues com eventos e usuários afetados (por dia, se `daily`).
pub async fn top_issues(
    http: &GoogleHttp,
    token: &str,
    app_id: &str,
    f: &Filter<'_>,
) -> AppResult<Vec<IssueStats>> {
    Ok(report(http, token, app_id, "topIssues", f)
        .await?
        .into_iter()
        .filter_map(|g| {
            let issue = g.issue?;
            validate::issue_id(issue.id.as_deref()?).ok()?;
            Some(IssueStats {
                metrics: g.metrics.iter().map(DayMetric::from).collect(),
                issue,
            })
        })
        .collect())
}

/// Versões com eventos, usuários afetados e % de usuários/sessões sem falha.
pub async fn top_versions(
    http: &GoogleHttp,
    token: &str,
    app_id: &str,
    f: &Filter<'_>,
) -> AppResult<Vec<VersionStats>> {
    Ok(report(http, token, app_id, "topVersions", f)
        .await?
        .into_iter()
        .filter_map(|g| {
            let v = g.version?;
            Some(VersionStats {
                build_version: v.build_version.filter(|b| b.len() <= 40)?,
                display_version: v.display_version.map(|d| d.chars().take(80).collect()),
                metrics: g.metrics.iter().map(DayMetric::from).collect(),
            })
        })
        .collect())
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub symbol: Option<String>,
    pub file: Option<String>,
    pub line: Option<String>,
    pub library: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Exception {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub exception_message: Option<String>,
    #[serde(default)]
    pub frames: Vec<Frame>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub name: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub blamed: bool,
    #[serde(default)]
    pub crashed: bool,
    #[serde(default)]
    pub frames: Vec<Frame>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub issue: Option<Issue>,
    pub blame_frame: Option<Frame>,
    #[serde(default)]
    pub exceptions: Vec<Exception>,
    #[serde(default)]
    pub threads: Vec<Thread>,
}

impl Event {
    /// Tipo da exceção mais interna (a última da lista).
    pub fn root_type(&self) -> Option<String> {
        self.exceptions.iter().rev().find_map(|e| e.kind.clone())
    }

    /// Stack trace legível (exceções; ou threads culpadas/travadas em ANRs).
    pub fn stack_text(&self) -> String {
        fn frames(out: &mut String, fs: &[Frame]) {
            for f in fs.iter().take(200) {
                out.push_str(&format!(
                    "    at {} ({}:{})\n",
                    f.symbol.as_deref().unwrap_or("?"),
                    f.file.as_deref().or(f.library.as_deref()).unwrap_or("?"),
                    f.line.as_deref().unwrap_or("?"),
                ));
            }
        }
        let mut out = String::new();
        for (i, e) in self.exceptions.iter().enumerate() {
            if i > 0 {
                out.push_str("Caused by: ");
            }
            out.push_str(e.kind.as_deref().unwrap_or("?"));
            if let Some(m) = &e.exception_message {
                out.push_str(": ");
                out.push_str(m);
            }
            out.push('\n');
            frames(&mut out, &e.frames);
        }
        if out.is_empty() {
            for t in self.threads.iter().filter(|t| t.blamed || t.crashed) {
                let name = t.name.as_deref().or(t.title.as_deref()).unwrap_or("?");
                out.push_str(&format!(
                    "\"{name}\"{}\n",
                    if t.blamed { " (culpada)" } else { "" }
                ));
                frames(&mut out, &t.frames);
                out.push('\n');
            }
        }
        out.chars().take(100_000).collect()
    }
}

#[derive(Deserialize)]
struct BatchGetResponse {
    #[serde(default)]
    events: Vec<Event>,
}

/// Eventos de amostra pelo nome completo (`projects/.../events/...`), até 100 por chamada.
pub async fn sample_events(
    http: &GoogleHttp,
    token: &str,
    app_id: &str,
    names: &[String],
) -> AppResult<Vec<Event>> {
    let base = app_path(app_id)?;
    let prefix = format!("{base}/events/");
    let names: Vec<&String> = names
        .iter()
        .filter(|n| {
            n.strip_prefix(&prefix)
                .is_some_and(|id| validate::issue_id(id).is_ok())
        })
        .collect();
    let mut out = Vec::new();
    for chunk in names.chunks(100) {
        let build = |mask: bool| {
            let mut u = url(&format!("{base}/events:batchGet"));
            {
                let mut q = u.query_pairs_mut();
                for n in chunk {
                    q.append_pair("names", n);
                }
                if mask {
                    // Só o necessário: sem logs, breadcrumbs ou custom keys.
                    q.append_pair("read_mask", "issue,blame_frame,exceptions,threads");
                }
            }
            u
        };
        let resp: BatchGetResponse = match http.get_json(build(true), token).await {
            Ok(r) => r,
            // Se a máscara não for aceita, busca o evento completo.
            Err(AppError::Api { status: 400, .. }) => http.get_json(build(false), token).await?,
            Err(e) => return Err(e),
        };
        out.extend(resp.events);
    }
    Ok(out)
}

/// `"1.2.3 (456)"` ou `"456"` -> `"456"` (versionCode).
pub fn version_code(v: &str) -> Option<String> {
    let v = v.trim();
    if !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()) {
        return Some(v.to_string());
    }
    let inner = v.rsplit_once('(')?.1.strip_suffix(')')?;
    inner
        .chars()
        .all(|c| c.is_ascii_digit())
        .then(|| inner.to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_paths_from_app_id() {
        assert_eq!(
            app_path("1:123456:android:ab12").unwrap(),
            "projects/123456/apps/1:123456:android:ab12"
        );
        assert!(app_path("1:123/../x:android:ab").is_err());
        let u = url(&format!(
            "{}/reports/topIssues",
            app_path("1:9:android:f").unwrap()
        ));
        assert_eq!(u.host_str(), Some("firebasecrashlytics.googleapis.com"));
    }

    #[test]
    fn parses_report_and_normalizes_percentages() {
        let raw = r#"{"groups":[{"issue":{"id":"abc123","errorType":"FATAL","sampleEvent":"projects/9/apps/1:9:android:f/events/e1"},
          "metrics":[{"startTime":"2026-10-20T00:00:00Z","eventsCount":"12","impactedUsersCount":"5","crashFreeUsersPercentage":99.5}]}]}"#;
        let r: Report = serde_json::from_str(raw).unwrap();
        let m = DayMetric::from(&r.groups[0].metrics[0]);
        assert_eq!(m.day.as_deref(), Some("2026-10-20"));
        assert_eq!((m.events, m.users), (12, 5));
        assert!((m.crash_free_users.unwrap() - 0.995).abs() < 1e-9);
    }

    #[test]
    fn extracts_version_code() {
        assert_eq!(version_code("1.2.3 (456)").as_deref(), Some("456"));
        assert_eq!(version_code("456").as_deref(), Some("456"));
        assert_eq!(version_code("1.2.3").as_deref(), None);
    }

    #[test]
    fn renders_stack_text() {
        let e: Event = serde_json::from_str(
            r#"{"exceptions":[{"type":"java.lang.IllegalStateException","exceptionMessage":"boom",
              "frames":[{"symbol":"com.acme.Foo.bar","file":"Foo.kt","line":"42"}]}]}"#,
        )
        .unwrap();
        assert_eq!(
            e.root_type().as_deref(),
            Some("java.lang.IllegalStateException")
        );
        assert!(e.stack_text().contains("at com.acme.Foo.bar (Foo.kt:42)"));
    }
}
