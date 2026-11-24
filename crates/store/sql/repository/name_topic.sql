INSERT INTO topic_name_tokens (project_id, topic_id, name_key, token_count)
SELECT project_id, id, $3, $4 FROM topics WHERE project_id = $1 AND name = $2
ON CONFLICT (project_id, topic_id) DO NOTHING
RETURNING topic_id
