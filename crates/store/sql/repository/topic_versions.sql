SELECT version FROM topic_states
WHERE topic_id = $1 AND deleted_at IS NULL
ORDER BY version ASC
