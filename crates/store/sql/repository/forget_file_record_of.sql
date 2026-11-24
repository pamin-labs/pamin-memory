DELETE FROM file_records f
USING sources s
WHERE s.project_id = $1 AND s.locator = 'file:' || $2
  AND f.project_id = $1 AND f.root = s.root
  AND ($2 = f.file OR starts_with($2, f.file || ' › ')
       OR starts_with($2, f.file || ' #'))
