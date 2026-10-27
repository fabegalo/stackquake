-- name: Top problemas por impacto
-- description: Os 20 problemas com mais usuários afetados (estimativa sem duplicar fontes: o maior entre usuários do Play e do Crashlytics), ignorando os marcados como "ignored".
WITH pl AS (
  SELECT signature, SUM(distinct_users) AS usuarios
  FROM play_issue WHERE package_name = :package GROUP BY signature
),
cl AS (
  SELECT c.signature, SUM(d.users) AS usuarios, SUM(d.events) AS eventos
  FROM crashlytics_issue c
  JOIN crashlytics_issue_daily d ON d.package_name = c.package_name AND d.issue_id = c.issue_id
  WHERE c.package_name = :package AND d.day >= date('now', '-' || :days || ' days')
  GROUP BY c.signature
)
SELECT p.kind AS tipo,
       p.title AS problema,
       p.status,
       p.assignee AS responsavel,
       pl.usuarios AS usuarios_play,
       cl.usuarios AS usuarios_crashlytics,
       cl.eventos AS eventos_crashlytics,
       MAX(COALESCE(pl.usuarios, 0), COALESCE(cl.usuarios, 0)) AS afetados_estimado
FROM problem p
LEFT JOIN pl ON pl.signature = p.signature
LEFT JOIN cl ON cl.signature = p.signature
WHERE p.package_name = :package AND p.status <> 'ignored'
ORDER BY afetados_estimado DESC
LIMIT 20
