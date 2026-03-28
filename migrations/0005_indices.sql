-- Optimization for tag lookups
CREATE INDEX IF NOT EXISTS host_tags_tag_idx ON host_tags(tag);
