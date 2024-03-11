-- Your SQL goes here

CREATE TABLE files (
  id UUID NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
  name TEXT NOT NULL,
  mime TEXT NULL,
  size BIGINT NULL,
  hash BIGINT NULL, -- sha256
  created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
