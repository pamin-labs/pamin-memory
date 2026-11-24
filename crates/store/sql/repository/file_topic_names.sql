SELECT t.name, t.current_state_id, s.root FROM sources s
  JOIN topics t ON t.project_id = s.project_id AND t.name = substr(s.locator, 6)
WHERE s.project_id = $1 AND s.kind = 'file' AND t.current_state_id IS NOT NULL
