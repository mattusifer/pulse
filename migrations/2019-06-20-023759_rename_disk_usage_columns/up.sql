DROP TABLE disk_usage;
CREATE TABLE disk_usage (
  id SERIAL PRIMARY KEY,
  mount VARCHAR NOT NULL,
  percent_disk_used DOUBLE PRECISION NOT NULL,
  recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
