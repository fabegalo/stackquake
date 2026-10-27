//! Incremental sync of both sources into the account's local database.
//!
//! Each source fails independently: a Crashlytics error does not prevent Play
//! data from being saved (it becomes a warning in the report).

use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Serialize;

use crate::db::{ClDayIn, ClIssueDetails, ClIssueIn, Db, PlayIssueIn, ProblemDetail, VitalsIn};
use crate::error::{AppError, AppResult, ErrorPayload};
use crate::google::crashlytics::{self, Filter};
use crate::google::firebase;
use crate::google::play::{self, MetricSet};
use crate::http::GoogleHttp;
use crate::{unify, validate};

const PLAY_FIRST_DAYS: i64 = 90;
const PLAY_WINDOW_DAYS: i64 = 30;
/// Days re-synced every time (late-arriving data).
const OVERLAP_DAYS: i64 = 3;
const MAX_PLAY_ISSUES: usize = 500;
const MAX_ANR_SAMPLES: usize = 30;
/// Crashlytics issues that get their stack/final signature per sync.
const MAX_CL_DETAILS: usize = 300;

/// What each step needs: network, the account database and a token.
pub struct Ctx<'a> {
    pub http: &'a GoogleHttp,
    pub db: &'a Db,
    pub token: &'a str,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub package_name: String,
    pub vitals_rows: usize,
    pub play_issues: usize,
    pub crashlytics_issues: usize,
    pub crashlytics_versions: usize,
    pub regressions: usize,
    pub warnings: Vec<SyncWarning>,
}

/// A non-fatal problem in one source during a sync (translated by the UI).
#[derive(Serialize)]
pub struct SyncWarning {
    /// `play_vitals`, `play_issues` or `crashlytics`.
    pub source: &'static str,
    pub error: ErrorPayload,
}

impl SyncWarning {
    fn new(source: &'static str, error: AppError) -> Self {
        Self {
            source,
            error: error.payload(),
        }
    }
}

