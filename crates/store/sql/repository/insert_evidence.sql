WITH version AS (
    INSERT INTO source_versions (
        id, project_id, source_id, version, content, content_hash,
        filter_decision, filter_reason, recorded_at
    )
    SELECT $1, $2, $3, COALESCE(MAX(version), 0) + 1, $4, $5, $6, $7, $8
    FROM source_versions WHERE project_id = $2 AND source_id = $3
    RETURNING id, version, recorded_at
), span AS (
    INSERT INTO source_spans (
        id, project_id, source_version_id, byte_start, byte_end,
        detected_language, language_confidence
    )
    SELECT $9, $2, version.id, 0, $10, $11, $12 FROM version
)
SELECT id, version, recorded_at FROM version
