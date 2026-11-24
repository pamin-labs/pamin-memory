SELECT count(*) FROM sources s
 JOIN topics t ON t.project_id = s.project_id AND t.name = substr(s.locator, 6)
WHERE s.project_id = $1 AND s.kind = 'file'
  AND (s.locator COLLATE "C") >= $2 AND (s.locator COLLATE "C") < $3
  AND t.current_state_id IS NOT NULL
