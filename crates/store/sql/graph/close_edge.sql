WITH edge AS (
    SELECT id FROM relationships
     WHERE project_id = $3 AND from_topic = $4 AND to_topic = $5 AND kind = $6
       FOR UPDATE
)
UPDATE relationship_versions
   SET invalidated_at = $1, tombstone_reason = $2
 WHERE invalidated_at IS NULL
   AND relationship_id IN (SELECT id FROM edge)
