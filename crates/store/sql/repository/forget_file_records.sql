DELETE FROM file_records f
USING unnest($2::text[], $3::text[]) AS k (root, file)
WHERE f.project_id = $1 AND f.root = k.root AND f.file = k.file
