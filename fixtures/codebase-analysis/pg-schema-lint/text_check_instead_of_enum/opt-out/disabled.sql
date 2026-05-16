CREATE TABLE images (
-- pg-schema-disable-next-line text-check-instead-of-enum
  upload_status TEXT NOT NULL DEFAULT 'complete' CHECK (upload_status IN ('pending', 'processing', 'complete', 'failed'))
);
