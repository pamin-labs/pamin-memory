SELECT ts.id, ts.project_id, ts.topic_id, ts.version,
       ts.source_span_id, ts.observed_at, ts.recorded_at, ts.valid_from, ts.valid_to, ts.supersedes, ts.deleted_at,
       sp.detected_language, sp.byte_start, sp.byte_end, sv.content AS evidence
FROM topic_states ts
JOIN source_spans sp ON sp.id = ts.source_span_id
JOIN source_versions sv ON sv.id = sp.source_version_id
WHERE ts.topic_id = $1 AND ts.deleted_at IS NULL
ORDER BY ts.version DESC LIMIT 1
