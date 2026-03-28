PRAGMA foreign_keys = ON;

CREATE TABLE hosts (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  hostname TEXT NOT NULL,
  user_name TEXT NOT NULL,
  port INTEGER NOT NULL DEFAULT 22,
  connection_mode TEXT NOT NULL,
  platform_family TEXT,
  ssh_alias TEXT NOT NULL UNIQUE,
  ssh_key_path TEXT NOT NULL,
  docker_context_name TEXT NOT NULL UNIQUE,
  is_default INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE host_tags (
  host_id TEXT NOT NULL,
  tag TEXT NOT NULL,
  PRIMARY KEY (host_id, tag),
  FOREIGN KEY (host_id) REFERENCES hosts(id) ON DELETE CASCADE
);
