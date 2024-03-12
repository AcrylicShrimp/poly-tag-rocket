-- Your SQL goes here

CREATE TABLE staging_files (
  id UUID NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
  name TEXT NOT NULL,
  mime TEXT NULL,
  size BIGINT NOT NULL,
  staged_at TIMESTAMP NOT NULL DEFAULT NOW()
);
