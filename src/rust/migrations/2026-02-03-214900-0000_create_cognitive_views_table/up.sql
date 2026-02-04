-- Cognitive Views Table - Derived Views + Promotion Gate
--
-- This table implements the core innovation from HEAD.md:
-- "Derived Cognitive Views" - temporary hypotheses about user patterns
-- that can be promoted to stable concepts after validation.
--
-- # Design Principles (HEAD.md)
-- - **慢抽象原则**: Derived Views 先行，可丢弃
-- - **Promotion Gate 把关**: 程序判定是否晋升为稳定概念
-- - **避免 LLM 幻觉放大**: 隔离 AI 判断与系统结构

-- Create cognitive_views table
CREATE TABLE IF NOT EXISTS cognitive_views (
    -- Primary key
    view_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- User ownership
    user_id TEXT NOT NULL,

    -- View content
    hypothesis TEXT NOT NULL,              -- "用户喜欢吃水果", "每周运动3次"
    view_type TEXT NOT NULL,               -- pattern, preference, habit, trend
    description TEXT,                       -- 详细描述

    -- Supporting evidence
    derived_from JSONB NOT NULL DEFAULT '[]'::JSONB,  -- 关联的事件ID列表 [UUID]
    evidence_count INTEGER NOT NULL DEFAULT 1,        -- 支撑证据数量

    -- Confidence & validation
    confidence FLOAT NOT NULL DEFAULT 0.5,            -- 置信度 (0.0-1.0)
    validation_count INTEGER NOT NULL DEFAULT 0,      -- 验证次数
    last_validated_at TIMESTAMP WITH TIME ZONE,       -- 最后验证时间

    -- Lifecycle management
    status TEXT NOT NULL DEFAULT 'active',           -- active | expired | promoted | rejected
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,    -- 过期时间 (默认30天)
    promoted_to UUID REFERENCES stable_concepts(concept_id),  -- 晋升后的稳定概念ID

    -- Metadata
    source TEXT NOT NULL DEFAULT 'pattern_detector',    -- 来源: pattern_detector, llm, user
    tags JSONB DEFAULT '{}'::JSONB,                         -- 标签: ["health", "habit"]
    metadata JSONB DEFAULT '{}'::JSONB                       -- 额外元数据
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_cognitive_views_user_status
    ON cognitive_views(user_id, status);

CREATE INDEX IF NOT EXISTS idx_cognitive_views_expires_at
    ON cognitive_views(expires_at)
    WHERE status = 'active';

CREATE INDEX IF NOT EXISTS idx_cognitive_views_confidence
    ON cognitive_views(confidence DESC)
    WHERE status = 'active';

CREATE INDEX IF NOT EXISTS idx_cognitive_views_type
    ON cognitive_views(view_type);

CREATE INDEX IF NOT EXISTS idx_cognitive_views_created_at
    ON cognitive_views(created_at DESC);

-- Composite index for finding views ready for promotion
CREATE INDEX IF NOT EXISTS idx_cognitive_views_promotion_ready
    ON cognitive_views(user_id, confidence, validation_count, expires_at)
    WHERE status = 'active' AND confidence > 0.85;

-- Trigger to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_cognitive_views_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_cognitive_views_updated_at
    BEFORE UPDATE ON cognitive_views
    FOR EACH ROW
    EXECUTE FUNCTION update_cognitive_views_updated_at();

-- Function to mark expired views
CREATE OR REPLACE FUNCTION mark_expired_views()
RETURNS INTEGER AS $$
DECLARE
    marked_count INTEGER;
BEGIN
    UPDATE cognitive_views
    SET status = 'expired'
    WHERE status = 'active'
      AND expires_at < NOW();

    GET DIAGNOSTICS marked_count = ROW_COUNT;
    RETURN marked_count;
END;
$$ LANGUAGE plpgsql;

-- Comment for documentation
COMMENT ON TABLE cognitive_views IS 'Derived cognitive views - temporary hypotheses that can be promoted to stable concepts';
COMMENT ON COLUMN cognitive_views.hypothesis IS 'The hypothesis or pattern this view represents (e.g., "User likes fruit")';
COMMENT ON COLUMN cognitive_views.derived_from IS 'Array of event IDs that support this hypothesis';
COMMENT ON COLUMN cognitive_views.confidence IS 'Confidence score (0.0-1.0), higher = more reliable';
COMMENT ON COLUMN cognitive_views.validation_count IS 'Number of times this view has been validated by new evidence';
COMMENT ON COLUMN cognitive_views.expires_at IS 'Expiration timestamp - views expire after 30 days by default';
COMMENT ON COLUMN cognitive_views.status IS 'View status: active (test), expired (discarded), promoted (stable), rejected (invalid)';
COMMENT ON COLUMN cognitive_views.promoted_to IS 'If promoted, references the stable_concept record';
