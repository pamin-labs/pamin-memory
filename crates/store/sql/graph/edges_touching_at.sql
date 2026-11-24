SELECT r.from_topic, r.to_topic, r.kind, v.confidence, v.derivation
FROM relationships r
JOIN relationship_versions v
  ON v.project_id = r.project_id AND v.relationship_id = r.id
WHERE r.project_id = $1
  AND (r.from_topic = ANY($2) OR r.to_topic = ANY($2))
  AND ($3::TEXT[] IS NULL OR r.kind = ANY ($3))
  AND (v.valid_from IS NULL OR v.valid_from <= $4)
 AND (v.valid_to IS NULL OR $4 < v.valid_to)
 AND (v.invalidated_at IS NULL
      OR (v.tombstone_reason = 'closed' AND $4 < v.invalidated_at))
