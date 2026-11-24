SELECT p.topic AS standing, TRUE AS outward,
       e.from_topic, e.to_topic, e.kind, e.confidence, e.derivation
  FROM unnest($2::uuid[]) AS p (topic)
 CROSS JOIN LATERAL (
       SELECT v.from_topic, v.to_topic, v.kind, v.confidence, v.derivation
         FROM relationship_versions v
        WHERE v.project_id = $1 AND v.from_topic = p.topic
          AND v.invalidated_at IS NULL
          AND ($3::TEXT[] IS NULL OR v.kind = ANY ($3))
        ORDER BY v.confidence DESC, v.to_topic
        LIMIT $4) e
UNION ALL
SELECT p.topic, FALSE,
       e.from_topic, e.to_topic, e.kind, e.confidence, e.derivation
  FROM unnest($2::uuid[]) AS p (topic)
 CROSS JOIN LATERAL (
       SELECT v.from_topic, v.to_topic, v.kind, v.confidence, v.derivation
         FROM relationship_versions v
        WHERE v.project_id = $1 AND v.to_topic = p.topic
          AND v.invalidated_at IS NULL
          AND ($3::TEXT[] IS NULL OR v.kind = ANY ($3))
        ORDER BY v.confidence DESC, v.from_topic
        LIMIT $4) e
