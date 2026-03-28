CREATE TABLE app_settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  default_host_id TEXT,
  last_used_host_id TEXT,
  first_run_completed INTEGER NOT NULL DEFAULT 0
);

INSERT INTO app_settings (id, default_host_id, last_used_host_id, first_run_completed)
VALUES (1, NULL, NULL, 0)
ON CONFLICT(id) DO NOTHING;
