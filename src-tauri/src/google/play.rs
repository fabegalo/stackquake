//! Google Play Developer Reporting API v1beta1 (Android Vitals).
//! Gratuita; exige só a permissão "Ver informações do app" no Play Console.

use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;
use url::Url;

use crate::error::AppResult;
use crate::http::GoogleHttp;
use crate::validate;

const BASE: &str = "https://playdeveloperreporting.googleapis.com/v1beta1/";

/// Concatena em vez de usar `Url::join`: com `join`, um caminho como
/// `apps:search` seria lido como uma URL absoluta de esquema `apps:`.
fn url(path: &str) -> Url {
    Url::parse(&format!("{BASE}{path}")).expect("URL da API válida")
}

#[derive(Clone, Copy)]
pub enum MetricSet {
    Crash,
    Anr,
}

impl MetricSet {
    pub fn id(self) -> &'static str {
        match self {
            MetricSet::Crash => "crash",
            MetricSet::Anr => "anr",
        }
    }
    fn resource(self) -> &'static str {
        match self {
            MetricSet::Crash => "crashRateMetricSet",
            MetricSet::Anr => "anrRateMetricSet",
        }
    }
    fn metrics(self) -> [&'static str; 3] {
        match self {
            MetricSet::Crash => ["crashRate", "userPerceivedCrashRate", "distinctUsers"],
            MetricSet::Anr => ["anrRate", "userPerceivedAnrRate", "distinctUsers"],
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct App {
    pub package_name: String,
    pub display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppsPage {
    #[serde(default)]
    apps: Vec<App>,
    next_page_token: Option<String>,
}

pub async fn search_apps(http: &GoogleHttp, token: &str) -> AppResult<Vec<App>> {
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    for _ in 0..50 {
        let mut u = url("apps:search");
        u.query_pairs_mut().append_pair("pageSize", "100");
        if let Some(p) = &page {
            u.query_pairs_mut().append_pair("pageToken", p);
        }
        let resp: AppsPage = http.get_json(u, token).await?;
        out.extend(
            resp.apps
                .into_iter()
                .filter(|a| validate::package(&a.package_name).is_ok()),
        );
        match resp.next_page_token.filter(|t| !t.is_empty()) {
            Some(t) => page = Some(t),
            None => break,
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
struct DateTime {
    year: Option<i32>,
    month: Option<u32>,
    day: Option<u32>,
}

impl DateTime {
    fn date(&self) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year?, self.month?, self.day?)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Freshness {
    aggregation_period: Option<String>,
    latest_end_time: Option<DateTime>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FreshnessInfo {
    #[serde(default)]
    freshnesses: Vec<Freshness>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetricSetInfo {
    freshness_info: Option<FreshnessInfo>,
}

/// Último dia (exclusivo) com dados diários disponíveis.
pub async fn daily_freshness(
    http: &GoogleHttp,
    token: &str,
    pkg: &str,
    set: MetricSet,
) -> AppResult<Option<NaiveDate>> {
    let pkg = validate::package(pkg)?;
    let info: MetricSetInfo = http
        .get_json(url(&format!("apps/{pkg}/{}", set.resource())), token)
        .await?;
    Ok(info
        .freshness_info
        .map(|f| f.freshnesses)
        .unwrap_or_default()
        .into_iter()
        .find(|f| f.aggregation_period.as_deref() == Some("DAILY"))
        .and_then(|f| f.latest_end_time)
        .and_then(|d| d.date()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Decimal {
    value: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetricValue {
    metric: String,
    decimal_value: Option<Decimal>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DimensionValue {
    dimension: String,
    string_value: Option<String>,
    int64_value: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetricsRow {
    start_time: Option<DateTime>,
    #[serde(default)]
    dimensions: Vec<DimensionValue>,
    #[serde(default)]
    metrics: Vec<MetricValue>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueryResponse {
    #[serde(default)]
    rows: Vec<MetricsRow>,
    next_page_token: Option<String>,
}

pub struct DailyMetric {
    pub day: NaiveDate,
    /// `None` = total do app (sem dimensão).
    pub version_code: Option<String>,
    pub rate: Option<f64>,
    pub user_perceived_rate: Option<f64>,
    pub distinct_users: Option<i64>,
}

fn ymd(d: NaiveDate) -> serde_json::Value {
    use chrono::Datelike;
    json!({ "year": d.year(), "month": d.month(), "day": d.day() })
}

/// Série diária `[start, end)`. Fuso: o padrão do metric set (America/Los_Angeles).
pub async fn query_daily(
    http: &GoogleHttp,
    token: &str,
    pkg: &str,
    set: MetricSet,
    start: NaiveDate,
    end: NaiveDate,
    by_version: bool,
) -> AppResult<Vec<DailyMetric>> {
    let pkg = validate::package(pkg)?;
    let [m_rate, m_user, _] = set.metrics();
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    for _ in 0..100 {
        let mut body = json!({
            "timelineSpec": {
                "aggregationPeriod": "DAILY",
                "startTime": ymd(start),
                "endTime": ymd(end),
            },
            "dimensions": if by_version { vec!["versionCode"] } else { vec![] },
            "metrics": set.metrics(),
            "pageSize": 10000,
        });
        if let Some(p) = &page {
            body["pageToken"] = json!(p);
        }
        let resp: QueryResponse = http
            .post_json(
                url(&format!("apps/{pkg}/{}:query", set.resource())),
                token,
                &body,
            )
            .await?;
        for row in resp.rows {
            let Some(day) = row.start_time.as_ref().and_then(DateTime::date) else {
                continue;
            };
            let metric = |name: &str| {
                row.metrics
                    .iter()
                    .find(|m| m.metric == name)
                    .and_then(|m| m.decimal_value.as_ref())
                    .and_then(|d| d.value.as_deref())
                    .and_then(|v| v.parse::<f64>().ok())
            };
            let version_code = row
                .dimensions
                .iter()
                .find(|d| d.dimension == "versionCode")
                .and_then(|d| d.int64_value.clone().or_else(|| d.string_value.clone()));
            out.push(DailyMetric {
                day,
                version_code,
                rate: metric(m_rate),
                user_perceived_rate: metric(m_user),
                distinct_users: metric("distinctUsers").map(|v| v.round() as i64),
            });
        }
        match resp.next_page_token.filter(|t| !t.is_empty()) {
            Some(t) => page = Some(t),
            None => break,
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppVersion {
    pub version_code: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorIssue {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub cause: Option<String>,
    pub location: Option<String>,
    pub error_report_count: Option<String>,
    pub distinct_users: Option<String>,
    pub first_app_version: Option<AppVersion>,
    pub last_app_version: Option<AppVersion>,
    pub last_error_report_time: Option<String>,
    pub issue_uri: Option<String>,
}

impl ErrorIssue {
    /// `apps/{app}/errorIssues/{id}` -> `{id}`.
    pub fn id(&self) -> Option<&str> {
        self.name
            .rsplit('/')
            .next()
            .filter(|id| validate::issue_id(id).is_ok())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssuesPage {
    #[serde(default)]
    error_issues: Vec<ErrorIssue>,
    next_page_token: Option<String>,
}

/// Issues mais frequentes (janela padrão da API), ordenados por número de relatórios.
pub async fn search_issues(
    http: &GoogleHttp,
    token: &str,
    pkg: &str,
    max: usize,
) -> AppResult<Vec<ErrorIssue>> {
    let pkg = validate::package(pkg)?;
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    while out.len() < max {
        let mut u = url(&format!("apps/{pkg}/errorIssues:search"));
        u.query_pairs_mut()
            .append_pair("pageSize", "100")
            .append_pair("orderBy", "errorReportCount desc");
        if let Some(p) = &page {
            u.query_pairs_mut().append_pair("pageToken", p);
        }
        let resp: IssuesPage = http.get_json(u, token).await?;
        out.extend(resp.error_issues);
        match resp.next_page_token.filter(|t| !t.is_empty()) {
            Some(t) => page = Some(t),
            None => break,
        }
    }
    out.truncate(max);
    Ok(out)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorReport {
    report_text: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportsPage {
    #[serde(default)]
    error_reports: Vec<ErrorReport>,
}

/// Texto de um relatório de amostra do issue (stack ou dump de ANR).
pub async fn sample_report(
    http: &GoogleHttp,
    token: &str,
    pkg: &str,
    issue_id: &str,
) -> AppResult<Option<String>> {
    let pkg = validate::package(pkg)?;
    let issue_id = validate::issue_id(issue_id)?;
    let mut u = url(&format!("apps/{pkg}/errorReports:search"));
    u.query_pairs_mut()
        .append_pair("pageSize", "1")
        .append_pair("filter", &format!("errorIssueId = {issue_id}"));
    let resp: ReportsPage = http.get_json(u, token).await?;
    Ok(resp.error_reports.into_iter().find_map(|r| r.report_text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metric_rows() {
        let raw = r#"{"rows":[{"aggregationPeriod":"DAILY","startTime":{"year":2026,"month":10,"day":1,"timeZone":{"id":"America/Los_Angeles"}},
          "dimensions":[{"dimension":"versionCode","int64Value":"412"}],
          "metrics":[{"metric":"crashRate","decimalValue":{"value":"0.0123"}},{"metric":"distinctUsers","decimalValue":{"value":"5321"}}]}]}"#;
        let r: QueryResponse = serde_json::from_str(raw).unwrap();
        let row = &r.rows[0];
        assert_eq!(
            row.start_time.as_ref().unwrap().date(),
            NaiveDate::from_ymd_opt(2026, 10, 1)
        );
        assert_eq!(row.dimensions[0].int64_value.as_deref(), Some("412"));
    }

    #[test]
    fn issue_id_from_name() {
        let i: ErrorIssue =
            serde_json::from_str(r#"{"name":"apps/com.acme/errorIssues/8f3a1c"}"#).unwrap();
        assert_eq!(i.id(), Some("8f3a1c"));
        let bad: ErrorIssue =
            serde_json::from_str(r#"{"name":"apps/com.acme/errorIssues/1 OR x"}"#).unwrap();
        assert_eq!(bad.id(), None);
    }

    #[test]
    fn urls_stay_on_the_api_host() {
        for path in [
            "apps:search",
            "apps/com.acme/crashRateMetricSet:query",
            "apps/com.acme/errorIssues:search",
        ] {
            let u = url(path);
            assert_eq!(
                u.host_str(),
                Some("playdeveloperreporting.googleapis.com"),
                "{path}"
            );
            assert_eq!(u.scheme(), "https");
            assert!(u.path().ends_with(path), "{path} -> {u}");
        }
    }
}
