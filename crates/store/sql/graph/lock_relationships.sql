SELECT r.id, r.from_topic, r.to_topic, r.kind
 FROM relationships r
 JOIN unnest($2::uuid[], $3::uuid[], $4::text[]) AS wanted(from_topic, to_topic, kind)
   ON r.project_id = $1 AND r.from_topic = wanted.from_topic
  AND r.to_topic = wanted.to_topic AND r.kind = wanted.kind
ORDER BY r.from_topic, r.to_topic, r.kind
  FOR UPDATE OF r
