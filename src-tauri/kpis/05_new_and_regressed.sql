-- name: New in latest version and regressions
-- name.pt-BR: Novos na última versão e regressões
-- description: Problems first seen in the most recent version, and resolved problems that came back.
-- description.pt-BR: Problemas que apareceram pela primeira vez na versão mais recente e problemas resolvidos que voltaram.
WITH latest AS (
  SELECT MAX(v) AS v FROM (
    SELECT MAX(CAST(version_code AS INTEGER)) AS v FROM vitals_daily
     WHERE package_name = :package AND version_code <> '*'
    UNION ALL
    SELECT MAX(CAST(build_version AS INTEGER)) FROM crashlytics_version_daily WHERE package_name = :package
  )
),
firsts AS (
  SELECT signature, MIN(v) AS first_v FROM (
    SELECT signature, CAST(first_version AS INTEGER) AS v FROM play_issue
     WHERE package_name = :package AND first_version IS NOT NULL
    UNION ALL
    SELECT signature, CAST(first_version AS INTEGER) FROM crashlytics_issue
     WHERE package_name = :package AND first_version IS NOT NULL
  ) GROUP BY signature
)
SELECT 'new' AS reason, p.kind, p.title AS problem, p.status, f.first_v AS version_code
FROM problem p
JOIN firsts f ON f.signature = p.signature
JOIN latest l ON f.first_v = l.v
WHERE p.package_name = :package
UNION ALL
SELECT 'regressed', p.kind, p.title, p.status, p.resolved_in_version
FROM problem p
WHERE p.package_name = :package AND p.status = 'regressed'
