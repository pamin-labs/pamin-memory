SELECT t.id, t.name, t.path, t.created_at FROM topics t
WHERE t.project_id = $1
  AND (NOT EXISTS (
           SELECT 1 FROM sources s
            WHERE s.project_id = t.project_id
              AND s.locator = 'file:' || t.name)
       OR EXISTS (
           SELECT 1 FROM sources s
            WHERE s.project_id = t.project_id
              AND s.locator = 'manual:' || t.name))
ORDER BY t.name ASC
