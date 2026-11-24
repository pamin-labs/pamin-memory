UPDATE topic_states SET deleted_at = $3
WHERE topic_id = $1 AND version = $2 AND deleted_at IS NULL
