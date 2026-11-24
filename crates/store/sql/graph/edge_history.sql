SELECT id, relationship_id, version, valid_from, valid_to, created_at,
       invalidated_at, supersedes, caused_by_topic_state,
       confidence, derivation, tombstone_reason
FROM relationship_versions
WHERE project_id = $1 AND relationship_id = $2 ORDER BY version ASC
