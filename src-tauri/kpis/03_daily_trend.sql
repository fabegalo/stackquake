-- name: Daily trend
-- name.pt-BR: Tendência diária
-- description: Play perceived rates and Crashlytics crashes/ANRs per day. Note: Play aggregates in America/Los_Angeles, Crashlytics in UTC.
-- description.pt-BR: Taxas percebidas do Play e falhas/ANRs do Crashlytics por dia. Atenção: o Play agrega no fuso America/Los_Angeles e o Crashlytics em UTC.
SELECT vd.day,
       MAX(CASE WHEN vd.metric_set = 'crash' THEN vd.user_perceived_rate END) AS perceived_crash_rate,
       MAX(CASE WHEN vd.metric_set = 'anr' THEN vd.user_perceived_rate END) AS perceived_anr_rate,
       MAX(CASE WHEN vd.metric_set = 'crash' THEN vd.distinct_users END) AS users,
       (SELECT SUM(d.events) FROM crashlytics_version_daily d
         WHERE d.package_name = :package AND d.day = vd.day AND d.kind IN ('crash', 'anr')) AS crashlytics_events
FROM vitals_daily vd
WHERE vd.package_name = :package
  AND vd.version_code = '*'
  AND vd.day >= date('now', '-' || :days || ' days')
GROUP BY vd.day
ORDER BY vd.day
