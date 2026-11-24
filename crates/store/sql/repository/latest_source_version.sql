SELECT id, project_id, source_id, version, content, content_hash,
       filter_decision, filter_reason, recorded_at
FROM source_versions WHERE project_id = $1 AND source_id = $2
ORDER BY version DESC LIMIT 1
