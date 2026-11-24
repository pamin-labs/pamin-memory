SELECT name_key, topic_id FROM topic_name_tokens
WHERE project_id = $1 AND name_key = ANY($2)
