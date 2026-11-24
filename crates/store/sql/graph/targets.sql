SELECT r.from_topic, r.to_topic FROM relationships r
WHERE r.project_id = $1 AND r.from_topic = ANY($2) AND r.kind = $3
  AND EXISTS (
      SELECT 1 FROM relationship_versions v
       WHERE v.relationship_id = r.id AND v.invalidated_at IS NULL
  )
