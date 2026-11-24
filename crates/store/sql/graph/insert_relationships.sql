INSERT INTO relationships (id, project_id, from_topic, to_topic, kind, created_at)
SELECT id, $1, from_topic, to_topic, kind, $2
  FROM unnest($3::uuid[], $4::uuid[], $5::uuid[], $6::text[])
    AS wanted(id, from_topic, to_topic, kind)
 ORDER BY from_topic, to_topic, kind
ON CONFLICT (project_id, from_topic, to_topic, kind) DO NOTHING
