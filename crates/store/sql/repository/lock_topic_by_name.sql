SELECT id, name, path, created_at, current_state_id FROM topics
WHERE project_id = $1 AND name = $2
FOR UPDATE
