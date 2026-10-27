-- name: Tendência diária
-- description: Taxas percebidas do Play e falhas/ANRs do Crashlytics por dia. Atenção: o Play agrega no fuso America/Los_Angeles e o Crashlytics em UTC.
SELECT vd.day,
       MAX(CASE WHEN vd.metric_set = 'crash' THEN vd.user_perceived_rate END) AS crash_percebida,
       MAX(CASE WHEN vd.metric_set = 'anr' THEN vd.user_perceived_rate END) AS anr_percebida,
       MAX(CASE WHEN vd.metric_set = 'crash' THEN vd.distinct_users END) AS usuarios,
       (SELECT SUM(d.events) FROM crashlytics_version_daily d
         WHERE d.package_name = :package AND d.day = vd.day AND d.kind IN ('crash', 'anr')) AS eventos_crashlytics
FROM vitals_daily vd
WHERE vd.package_name = :package
  AND vd.version_code = '*'
  AND vd.day >= date('now', '-' || :days || ' days')
GROUP BY vd.day
ORDER BY vd.day
