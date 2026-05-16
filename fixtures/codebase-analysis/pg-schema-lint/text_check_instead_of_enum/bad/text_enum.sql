CREATE TABLE orders (
  id UUID PRIMARY KEY DEFAULT uuidv7(),
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected'))
);
