-- name: Version health
-- name.pt-BR: Saúde por versão
-- description: Play perceived rates per versionCode plus Crashlytics crash-free users, crashes and ANRs for the same version over the last :days days. "over_threshold" = worse than Google Play's bad-behavior threshold.
-- description.pt-BR: Taxas percebidas do Play por versionCode e, do Crashlytics, usuários sem falha, falhas e ANRs na mesma versão, nos últimos :days dias. "Acima do limite" = pior que o limite de mau comportamento do Play.
WITH v AS (
  SELECT version_code,
         SUM(CASE WHEN metric_set = 'crash' THEN user_perceived_rate * distinct_users END)
           / NULLIF(SUM(CASE WHEN metric_set = 'crash' THEN distinct_users END), 0) AS perceived_crash_rate,
         SUM(CASE WHEN metric_set = 'anr' THEN user_perceived_rate * distinct_users END)
           / NULLIF(SUM(CASE WHEN metric_set = 'anr' THEN distinct_users END), 0) AS perceived_anr_rate,
         MAX(CASE WHEN metric_set = 'crash' THEN distinct_users END) AS peak_daily_users
  FROM vitals_daily
  WHERE package_name = :package
    AND version_code <> '*'
    AND day >= date('now', '-' || :days || ' days')
  GROUP BY version_code
),
c AS (
  SELECT build_version AS version_code,
         MAX(display_version) AS version_name,
         AVG(CASE WHEN kind = 'all' THEN crash_free_users END) AS crash_free_users,
         SUM(CASE WHEN kind = 'crash' THEN events ELSE 0 END) AS crashlytics_crashes,
         SUM(CASE WHEN kind = 'anr' THEN events ELSE 0 END) AS crashlytics_anrs
  FROM crashlytics_version_daily
  WHERE package_name = :package
    AND day >= date('now', '-' || :days || ' days')
  GROUP BY build_version
)
SELECT COALESCE(v.version_code, c.version_code) AS version_code,
       c.version_name,
       v.perceived_crash_rate,
       v.perceived_anr_rate,
       c.crash_free_users,
       v.peak_daily_users,
       c.crashlytics_crashes,
       c.crashlytics_anrs,
       CASE
         WHEN v.version_code IS NULL THEN 'no_play_data'
         WHEN v.perceived_crash_rate >= 0.0109 OR v.perceived_anr_rate >= 0.0047 THEN 'over_threshold'
         ELSE 'ok'
       END AS health
FROM v
FULL JOIN c ON c.version_code = v.version_code
ORDER BY CAST(COALESCE(v.version_code, c.version_code) AS INTEGER) DESC
LIMIT 30
