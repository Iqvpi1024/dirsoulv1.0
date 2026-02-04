-- Stable Concepts Table - Promoted, Versioned Concepts
--
-- This table stores concepts that have passed the Promotion Gate
-- and are considered stable knowledge about the user.
--
-- # Design Principles (HEAD.md)
-- - **Promotion Gate**: Only high-confidence, time-tested views are promoted
-- - **Versioning**: Concepts can evolve over time
-- - **Immutable history**: Old versions are preserved, not overwritten

-- Create stable_concepts table
CREATE TABLE IF NOT EXISTS stable_concepts (
    -- Primary key
    concept_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- User ownership
    user_id TEXT NOT NULL,

    -- Concept identity
    canonical_name TEXT NOT NULL,               -- Standard name: "likes_fruit"
    display_name TEXT NOT NULL,                -- Human-readable: "喜欢吃水果"
    concept_type TEXT NOT NULL,                -- preference | habit | pattern | fact

    -- Concept content
    description TEXT,                           -- Detailed description
    definition JSONB NOT NULL DEFAULT '{}'::JSONB,  -- Structured definition

    -- Version tracking
    version INTEGER NOT NULL DEFAULT 1,         -- Current version
    parent_concept_id UUID REFERENCES stable_concepts(concept_id),  -- Previous version
    is_deprecated BOOLEAN NOT NULL DEFAULT FALSE,  -- Deprecated flag

    -- Promotion metadata
    promoted_from UUID,  -- Source view (no FK - avoid circular dependency)
    promoted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    promotion_confidence FLOAT NOT NULL,      -- Confidence at promotion time

    -- Lifecycle
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    deprecated_at TIMESTAMP WITH TIME ZONE,     -- When deprecated (if applicable)

    -- Usage tracking
    access_count INTEGER NOT NULL DEFAULT 0,    -- How many times accessed
    last_accessed_at TIMESTAMP WITH TIME ZONE,  -- Last access time

    -- Metadata
    source TEXT NOT NULL,                       -- Where it came from
    tags JSONB DEFAULT '{}'::JSONB,             -- Tags for categorization
    metadata JSONB DEFAULT '{}'::JSONB         -- Additional metadata
);

-- Unique constraint: one active version per canonical name per user
CREATE UNIQUE INDEX IF NOT EXISTS idx_stable_concepts_user_name_active
    ON stable_concepts(user_id, canonical_name)
    WHERE is_deprecated = FALSE;

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_stable_concepts_user_type
    ON stable_concepts(user_id, concept_type);

CREATE INDEX IF NOT EXISTS idx_stable_concepts_promoted_from
    ON stable_concepts(promoted_from);

CREATE INDEX IF NOT EXISTS idx_stable_concepts_created_at
    ON stable_concepts(created_at DESC);

-- Trigger to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_stable_concepts_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_stable_concepts_updated_at
    BEFORE UPDATE ON stable_concepts
    FOR EACH ROW
    EXECUTE FUNCTION update_stable_concepts_updated_at();

-- Comment for documentation
COMMENT ON TABLE stable_concepts IS 'Stable concepts - promoted views that have passed the promotion gate';
COMMENT ON COLUMN stable_concepts.canonical_name IS 'Unique identifier (machine-readable), e.g., "likes_fruit"';
COMMENT ON COLUMN stable_concepts.display_name IS 'Human-readable name, e.g., "喜欢吃水果"';
COMMENT ON COLUMN stable_concepts.version IS 'Version number - allows concepts to evolve while preserving history';
COMMENT ON COLUMN stable_concepts.parent_concept_id IS 'Links to previous version for version history';
COMMENT ON COLUMN stable_concepts.is_deprecated IS 'Marked as deprecated when replaced by a new version';
COMMENT ON COLUMN stable_concepts.promoted_from IS 'References the cognitive_view that was promoted';
COMMENT ON COLUMN stable_concepts.promotion_confidence IS 'Confidence score at time of promotion (should be >0.85)';
