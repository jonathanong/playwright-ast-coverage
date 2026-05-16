CREATE TABLE posts (
  id UUID PRIMARY KEY DEFAULT uuidv7(),
  author_uuid UUID,
  created_at TIMESTAMPTZ GENERATED ALWAYS AS (uuid_extract_timestamp(id)) VIRTUAL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
