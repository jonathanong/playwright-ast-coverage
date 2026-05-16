CREATE TABLE widgets (
  id UUID PRIMARY KEY DEFAULT uuidv7(),
  name TEXT NOT NULL,
  created_at TIMESTAMPTZ GENERATED ALWAYS AS (uuid_extract_timestamp(id)) VIRTUAL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE widgets IS 'Reusable UI widgets.';
COMMENT ON COLUMN widgets.name IS 'The widget name.';

ALTER TABLE widgets ADD COLUMN IF NOT EXISTS color TEXT;
-- Missing COMMENT ON COLUMN widgets.color
