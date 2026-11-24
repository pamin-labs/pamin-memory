SELECT DISTINCT ON (relationship_id) id, relationship_id, version, valid_from, valid_to, created_at,
       invalidated_at, supersedes, caused_by_topic_state,
       confidence, derivation, tombstone_reason
FROM relationship_versions
WHERE relationship_id = ANY($1) AND invalidated_at IS NULL
ORDER BY relationship_id, version DESC