fn day(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn parse_day(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn midnight(d: NaiveDate) -> DateTime<Utc> {
    d.and_hms_opt(0, 0, 0).expect("valid midnight").and_utc()
}

/// Maps Crashlytics API failures to translatable codes.
fn crashlytics_error(e: AppError) -> AppError {
    match e {
        // API disabled in the OAuth client's project (the project of whoever
        // publishes Stackquake), not a permission problem of the user.
        AppError::Api {
            status: 403,
            message,
        } if message.contains("has not been used in project")
            || message.contains("it is disabled") =>
        {
            AppError::known("crashlytics_api_disabled")
        }
        AppError::Api {
            status: 403,
            message,
        } => AppError::known_with("crashlytics_no_permission", vec![("detail", message)]),
        AppError::Api { status: 404, .. } => AppError::known("crashlytics_no_data"),
        other => other,
    }
}

/// Finds the Firebase project/App ID of apps that don't have one yet.
/// Failures here never block the rest.
pub async fn discover_firebase_apps(ctx: &Ctx<'_>) -> AppResult<usize> {
    let missing: Vec<String> = ctx
        .db
        .list_apps()?
        .into_iter()
        .filter(|a| a.firebase_app_id.is_none())
        .map(|a| a.package_name)
        .collect();
    let found = firebase::apps_for(ctx.http, ctx.token, &missing).await?;
    for (pkg, app) in &found {
        ctx.db.set_firebase_app(pkg, &app.project_id, &app.app_id)?;
    }
    Ok(found.len())
}

pub async fn sync_app(ctx: &Ctx<'_>, pkg: &str) -> AppResult<SyncReport> {
    let pkg = validate::package(pkg)?.to_string();
    if ctx.db.get_app(&pkg)?.firebase_app_id.is_none() {
        let _ = discover_firebase_apps(ctx).await;
    }
    let app = ctx.db.get_app(&pkg)?;
    let mut report = SyncReport {
        package_name: pkg.clone(),
        ..Default::default()
    };

    for set in [MetricSet::Crash, MetricSet::Anr] {
        match sync_vitals(ctx, &pkg, set).await {
            Ok(n) => report.vitals_rows += n,
            Err(e) => report.warnings.push(SyncWarning::new("play_vitals", e)),
        }
    }

    match sync_play_issues(ctx, &pkg).await {
        Ok(n) => report.play_issues = n,
        Err(e) => report.warnings.push(SyncWarning::new("play_issues", e)),
    }

    match app.firebase_app_id.as_deref() {
        Some(app_id) => match sync_crashlytics(ctx, &pkg, app_id).await {
            Ok((issues, versions)) => {
                report.crashlytics_issues = issues;
                report.crashlytics_versions = versions;
            }
            Err(e) => report
                .warnings
                .push(SyncWarning::new("crashlytics", crashlytics_error(e))),
        },
        None => report.warnings.push(SyncWarning::new(
            "crashlytics",
            AppError::known("crashlytics_no_firebase_project"),
        )),
    }

    report.regressions = ctx.db.unify(&pkg)?;
    ctx.db.mark_synced(&pkg)?;
    Ok(report)
}

async fn sync_vitals(ctx: &Ctx<'_>, pkg: &str, set: MetricSet) -> AppResult<usize> {
    let source = format!("play-{}", set.id());
    let today = Utc::now().date_naive();
    let end = play::daily_freshness(ctx.http, ctx.token, pkg, set)
        .await?
        .unwrap_or(today)
        .min(today);
    let mut start = ctx
        .db
        .cursor(&source, pkg)?
        .as_deref()
        .and_then(parse_day)
        .map(|c| c - Duration::days(OVERLAP_DAYS))
        .unwrap_or(end - Duration::days(PLAY_FIRST_DAYS));

    let mut total = 0;
    while start < end {
        let window_end = (start + Duration::days(PLAY_WINDOW_DAYS)).min(end);
        let mut rows = Vec::new();
        for by_version in [true, false] {
            let metrics =
                play::query_daily(ctx.http, ctx.token, pkg, set, start, window_end, by_version)
                    .await?;
            rows.extend(metrics.into_iter().map(|m| VitalsIn {
                day: day(m.day),
                version_code: m.version_code.unwrap_or_else(|| "*".to_string()),
                rate: m.rate,
                user_perceived_rate: m.user_perceived_rate,
                distinct_users: m.distinct_users,
            }));
        }
        ctx.db.upsert_vitals(pkg, set.id(), &rows)?;
        ctx.db.set_cursor(&source, pkg, &day(window_end))?;
        total += rows.len();
        start = window_end;
    }
    Ok(total)
}

async fn sync_play_issues(ctx: &Ctx<'_>, pkg: &str) -> AppResult<usize> {
    let issues = play::search_issues(ctx.http, ctx.token, pkg, MAX_PLAY_ISSUES).await?;
    let mut out = Vec::with_capacity(issues.len());
    let mut samples_fetched = 0;
    for issue in &issues {
        let Some(id) = issue.id() else { continue };
        let kind = unify::kind_from_play(issue.kind.as_deref().unwrap_or("CRASH"));
        let cause = issue.cause.clone().unwrap_or_default();
        let location = issue.location.clone().unwrap_or_default();

        // ANR: Play does not give the blamed method; we use the first app frame
        // of the sample report (cached so it is not fetched again).
        let mut sample = None;
        let mut sig_location = location.clone();
        if kind == "anr" {
            let cached = ctx.db.play_sample(pkg, id)?;
            if cached.is_none() && samples_fetched < MAX_ANR_SAMPLES {
                samples_fetched += 1;
                sample = play::sample_report(ctx.http, ctx.token, pkg, id)
                    .await
                    .ok()
                    .flatten();
            }
            if let Some(frame) = cached
                .as_deref()
                .or(sample.as_deref())
                .and_then(unify::first_app_frame)
            {
                sig_location = frame;
            }
        }

        out.push(PlayIssueIn {
            issue_id: id.to_string(),
            kind: kind.to_string(),
            title: unify::title(kind, &cause, &sig_location),
            signature: unify::signature(kind, &cause, &sig_location),
            cause: issue.cause.clone(),
            location: issue.location.clone(),
            report_count: issue
                .error_report_count
                .as_deref()
                .and_then(|v| v.parse().ok()),
            distinct_users: issue.distinct_users.as_deref().and_then(|v| v.parse().ok()),
            first_version: issue
                .first_app_version
                .as_ref()
                .and_then(|v| v.version_code.clone()),
            last_version: issue
                .last_app_version
                .as_ref()
                .and_then(|v| v.version_code.clone()),
            last_seen: issue.last_error_report_time.clone(),
            issue_uri: issue
                .issue_uri
                .clone()
                .filter(|u| u.starts_with("https://play.google.com/")),
            sample_report: sample,
        });
    }
    ctx.db.upsert_play_issues(pkg, &out)?;
    Ok(out.len())
}

/// Daily rows of a report (skips points without a date).
fn day_rows(key: &str, display: Option<&str>, metrics: &[crashlytics::DayMetric]) -> Vec<ClDayIn> {
    metrics
        .iter()
        .filter_map(|m| {
            Some(ClDayIn {
                key: key.to_string(),
                display_version: display.map(str::to_string),
                day: m.day.clone()?,
                events: m.events,
                users: m.users,
                crash_free_users: m.crash_free_users,
                crash_free_sessions: m.crash_free_sessions,
            })
        })
        .collect()
}

async fn sync_crashlytics(ctx: &Ctx<'_>, pkg: &str, app_id: &str) -> AppResult<(usize, usize)> {
    let source = "crashlytics";
    let now = Utc::now();
    let earliest = now - Duration::days(crashlytics::MAX_DAYS);
    let start = ctx
        .db
        .cursor(source, pkg)?
        .as_deref()
        .and_then(parse_day)
        .map(|c| midnight(c - Duration::days(OVERLAP_DAYS)))
        .map_or(earliest, |s| s.max(earliest));
    let filter = |error_type| Filter {
        start,
        end: now,
        error_type,
        issue_id: None,
        daily: true,
    };

    // Issues: metadata + events/users per day.
    let issues = crashlytics::top_issues(ctx.http, ctx.token, app_id, &filter(None)).await?;
    let mut issue_rows = Vec::with_capacity(issues.len());
    let mut daily = Vec::new();
    for s in &issues {
        let i = &s.issue;
        let Some(id) = i.id.as_deref() else { continue };
        let kind = unify::kind_from_crashlytics(i.error_type.as_deref().unwrap_or("FATAL"));
        daily.extend(day_rows(id, None, &s.metrics));
        issue_rows.push(ClIssueIn {
            issue_id: id.to_string(),
            kind: kind.to_string(),
            title: i
                .title
                .clone()
                .unwrap_or_else(|| format!("Issue {id}"))
                .chars()
                .take(300)
                .collect(),
            subtitle: i.subtitle.as_ref().map(|t| t.chars().take(500).collect()),
            fallback_signature: format!("{kind}|cl:{id}"),
            first_version: i
                .first_seen_version
                .as_deref()
                .and_then(crashlytics::version_code),
            last_version: i
                .last_seen_version
                .as_deref()
                .and_then(crashlytics::version_code),
            first_seen: i.first_seen_time.clone(),
            last_seen: i.last_seen_time.clone(),
            issue_uri: i
                .uri
                .clone()
                .filter(|u| u.starts_with("https://console.firebase.google.com/")),
            sample_event: i.sample_event.clone(),
        });
    }
    ctx.db.upsert_crashlytics_issues(pkg, &issue_rows)?;
    ctx.db.upsert_crashlytics_issue_daily(pkg, &daily)?;

    // Versions: overall (with crash-free %), crashes only and ANRs only.
    let mut versions = 0;
    for (kind, error_type) in [
        ("all", None),
        ("crash", Some("FATAL")),
        ("anr", Some("ANR")),
    ] {
        let stats =
            crashlytics::top_versions(ctx.http, ctx.token, app_id, &filter(error_type)).await?;
        let rows: Vec<ClDayIn> = stats
            .iter()
            .flat_map(|v| day_rows(&v.build_version, v.display_version.as_deref(), &v.metrics))
            .collect();
        if kind == "all" {
            versions = stats.len();
        }
        ctx.db.upsert_crashlytics_version_daily(pkg, kind, &rows)?;
    }

    // Sample stack -> final signature (links it with Play).
    let pending = ctx.db.crashlytics_without_details(pkg, MAX_CL_DETAILS)?;
    fetch_details(ctx, pkg, app_id, &pending).await?;

    ctx.db.set_cursor(source, pkg, &day(now.date_naive()))?;
    Ok((issue_rows.len(), versions))
}

/// Fetches sample events and stores each issue's stack + signature.
async fn fetch_details(
    ctx: &Ctx<'_>,
    pkg: &str,
    app_id: &str,
    pending: &[(String, String, String)],
) -> AppResult<usize> {
    if pending.is_empty() {
        return Ok(0);
    }
    let names: Vec<String> = pending.iter().map(|(_, ev, _)| ev.clone()).collect();
    let kinds: HashMap<&str, &str> = pending
        .iter()
        .map(|(id, _, k)| (id.as_str(), k.as_str()))
        .collect();
    let events = crashlytics::sample_events(ctx.http, ctx.token, app_id, &names).await?;
    let mut n = 0;
    for ev in &events {
        let Some(id) = ev.issue.as_ref().and_then(|i| i.id.as_deref()) else {
            continue;
        };
        let Some(kind) = kinds.get(id).copied() else {
            continue;
        };
        let root = ev.root_type();
        let blame = ev.blame_frame.as_ref().and_then(|f| f.symbol.clone());
        let (rt, bl) = (
            root.as_deref().unwrap_or(""),
            blame.as_deref().unwrap_or(""),
        );
        ctx.db.set_crashlytics_details(
            pkg,
            &ClIssueDetails {
                issue_id: id.to_string(),
                signature: unify::signature(kind, rt, bl),
                title: unify::title(kind, rt, bl),
                stack: ev.stack_text(),
                root_type: root,
                blame_symbol: blame,
            },
        )?;
        n += 1;
    }
    Ok(n)
}

/// Fetches on demand what the detail view is missing: sample stacks from both
/// sources and the per-version distribution of each Crashlytics issue.
pub async fn fetch_problem_details(ctx: &Ctx<'_>, id: i64) -> AppResult<ProblemDetail> {
    let detail = ctx.db.get_problem(id)?;
    let pkg = detail.problem.package_name.clone();

    if let Some(pi) = detail
        .play_issues
        .iter()
        .find(|i| i.sample_report.is_none())
    {
        if let Some(text) = play::sample_report(ctx.http, ctx.token, &pkg, &pi.issue_id).await? {
            ctx.db.set_play_sample(&pkg, &pi.issue_id, &text)?;
        }
    }

    if !detail.crashlytics_issues.is_empty() {
        let app = ctx.db.get_app(&pkg)?;
        if let Some(app_id) = app.firebase_app_id.as_deref() {
            let now = Utc::now();
            let mut pending = Vec::new();
            for ci in &detail.crashlytics_issues {
                let f = Filter {
                    start: now - Duration::days(crashlytics::MAX_DAYS),
                    end: now,
                    error_type: None,
                    issue_id: Some(&ci.issue_id),
                    daily: false,
                };
                let versions = crashlytics::top_versions(ctx.http, ctx.token, app_id, &f)
                    .await
                    .map_err(crashlytics_error)?;
                let rows: Vec<ClDayIn> = versions
                    .iter()
                    .map(|v| ClDayIn {
                        key: v.build_version.clone(),
                        display_version: v.display_version.clone(),
                        day: String::new(),
                        events: v.metrics.iter().map(|m| m.events).sum(),
                        users: v.metrics.iter().map(|m| m.users).sum(),
                        crash_free_users: None,
                        crash_free_sessions: None,
                    })
                    .collect();
                ctx.db
                    .replace_crashlytics_issue_versions(&pkg, &ci.issue_id, &rows)?;
                if ci.sample_stack.is_none() {
                    if let Some((ev, kind)) = ctx.db.crashlytics_sample_event(&pkg, &ci.issue_id)? {
                        pending.push((ci.issue_id.clone(), ev, kind));
                    }
                }
            }
            fetch_details(ctx, &pkg, app_id, &pending).await?;
        }
    }

    ctx.db.get_problem(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_disabled_api_instead_of_blaming_permissions() {
        let e = crashlytics_error(AppError::Api {
            status: 403,
            message:
                "Firebase Crashlytics API has not been used in project 1 before or it is disabled."
                    .into(),
        });
        assert_eq!(e.payload().code, "crashlytics_api_disabled");
        let e = crashlytics_error(AppError::Api {
            status: 403,
            message: "caller does not have permission".into(),
        });
        assert_eq!(e.payload().code, "crashlytics_no_permission");
    }
}
