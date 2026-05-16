-- pg-schema-disable-next-line required-table-column-comments
CREATE TABLE legacy_table (
  id UUID PRIMARY KEY,
  name TEXT NOT NULL,
  created_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ
);
