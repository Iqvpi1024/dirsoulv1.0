-- DirSoul Migration: Create event_memories table (Layer 2 - Event Memory)
-- Phase 3, Task 3.1
-- Follows: PostgresSchemaDesign, EventExtractionPatterns skills
--
-- Design Principles (from HEAD.md):
-- - 每个事件必须有精确时间戳
-- - 数量必须结构化存储
-- - 行为必须类型化
-- - 支持时间范围查询

-- Create event_memories table
CREATE TABLE event_memories (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    memory_id UUID NOT NULL REFERENCES raw_memories(memory_id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    actor TEXT,
    action TEXT NOT NULL,
    target TEXT NOT NULL,
    quantity FLOAT,
    unit TEXT,
    confidence FLOAT NOT NULL,
    extractor_version TEXT,

    -- Ensure confidence is in valid range [0, 1]
    CONSTRAINT chk_confidence_range CHECK (
        confidence >= 0.0 AND confidence <= 1.0
    ),

    -- Ensure quantity and unit are consistent
    CONSTRAINT chk_quantity_unit_consistency CHECK (
        (quantity IS NULL AND unit IS NULL) OR
        (quantity IS NOT NULL)
    )
);

-- Composite index for time-range queries (per user) - most critical for event queries
-- Enables: "Show me all events from last week"
CREATE INDEX idx_event_memories_user_time ON event_memories (user_id, timestamp DESC);

-- Composite index for action-target queries
-- Enables: "How many times did I [action] [target]?"
CREATE INDEX idx_event_memories_action_target ON event_memories (action, target);

-- Index for actor-based queries
-- Enables: "What events involved [actor]?"
CREATE INDEX idx_event_memories_actor ON event_memories (actor) WHERE actor IS NOT NULL;

-- Index for confidence-based filtering (used in promotion gate)
-- Enables: "Show high-confidence events"
CREATE INDEX idx_event_memories_confidence ON event_memories (confidence) WHERE confidence >= 0.7;

-- Composite index for pattern detection queries
-- Enables: "Show all [action] events for [user] in timeframe"
CREATE INDEX idx_event_memories_user_action_time ON event_memories (user_id, action, timestamp DESC);

-- Comments for documentation
COMMENT ON TABLE event_memories IS 'Layer 2: Event Memory - structured events extracted from raw memories';
COMMENT ON COLUMN event_memories.event_id IS 'Unique identifier for each event';
COMMENT ON COLUMN event_memories.memory_id IS 'Reference to source raw memory (CASCADE DELETE)';
COMMENT ON COLUMN event_memories.user_id IS 'User who owns this event (denormalized for query performance)';
COMMENT ON COLUMN event_memories.timestamp IS 'Precise timestamp when event occurred (HEAD.md: 每个事件必须有精确时间戳)';
COMMENT ON COLUMN event_memories.actor IS 'Entity that performed the action (optional)';
COMMENT ON COLUMN event_memories.action IS 'Action performed (HEAD.md: 行为必须类型化)';
COMMENT ON COLUMN event_memories.target IS 'Object/entity affected by the action';
COMMENT ON COLUMN event_memories.quantity IS 'Structured quantity (HEAD.md: 数量必须结构化存储)';
COMMENT ON COLUMN event_memories.unit IS 'Unit of measurement (e.g., 个, kg, 次)';
COMMENT ON COLUMN event_memories.confidence IS 'Extraction confidence (0-1), used in promotion gate';
COMMENT ON COLUMN event_memories.extractor_version IS 'Version of extractor that created this event';
