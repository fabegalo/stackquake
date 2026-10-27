//! Failure signature: the key that links a Play issue to a Crashlytics issue.
//!
//! Human-readable on purpose (easy to debug and tune):
//!     `<kind>|<SimpleExceptionType>|<Class.method>`
//!
//! - Play CRASH/NON_FATAL: `cause` (innermost exception) + `location` (likely method).
//! - Play ANR: first app frame of the sample report (or the activity, if none).
//! - Crashlytics: innermost exception type + `blame_frame.symbol`.
//!
//! It is a heuristic: if your app gets wrong merges or splits, this is the file
//! to change (tests below).

/// Framework packages skipped when looking for the "first app frame".
const FRAMEWORK_PREFIXES: &[&str] = &[
    "java.",
    "javax.",
    "jdk.",
    "sun.",
    "libcore.",
    "dalvik.",
    "android.",
    "androidx.",
    "com.android.",
    "com.google.android.",
    "kotlin.",
    "kotlinx.",
    "org.chromium.",
];

pub fn kind_from_play(t: &str) -> &'static str {
    match t {
        "APPLICATION_NOT_RESPONDING" => "anr",
        "NON_FATAL" => "nonfatal",
        _ => "crash",
    }
}

pub fn kind_from_crashlytics(t: &str) -> &'static str {
    match t {
        "ANR" => "anr",
        "NON_FATAL" => "nonfatal",
        _ => "crash",
    }
}

/// `java.lang.IllegalStateException` -> `IllegalStateException`.
pub fn simple_type(t: &str) -> &str {
    let t = t.trim();
    let t = t.split(':').next().unwrap_or(t).trim();
    t.rsplit(['.', '$']).next().unwrap_or(t)
}

/// Normalizes a stack symbol to `Class.method`, removing the package,
/// arguments, file/line and compiler-generated suffixes (lambdas, anonymous
/// classes, R8/D8 synthetics).
pub fn normalize_symbol(sym: &str) -> String {
    let s = sym.trim();
    let s = s.strip_prefix("at ").unwrap_or(s);
    let s = s.split('(').next().unwrap_or(s).trim();
    let mut parts: Vec<String> = Vec::new();
    for seg in s.split(['.', '$']) {
        let seg = seg.trim();
        let generated = seg.is_empty()
            || seg.chars().all(|c| c.is_ascii_digit())
            || seg.starts_with("lambda")
            || seg.starts_with("ExternalSynthetic")
            || seg.starts_with("$")
            || seg == "Companion"
            || seg == "invoke"
            || seg == "invokeSuspend";
        if !generated {
            parts.push(seg.to_string());
        }
    }
    let n = parts.len();
    if n >= 2 {
        format!("{}.{}", parts[n - 2], parts[n - 1])
    } else {
        parts.pop().unwrap_or_default()
    }
}

pub fn is_app_frame(sym: &str) -> bool {
    !FRAMEWORK_PREFIXES.iter().any(|p| sym.starts_with(p))
}

/// First app frame in a textual report (Java stack or ANR dump).
pub fn first_app_frame(report: &str) -> Option<String> {
    report
        .lines()
        .filter_map(|l| l.trim().strip_prefix("at "))
        .map(|f| f.split('(').next().unwrap_or(f).trim())
        .find(|f| is_app_frame(f))
        .map(normalize_symbol)
}

pub fn signature(kind: &str, exception_type: &str, location: &str) -> String {
    let ty = if kind == "anr" {
        ""
    } else {
        simple_type(exception_type)
    };
    format!("{kind}|{ty}|{}", normalize_symbol(location))
}

pub fn title(kind: &str, exception_type: &str, location: &str) -> String {
    let loc = normalize_symbol(location);
    match kind {
        "anr" if loc.is_empty() => "ANR".to_string(),
        "anr" => format!("ANR in {loc}"),
        _ if loc.is_empty() => simple_type(exception_type).to_string(),
        _ => format!("{} in {loc}", simple_type(exception_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_crash_from_both_sources_matches() {
        // Play: cause = innermost type, location = likely method.
        let play = signature(
            "crash",
            "IllegalStateException",
            "com.acme.ui.CheckoutActivity.onCreate",
        );
        // Crashlytics: fully qualified type + blame_frame.symbol.
        let cl = signature(
            "crash",
            "java.lang.IllegalStateException",
            "com.acme.ui.CheckoutActivity.onCreate",
        );
        assert_eq!(play, cl);
        assert_eq!(
            play,
            "crash|IllegalStateException|CheckoutActivity.onCreate"
        );
    }

    #[test]
    fn strips_lambdas_and_synthetic_suffixes() {
        assert_eq!(
            normalize_symbol("com.acme.Repo$load$1.invokeSuspend"),
            "Repo.load"
        );
        assert_eq!(
            normalize_symbol("com.acme.Foo.lambda$bar$0(Foo.java:12)"),
            "Foo.bar"
        );
        assert_eq!(
            normalize_symbol("com.acme.Foo$$ExternalSyntheticLambda0.run"),
            "Foo.run"
        );
    }

    #[test]
    fn finds_first_app_frame_in_anr_dump() {
        let dump = r#""main" tid=1 Blocked
  at java.lang.Object.wait(Native method)
  at android.os.Handler.dispatchMessage(Handler.java:106)
  at com.acme.db.Dao.query(Dao.kt:40)
  at com.acme.ui.Main.onResume(Main.kt:10)"#;
        assert_eq!(first_app_frame(dump).as_deref(), Some("Dao.query"));
        assert_eq!(
            signature(
                "anr",
                "Input dispatching timed out",
                "com.acme.db.Dao.query"
            ),
            "anr||Dao.query"
        );
    }

    #[test]
    fn kinds_map_consistently() {
        assert_eq!(kind_from_play("CRASH"), kind_from_crashlytics("FATAL"));
        assert_eq!(
            kind_from_play("APPLICATION_NOT_RESPONDING"),
            kind_from_crashlytics("ANR")
        );
        assert_eq!(
            kind_from_play("NON_FATAL"),
            kind_from_crashlytics("NON_FATAL")
        );
    }
}
