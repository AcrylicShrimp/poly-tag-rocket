-- Your SQL goes here

CREATE TABLE collection_file_pairs (
  collection_id UUID NOT NULL REFERENCES collections(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  file_id UUID NOT NULL REFERENCES files(id) ON UPDATE CASCADE ON DELETE CASCADE,
  PRIMARY KEY (collection_id, file_id)
);
