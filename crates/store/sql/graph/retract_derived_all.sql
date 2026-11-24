WITH doomed AS (
    SELECT r.id FROM relationships r
     WHERE r.project_id = $4 AND r.from_topic = ANY($5) AND r.kind = $6
       AND NOT EXISTS (
           SELECT 1 FROM unnest($7::uuid[], $8::uuid[]) AS kept (from_topic, to_topic)
            WHERE kept.from_topic = r.from_topic AND kept.to_topic = r.to_topic
       )
       AND EXISTS (
           SELECT 1 FROM relationship_versions v
            WHERE v.relationship_id = r.id AND v.invalidated_at IS NULL
              AND v.derivation = $3
       )
     ORDER BY r.from_topic, r.to_topic, r.kind
       FOR UPDATE OF r
)
UPDATE relationship_versions
   SET invalidated_at = $1, tombstone_reason = $2
 WHERE invalidated_at IS NULL
   AND derivation = $3
   AND relationship_id IN (SELECT id FROM doomed)
