WITH gone AS (DELETE FROM projects WHERE id = $1 RETURNING id)
SELECT gone.id,
       (SELECT count(*) FROM topics WHERE project_id = gone.id),
       (SELECT count(*) FROM sources WHERE project_id = gone.id),
       ARRAY(SELECT path FROM synced_paths
              WHERE project_id = gone.id ORDER BY path)
  FROM gone
