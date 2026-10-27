-- Unified problem list.
-- Impact = estimated affected users WITHOUT adding the sources together (the
-- same failure shows up in Play and Crashlytics): the larger of Play users and
-- Crashlytics affected users, with a lower weight for non-fatals.
WITH pl AS (
  SELECT signature,
         SUM(report_count) AS reports,
         SUM(distinct_users) AS users,
         MIN(CAST(first_version AS INTEGER)) AS first_v,
         MAX(CAST(last_version AS INTEGER)) AS last_v,
         MAX(substr(last_seen, 1, 10)) AS last_seen
  FROM play_issue
  WHERE package_name = :package
  GROUP BY signature
),
cl AS (
  SELECT c.signature,
         SUM(d.events) AS events,
         SUM(d.users) AS users,
         MIN(CAST(c.first_version AS INTEGER)) AS first_v,
         MAX(CAST(c.last_version AS INTEGER)) AS last_v,
         MAX(d.day) AS last_seen
  FROM crashlytics_issue c
  JOIN crashlytics_issue_daily d ON d.package_name = c.package_name AND d.issue_id = c.issue_id
  WHERE c.package_name = :package
    AND d.day >= date('now', '-' || :days || ' days')
  GROUP BY c.signature
)
SELECT p.id, p.kind, p.title, p.status, p.assignee,
       CASE
         WHEN pl.signature IS NOT NULL AND cl.signature IS NOT NULL THEN 'both'
         WHEN pl.signature IS NOT NULL THEN 'play'
         WHEN cl.signature IS NOT NULL THEN 'crashlytics'
         ELSE 'none'
       END AS sources,
       pl.reports AS play_reports,
       pl.users AS play_users,
       cl.events AS cl_events,
       cl.users AS cl_users,
       MIN(COALESCE(pl.first_v, cl.first_v), COALESCE(cl.first_v, pl.first_v)) AS first_version,
       MAX(COALESCE(pl.last_v, cl.last_v), COALESCE(cl.last_v, pl.last_v)) AS last_version,
       NULLIF(MAX(COALESCE(pl.last_seen, ''), COALESCE(cl.last_seen, '')), '') AS last_seen,
       CAST(MAX(COALESCE(pl.users, 0), COALESCE(cl.users, 0)) AS REAL)
         * CASE p.kind WHEN 'nonfatal' THEN 0.3 ELSE 1.0 END AS impact
FROM problem p
LEFT JOIN pl ON pl.signature = p.signature
LEFT JOIN cl ON cl.signature = p.signature
WHERE p.package_name = :package
  AND (:status IS NULL OR p.status = :status)
  AND (:kind IS NULL OR p.kind = :kind)
  AND (pl.signature IS NOT NULL OR cl.signature IS NOT NULL
       OR p.status IN ('investigating', 'regressed'))
ORDER BY impact DESC, p.id
LIMIT 1000
