UPDATE sources SET root = $3
WHERE project_id = $1 AND locator = ANY($2) AND root IS DISTINCT FROM $3
