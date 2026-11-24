SELECT DISTINCT ON (r.id) r.from_topic, r.to_topic, r.kind,
       v.id, v.relationship_id, v.version, v.valid_from, v.valid_to, v.created_at,
       v.invalidated_at, v.supersedes, v.caused_by_topic_state, v.confidence,
       v.derivation, v.tombstone_reason
FROM unnest($2::uuid[], $3::uuid[], $4::text[]) AS wanted(from_topic, to_topic, kind)
 JOIN relationships r
   ON r.project_id = $1 AND r.from_topic = wanted.from_topic
  AND r.to_topic = wanted.to_topic AND r.kind = wanted.kind
 JOIN relationship_versions v
   ON v.relationship_id = r.id AND v.invalidated_at IS NULL
ORDER BY r.id, v.version DESC
