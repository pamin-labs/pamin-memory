DELETE FROM index_jobs
    USING unnest($1::uuid[], $3::timestamptz[]) AS held (id, claimed_at)
    WHERE index_jobs.id = held.id
      AND index_jobs.claimed_by = $2
      AND index_jobs.claimed_at = held.claimed_at
RETURNING index_jobs.id
