CREATE TABLE bootstrap_runs (
  id TEXT PRIMARY KEY,
  host_id TEXT NOT NULL,
  action_name TEXT NOT NULL,
  status TEXT NOT NULL,
  output TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  FOREIGN KEY (host_id) REFERENCES hosts(id) ON DELETE CASCADE
);
