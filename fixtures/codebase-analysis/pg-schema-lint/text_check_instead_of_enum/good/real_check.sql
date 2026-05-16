CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT uuidv7(),
  username TEXT NOT NULL CHECK (char_length(username) <= 255)
);
