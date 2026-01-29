-- 020: images の image_data (BLOB) を file_name (TEXT) に変更 (issue #47)
ALTER TABLE images ADD COLUMN file_name TEXT;
ALTER TABLE images DROP COLUMN image_data;
