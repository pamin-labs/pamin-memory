SELECT id, relationship_id, version, valid_from, valid_to, created_at,
       invalidated_at, supersedes, caused_by_topic_state,
       confidence, derivation, tombstone_reason
FROM relationship_versions
WHERE relationship_id = $1 AND invalidated_at IS NULL
ORDER BY version DESC LIMIT 1
