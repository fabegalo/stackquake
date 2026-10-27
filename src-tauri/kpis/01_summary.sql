-- name: Summary
-- name.pt-BR: Resumo
-- description: Play rates over the last :days days (weighted by users) against Google Play's bad-behavior thresholds, Crashlytics crash-free users and problem status.
-- description.pt-BR: Taxas do Play nos últimos :days dias (ponderadas por usuários) comparadas aos limites de mau comportamento do Google Play, usuários sem falha do Crashlytics e situação dos problemas.
WITH v AS (
  SELECT metric_set,
         SUM(user_perceived_rate * distinct_users) / NULLIF(SUM(distinct_users), 0) AS perceived,
         SUM(rate * distinct_users) / NULLIF(SUM(distinct_users), 0) AS overall,
         AVG(distinct_users) AS users
  FROM vitals_daily
  WHERE package_name = :package
    AND version_code = '*'
    AND day >= date('now', '-' || :days || ' days')
  GROUP BY metric_set
)
SELECT
  (SELECT perceived FROM v WHERE metric_set = 'crash') AS perceived_crash_rate,
  0.0109 AS crash_threshold,
  (SELECT perceived FROM v WHERE metric_set = 'anr') AS perceived_anr_rate,
  0.0047 AS anr_threshold,
  (SELECT overall FROM v WHERE metric_set = 'crash') AS overall_crash_rate,
  (SELECT overall FROM v WHERE metric_set = 'anr') AS overall_anr_rate,
  (SELECT CAST(users AS INTEGER) FROM v WHERE metric_set = 'crash') AS daily_users,
  (SELECT AVG(crash_free_users) FROM crashlytics_version_daily
    WHERE package_name = :package AND kind = 'all'
      AND day >= date('now', '-' || :days || ' days')
      AND crash_free_users IS NOT NULL) AS crash_free_users,
  (SELECT COUNT(*) FROM problem WHERE package_name = :package AND status IN ('open', 'investigating')) AS open_problems,
  (SELECT COUNT(*) FROM problem WHERE package_name = :package AND status = 'regressed') AS regressions
