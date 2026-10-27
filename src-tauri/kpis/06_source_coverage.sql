-- name: Source coverage
-- name.pt-BR: Cobertura das fontes
-- description: How many problems appear only in Play, only in Crashlytics, or in both. Many single-source cases may mean a missing R8 mapping in one source or a signature to tune in src-tauri/src/unify.rs.
-- description.pt-BR: Quantos problemas aparecem só no Play, só no Crashlytics ou nos dois. Muitos casos de uma fonte só podem indicar mapping do R8 ausente numa delas ou uma assinatura a ajustar em src-tauri/src/unify.rs.
WITH s AS (
  SELECT p.id,
         EXISTS (SELECT 1 FROM play_issue x
                  WHERE x.package_name = p.package_name AND x.signature = p.signature) AS in_play,
         EXISTS (SELECT 1 FROM crashlytics_issue y
                  WHERE y.package_name = p.package_name AND y.signature = p.signature) AS in_crashlytics
  FROM problem p
  WHERE p.package_name = :package
)
SELECT CASE
         WHEN in_play AND in_crashlytics THEN 'both'
         WHEN in_play THEN 'play_only'
         WHEN in_crashlytics THEN 'crashlytics_only'
         ELSE 'none'
       END AS source,
       COUNT(*) AS problems
FROM s
GROUP BY 1
ORDER BY 2 DESC
