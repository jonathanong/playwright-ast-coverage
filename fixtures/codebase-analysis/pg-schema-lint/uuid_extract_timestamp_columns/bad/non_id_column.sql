CREATE TABLE posts (
  id UUID PRIMARY KEY DEFAULT uuidv7(),
  parent_id UUID REFERENCES posts(id),
  created_at TIMESTAMPTZ GENERATED ALWAYS AS (uuid_extract_timestamp(id)) VIRTUAL,
  parent_created_at TIMESTAMPTZ GENERATED ALWAYS AS (uuid_extract_timestamp(parent_id)) VIRTUAL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
