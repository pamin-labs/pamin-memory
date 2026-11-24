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
), state AS (
    INSERT INTO topic_states (
        id, project_id, topic_id, version, source_span_id,
        observed_at, recorded_at, supersedes, valid_from, valid_to
    )
    SELECT $13, $2, $14, COALESCE(MAX(version), 0) + 1,
           $9, $15, $8, $16, $17, $18
    FROM topic_states
    WHERE project_id = $2 AND topic_id = $14
    RETURNING id, version, recorded_at
), pointer AS (
    UPDATE topics SET current_state_id = state.id, current_version = state.version
    FROM state WHERE topics.id = $14
), owed AS (
    INSERT INTO index_jobs
        (id, project_id, job_type, subject, available_at, created_at, priority)
    SELECT job.id, $2, job.label, $14, $8, $8, job.priority
    FROM unnest($19::uuid[], $20::text[], $21::int[]) AS job(id, label, priority)
    ON CONFLICT (project_id, job_type, subject) DO UPDATE
        SET available_at = $8,
            claimed_at   = NULL,
            claimed_by   = NULL,
            last_error   = NULL,
            attempts     = 0
)
SELECT version.id, version.version, version.recorded_at,
       state.id AS state_id, state.version AS state_version
FROM version, state
