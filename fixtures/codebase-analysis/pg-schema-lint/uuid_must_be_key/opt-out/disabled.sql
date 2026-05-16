CREATE TABLE posts (
  id UUID PRIMARY KEY DEFAULT uuidv7(),
-- pg-schema-disable-next-line uuid-must-be-key
  special_uuid UUID,
  created_at TIMESTAMPTZ GENERATED ALWAYS AS (uuid_extract_timestamp(id)) VIRTUAL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
