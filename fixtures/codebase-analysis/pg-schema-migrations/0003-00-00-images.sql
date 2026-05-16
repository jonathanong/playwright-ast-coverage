CREATE TABLE IF NOT EXISTS images (
  id UUID PRIMARY KEY DEFAULT uuidv7(),

  created_by_id UUID NOT NULL REFERENCES users ON DELETE CASCADE,
  created_at TIMESTAMPTZ GENERATED ALWAYS AS (uuid_extract_timestamp(id)) VIRTUAL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  deleted_at TIMESTAMPTZ,
  deleted_by_id UUID REFERENCES users ON DELETE SET NULL,

  -- image data
  data JSONB NOT NULL, -- raw sharp metadata,
  sha_256 BYTEA NOT NULL CHECK (octet_length(sha_256) = 32) UNIQUE, -- sha 256 sum of the image data

  -- upload lifecycle tracking
  s3_key TEXT NOT NULL,
-- pg-schema-disable-next-line text-check-instead-of-enum
  upload_status TEXT NOT NULL DEFAULT 'complete'
    CHECK (upload_status IN ('pending', 'processing', 'complete', 'failed')),
  upload_completed_at TIMESTAMPTZ,
  upload_error TEXT,

  openai_omni_moderation_results JSONB,
  openai_omni_moderation_flagged BOOLEAN,
  openai_omni_moderation_created_at TIMESTAMPTZ
);

CREATE OR REPLACE TRIGGER trigger_images_updated_at
BEFORE UPDATE ON images
FOR EACH ROW
EXECUTE FUNCTION fn_update_updated_at();

-- Index for cleanup job to find pending uploads (using id instead of created_at since created_at is virtual)
CREATE INDEX IF NOT EXISTS idx_images__upload_status_id
ON images (upload_status, id)
WHERE upload_status IN ('pending', 'processing');

-- Add profile image reference to users (FK to images defined in this file)
ALTER TABLE users
ADD COLUMN IF NOT EXISTS profile_image_id UUID REFERENCES images(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_users__profile_image_id
ON users (profile_image_id)
WHERE profile_image_id IS NOT NULL;

COMMENT ON COLUMN users.profile_image_id IS 'The user''s profile image. NULL if no profile image is set.';

COMMENT ON TABLE images IS 'Uploaded images with S3 storage, metadata, and moderation results.';
COMMENT ON COLUMN images.data IS 'Raw sharp (image processing library) metadata as JSONB.';
COMMENT ON COLUMN images.sha_256 IS 'SHA-256 hash of the original image file bytes. Unique, used for deduplication.';
COMMENT ON COLUMN images.s3_key IS 'S3 object key where the image is stored.';
COMMENT ON COLUMN images.upload_status IS 'Upload lifecycle state: pending, processing, complete, or failed.';
COMMENT ON COLUMN images.upload_completed_at IS 'When the upload processing finished successfully.';
COMMENT ON COLUMN images.upload_error IS 'Error message if upload processing failed.';
COMMENT ON COLUMN images.openai_omni_moderation_results IS 'Raw JSONB results from OpenAI omni moderation API.';
COMMENT ON COLUMN images.openai_omni_moderation_flagged IS 'Whether OpenAI moderation flagged this image.';
COMMENT ON COLUMN images.openai_omni_moderation_created_at IS 'When the moderation check was performed.';
