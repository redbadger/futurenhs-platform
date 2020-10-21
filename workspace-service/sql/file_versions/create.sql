INSERT INTO file_versions (
    folder,
    file,
    file_title,
    file_description,
    file_name,
    file_type,
    blob_storage_path,
    created_at,
    created_by,
    version_number,
    version_label
)
VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), $8, $9, $10)
RETURNING *

