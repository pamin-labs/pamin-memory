UPDATE topics t
  SET current_state_id = latest.id,
      current_version  = latest.version
 FROM (
     SELECT topics.id AS topic_id, live.id, live.version
     FROM topics
     LEFT JOIN LATERAL (
         SELECT id, version FROM topic_states
         WHERE topic_id = topics.id AND deleted_at IS NULL
         ORDER BY version DESC LIMIT 1
     ) AS live ON TRUE
     WHERE topics.project_id = $1
 ) AS latest
WHERE latest.topic_id = t.id
  AND (t.current_state_id, t.current_version)
      IS DISTINCT FROM (latest.id, latest.version)
