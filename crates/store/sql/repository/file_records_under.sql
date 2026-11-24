SELECT root, file, size, modified_ns, changed_ns, inode, read_ns, content_hash, sections,
      build
 FROM file_records
WHERE project_id = $1 AND root = $2 AND starts_with(file, $3)
