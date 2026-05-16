-- pg-schema: no-timestamps
CREATE TABLE log_entries (
  id UUID PRIMARY KEY DEFAULT uuidv7(),
  message TEXT NOT NULL
);
