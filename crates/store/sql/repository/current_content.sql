SELECT sp.byte_start, sp.byte_end, sv.content AS evidence
 FROM topics t
 JOIN topic_states ts ON ts.id = t.current_state_id
JOIN source_spans sp ON sp.id = ts.source_span_id
JOIN source_versions sv ON sv.id = sp.source_version_id
WHERE t.project_id = $1 AND t.name = $2
