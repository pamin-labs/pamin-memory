SELECT id, created_at FROM relationships
WHERE project_id = $1 AND from_topic = $2
  AND to_topic = $3 AND kind = $4
