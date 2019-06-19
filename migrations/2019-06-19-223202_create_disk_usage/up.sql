CREATE TABLE disk_usage (
  id SERIAL PRIMARY KEY,
  mount VARCHAR NOT NULL,
  available_space BIGINT NOT NULL,
  space_used BIGINT NOT NULL,
  recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
)
