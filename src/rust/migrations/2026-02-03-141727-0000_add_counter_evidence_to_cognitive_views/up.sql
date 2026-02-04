-- Add counter_evidence field to cognitive_views for Promotion Gate
--
-- This field stores events that contradict the view hypothesis,
-- allowing the system to track view reliability and detect conflicts.
--
-- # Design Principles (HEAD.md)
-- - **Promotion Gate**: Check counter-evidence ratio before promotion
-- - **Conflict Detection**: Identify contradictory hypotheses
-- # Skill Reference
-- - docs/skills/cognitive_view_generation.md

-- Add counter_evidence column (JSONB array of event IDs)
ALTER TABLE cognitive_views
ADD COLUMN counter_evidence JSONB NOT NULL DEFAULT '[]'::JSONB;

-- Add counter_evidence_count for efficient querying
ALTER TABLE cognitive_views
ADD COLUMN counter_evidence_count INTEGER NOT NULL DEFAULT 0;

-- Add index for views with high counter-evidence (for cleanup)
CREATE INDEX IF NOT EXISTS idx_cognitive_views_counter_evidence_ratio
    ON cognitive_views(user_id)
    WHERE (counter_evidence_count::FLOAT / evidence_count) > 0.15;

-- Comment for documentation
COMMENT ON COLUMN cognitive_views.counter_evidence IS 'Events that contradict this view hypothesis (JSONB array of event IDs)';
COMMENT ON COLUMN cognitive_views.counter_evidence_count IS 'Cached count of counter-evidence events for efficient querying';
