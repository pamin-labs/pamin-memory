SELECT v.id, v.project_id, v.source_id, v.version, v.content, v.content_hash,
       v.filter_decision, v.filter_reason, v.recorded_at,
       s.locator, m.match_position
FROM source_versions v
JOIN sources s ON s.id = v.source_id
CROSS JOIN LATERAL (SELECT position($2 IN v.content) AS match_position) m
WHERE v.project_id = $1 AND m.match_position > 0
ORDER BY v.recorded_at DESC, v.id
LIMIT $3
