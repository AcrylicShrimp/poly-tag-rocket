-- Your SQL goes here

CREATE TABLE files (
  id UUID NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
  name TEXT NOT NULL,
  mime TEXT NOT NULL,
  size BIGINT NOT NULL,
  hash BIGINT NOT NULL, -- sha256
  uploaded_at TIMESTAMP NOT NULL DEFAULT NOW()
);
