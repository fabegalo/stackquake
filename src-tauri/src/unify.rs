//! Assinatura de falha: a chave que liga um issue do Play a um do Crashlytics.
//!
//! Formato legível (de propósito, para depurar e ajustar):
//!     `<kind>|<TipoSimplesDaExceção>|<Classe.método>`
//!
//! - Play CRASH/NON_FATAL: `cause` (exceção mais interna) + `location` (método provável).
//! - Play ANR: primeiro frame do app no relatório de amostra (ou a activity, se não houver).
//! - Crashlytics: tipo da exceção mais interna + `blame_frame.symbol`.
//!
//! É uma heurística: se seu app tiver colisões ou separações erradas, este é o
//! arquivo a mudar (há testes abaixo).

/// Pacotes de framework ignorados ao procurar o "primeiro frame do app".
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

/// Normaliza um símbolo de stack para `Classe.método`, removendo pacote,
/// argumentos, arquivo/linha e sufixos gerados pelo compilador (lambdas,
/// classes anônimas, sintéticos do R8/D8).
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

/// Primeiro frame do app num relatório textual (stack Java ou dump de ANR).
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
        "anr" => format!("ANR em {loc}"),
        _ if loc.is_empty() => simple_type(exception_type).to_string(),
        _ => format!("{} em {loc}", simple_type(exception_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_crash_from_both_sources_matches() {
        // Play: cause = tipo mais interno, location = método provável.
        let play = signature(
            "crash",
            "IllegalStateException",
            "com.acme.ui.CheckoutActivity.onCreate",
        );
        // Crashlytics: tipo totalmente qualificado + blame_frame.symbol.
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
