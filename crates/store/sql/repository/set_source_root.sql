UPDATE sources SET root = $2 WHERE id = $1 AND root IS DISTINCT FROM $2
