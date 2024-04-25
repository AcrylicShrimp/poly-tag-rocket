-- Your SQL goes here

CREATE TABLE collection_file_pairs (
  collection_id UUID NOT NULL,
  file_id UUID NOT NULL,
  PRIMARY KEY (collection_id, file_id),
  CONSTRAINT collection_file_pairs_collection_fk FOREIGN KEY (collection_id) REFERENCES collections(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  CONSTRAINT collection_file_pairs_file_fk FOREIGN KEY (file_id) REFERENCES files(id) ON UPDATE CASCADE ON DELETE CASCADE
);
