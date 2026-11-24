INSERT INTO index_jobs
    (id, project_id, job_type, subject, available_at, created_at, priority)
SELECT job.id, $1, job.label, $2, $3, $3, job.priority
  FROM unnest($4::uuid[], $5::text[], $6::int[])
    AS job(id, label, priority)
ON CONFLICT (project_id, job_type, subject) DO UPDATE
    SET available_at = $3,
        claimed_at   = NULL,
        claimed_by   = NULL,
        last_error   = NULL,
        attempts     = 0
