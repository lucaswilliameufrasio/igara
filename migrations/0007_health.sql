-- Health checks
CREATE TABLE host_health (
  host_id TEXT PRIMARY KEY,
  is_up INTEGER NOT NULL,
  last_checked_at TEXT NOT NULL,
  FOREIGN KEY (host_id) REFERENCES hosts(id) ON DELETE CASCADE
);
