SELECT topics.id FROM topics
JOIN topic_states ts ON ts.id = topics.current_state_id
WHERE topics.project_id = $1 AND ts.deleted_at IS NULL
ORDER BY topics.id ASC
