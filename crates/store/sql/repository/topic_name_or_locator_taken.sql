SELECT 1 FROM topics WHERE project_id = $1 AND name = $2
UNION ALL
SELECT 1 FROM sources WHERE project_id = $1 AND locator = $3
LIMIT 1
