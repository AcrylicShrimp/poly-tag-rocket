-- Your SQL goes here

CREATE TABLE tags (
  name TEXT NOT NULL,
  file_id UUID NOT NULL,
  PRIMARY KEY (name, file_id),
  CONSTRAINT tags_file_fk FOREIGN KEY (file_id) REFERENCES files(id) ON UPDATE CASCADE ON DELETE CASCADE
);
