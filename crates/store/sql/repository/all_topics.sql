SELECT id, name, path, created_at FROM topics
WHERE project_id = $1 ORDER BY name ASC
