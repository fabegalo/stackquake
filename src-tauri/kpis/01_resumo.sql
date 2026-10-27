-- name: Resumo
-- description: Taxas do Play nos últimos :days dias (média ponderada por usuários) comparadas aos limites de mau comportamento do Google Play, % de usuários sem falha do Crashlytics e situação dos problemas.
WITH v AS (
  SELECT metric_set,
         SUM(user_perceived_rate * distinct_users) / NULLIF(SUM(distinct_users), 0) AS percebida,
         SUM(rate * distinct_users) / NULLIF(SUM(distinct_users), 0) AS geral,
         AVG(distinct_users) AS usuarios
  FROM vitals_daily
  WHERE package_name = :package
    AND version_code = '*'
    AND day >= date('now', '-' || :days || ' days')
  GROUP BY metric_set
)
SELECT
  (SELECT percebida FROM v WHERE metric_set = 'crash') AS crash_percebida,
  0.0109 AS limite_crash,
  (SELECT percebida FROM v WHERE metric_set = 'anr') AS anr_percebida,
  0.0047 AS limite_anr,
  (SELECT geral FROM v WHERE metric_set = 'crash') AS crash_geral,
  (SELECT geral FROM v WHERE metric_set = 'anr') AS anr_geral,
  (SELECT CAST(usuarios AS INTEGER) FROM v WHERE metric_set = 'crash') AS usuarios_dia,
  (SELECT AVG(crash_free_users) FROM crashlytics_version_daily
    WHERE package_name = :package AND kind = 'all'
      AND day >= date('now', '-' || :days || ' days')
      AND crash_free_users IS NOT NULL) AS usuarios_sem_falha_crashlytics,
  (SELECT COUNT(*) FROM problem WHERE package_name = :package AND status IN ('open', 'investigating')) AS problemas_abertos,
  (SELECT COUNT(*) FROM problem WHERE package_name = :package AND status = 'regressed') AS regressoes
