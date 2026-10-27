-- name: Top problems by impact
-- name.pt-BR: Top problemas por impacto
-- description: The 20 problems affecting the most users (estimated without double counting sources: the larger of Play users and Crashlytics users), excluding ignored ones.
-- description.pt-BR: Os 20 problemas com mais usuários afetados (estimativa sem duplicar fontes: o maior entre usuários do Play e do Crashlytics), ignorando os marcados como ignorados.
WITH pl AS (
  SELECT signature, SUM(distinct_users) AS users
  FROM play_issue WHERE package_name = :package GROUP BY signature
),
cl AS (
  SELECT c.signature, SUM(d.users) AS users, SUM(d.events) AS events
  FROM crashlytics_issue c
  JOIN crashlytics_issue_daily d ON d.package_name = c.package_name AND d.issue_id = c.issue_id
  WHERE c.package_name = :package AND d.day >= date('now', '-' || :days || ' days')
  GROUP BY c.signature
)
SELECT p.kind,
       p.title AS problem,
       p.status,
       p.assignee,
       pl.users AS play_users,
       cl.users AS crashlytics_users,
       cl.events AS crashlytics_events,
       MAX(COALESCE(pl.users, 0), COALESCE(cl.users, 0)) AS estimated_affected
FROM problem p
LEFT JOIN pl ON pl.signature = p.signature
LEFT JOIN cl ON cl.signature = p.signature
WHERE p.package_name = :package AND p.status <> 'ignored'
ORDER BY estimated_affected DESC
LIMIT 20
