SELECT id, name, current_state_id FROM topics
WHERE project_id = $1 AND id = ANY($2)
