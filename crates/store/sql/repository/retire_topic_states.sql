UPDATE topic_states SET deleted_at = $2
WHERE topic_id = $1 AND deleted_at IS NULL
