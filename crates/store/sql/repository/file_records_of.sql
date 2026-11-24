SELECT f.root, f.file, f.size, f.modified_ns, f.changed_ns, f.inode, f.read_ns,
     f.content_hash, f.sections, f.build
FROM unnest($2::text[], $3::text[]) AS k (root, file)
JOIN file_records f ON f.project_id = $1 AND f.root = k.root AND f.file = k.file
