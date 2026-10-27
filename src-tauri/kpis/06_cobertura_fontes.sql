-- name: Cobertura das fontes
-- description: Quantos problemas aparecem só no Play, só no Crashlytics ou nos dois. Muitos casos "só em uma fonte" podem indicar mapping do R8 ausente numa delas ou uma assinatura a ajustar em src-tauri/src/unify.rs.
WITH s AS (
  SELECT p.id,
         EXISTS (SELECT 1 FROM play_issue x
                  WHERE x.package_name = p.package_name AND x.signature = p.signature) AS em_play,
         EXISTS (SELECT 1 FROM crashlytics_issue y
                  WHERE y.package_name = p.package_name AND y.signature = p.signature) AS em_crashlytics
  FROM problem p
  WHERE p.package_name = :package
)
SELECT CASE
         WHEN em_play AND em_crashlytics THEN 'nas duas fontes'
         WHEN em_play THEN 'só Play'
         WHEN em_crashlytics THEN 'só Crashlytics'
         ELSE 'nenhuma (histórico)'
       END AS fonte,
       COUNT(*) AS problemas
FROM s
GROUP BY 1
ORDER BY 2 DESC
