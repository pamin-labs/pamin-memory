SELECT DISTINCT t.name, t.current_state_id, s.root
 FROM unnest($2::text[], $3::text[]) AS r (low, high)
 JOIN sources s
   ON s.project_id = $1 AND s.kind = 'file'
  AND (s.locator COLLATE "C") >= r.low AND (s.locator COLLATE "C") < r.high
 JOIN topics t ON t.project_id = s.project_id AND t.name = substr(s.locator, 6)
WHERE t.current_state_id IS NOT NULL
