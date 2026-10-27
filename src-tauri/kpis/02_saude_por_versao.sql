-- name: Saúde por versão
-- description: Taxas percebidas do Play por versionCode e, do Crashlytics, % de usuários sem falha, falhas e ANRs na mesma versão, nos últimos :days dias. "acima do limite" = pior que o limite de mau comportamento do Play.
WITH v AS (
  SELECT version_code,
         SUM(CASE WHEN metric_set = 'crash' THEN user_perceived_rate * distinct_users END)
           / NULLIF(SUM(CASE WHEN metric_set = 'crash' THEN distinct_users END), 0) AS crash_percebida,
         SUM(CASE WHEN metric_set = 'anr' THEN user_perceived_rate * distinct_users END)
           / NULLIF(SUM(CASE WHEN metric_set = 'anr' THEN distinct_users END), 0) AS anr_percebida,
         MAX(CASE WHEN metric_set = 'crash' THEN distinct_users END) AS pico_usuarios_dia
  FROM vitals_daily
  WHERE package_name = :package
    AND version_code <> '*'
    AND day >= date('now', '-' || :days || ' days')
  GROUP BY version_code
),
c AS (
  SELECT build_version AS version_code,
         MAX(display_version) AS versao,
         AVG(CASE WHEN kind = 'all' THEN crash_free_users END) AS usuarios_sem_falha,
         SUM(CASE WHEN kind = 'crash' THEN events ELSE 0 END) AS crashes_crashlytics,
         SUM(CASE WHEN kind = 'anr' THEN events ELSE 0 END) AS anrs_crashlytics
  FROM crashlytics_version_daily
  WHERE package_name = :package
    AND day >= date('now', '-' || :days || ' days')
  GROUP BY build_version
)
SELECT COALESCE(v.version_code, c.version_code) AS version_code,
       c.versao,
       v.crash_percebida,
       v.anr_percebida,
       c.usuarios_sem_falha,
       v.pico_usuarios_dia,
       c.crashes_crashlytics,
       c.anrs_crashlytics,
       CASE
         WHEN v.version_code IS NULL THEN 'sem dados do Play'
         WHEN v.crash_percebida >= 0.0109 OR v.anr_percebida >= 0.0047 THEN 'acima do limite'
         ELSE 'ok'
       END AS situacao
FROM v
FULL JOIN c ON c.version_code = v.version_code
ORDER BY CAST(COALESCE(v.version_code, c.version_code) AS INTEGER) DESC
LIMIT 30
