CREATE TABLE host_capabilities (
  host_id TEXT PRIMARY KEY,
  os_name TEXT,
  kernel_version TEXT,
  arch TEXT,
  has_docker INTEGER NOT NULL DEFAULT 0,
  has_buildx INTEGER NOT NULL DEFAULT 0,
  has_buildkit_enabled INTEGER NOT NULL DEFAULT 0,
  has_tailscale INTEGER NOT NULL DEFAULT 0,
  has_k3s INTEGER NOT NULL DEFAULT 0,
  has_nvidia INTEGER NOT NULL DEFAULT 0,
  docker_version TEXT,
  tailscale_ip TEXT,
  last_checked_at TEXT,
  FOREIGN KEY (host_id) REFERENCES hosts(id) ON DELETE CASCADE
);
