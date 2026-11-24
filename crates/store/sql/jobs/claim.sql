UPDATE index_jobs
      SET claimed_at   = $1,
          claimed_by   = $2,
          attempts     = attempts + 1,
          available_at = $3
    WHERE id IN (
        SELECT id FROM index_jobs
         WHERE project_id = $6
           AND job_type = ANY($7)
           -- Says nothing `job_type` does not, since priority is a
           -- function of kind -- except to the index, which is ordered
           -- by project and priority and cannot see kind at all.
           -- Without it a claim for upkeep read every owed row to find
           -- none of its own.
           AND priority = ANY($8)
           AND available_at <= $1
           -- A job that has used its attempts stays pending with its
           -- error rather than coming round again. Retrying for ever
           -- turns one poisoned job into a worker that never does
           -- anything else; `cascade replay` is how it comes back.
           AND attempts < $5
         ORDER BY priority, available_at
           FOR UPDATE SKIP LOCKED
         LIMIT $4
    )
RETURNING id, project_id, job_type, subject, attempts, claimed_at
