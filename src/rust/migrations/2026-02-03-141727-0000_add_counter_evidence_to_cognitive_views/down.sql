-- Rollback: Remove counter_evidence fields from cognitive_views

DROP INDEX IF EXISTS idx_cognitive_views_counter_evidence_ratio;

ALTER TABLE cognitive_views
DROP COLUMN IF EXISTS counter_evidence_count;

ALTER TABLE cognitive_views
DROP COLUMN IF EXISTS counter_evidence;
