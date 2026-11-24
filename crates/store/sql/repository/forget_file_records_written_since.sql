DELETE FROM file_records f
USING unnest($2::text[], $3::text[], $4::text[]) AS k (root, file, topic)
 JOIN sources s ON s.project_id = $1 AND s.locator = 'manual:' || k.topic
 JOIN source_versions v
   ON v.project_id = $1 AND v.source_id = s.id AND v.recorded_at >= $5
WHERE f.project_id = $1 AND f.root = k.root AND f.file = k.file
