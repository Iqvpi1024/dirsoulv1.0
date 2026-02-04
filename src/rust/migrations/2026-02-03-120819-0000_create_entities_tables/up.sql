-- DirSoul Migration: Create entities and entity_relations tables (Layer 2 - Structured Memory)
-- Phase 4, Task 4.1
-- Follows: PostgresSchemaDesign, EntityResolution skills
--
-- Design Principles (from HEAD.md):
-- - 暂用 Postgres 数组+JSONB 模拟关系图谱
-- - 不过早引入图数据库/知识图谱
-- - 支持实体消歧和属性动态增长

-- Create entities table
CREATE TABLE entities (
    entity_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    canonical_name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    attributes JSONB DEFAULT '{}',
    first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    confidence FLOAT NOT NULL DEFAULT 0.5,
    embedding VECTOR(512),

    -- Unique constraint: (user_id, canonical_name)
    CONSTRAINT uq_entities_user_canonical UNIQUE (user_id, canonical_name)
);

-- Create entity_relations table
CREATE TABLE entity_relations (
    relation_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    source_entity_id UUID NOT NULL REFERENCES entities(entity_id) ON DELETE CASCADE,
    target_entity_id UUID NOT NULL REFERENCES entities(entity_id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL,
    confidence FLOAT NOT NULL DEFAULT 0.5,
    first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    strength FLOAT NOT NULL DEFAULT 1.0
);

-- Indexes for entities table
-- Fast lookup by user and type
CREATE INDEX idx_entities_user_type ON entities (user_id, entity_type);

-- Index for canonical name search (LIKE queries)
CREATE INDEX idx_entities_canonical_name ON entities (canonical_name);

-- Index for occurrence count (find most common entities)
CREATE INDEX idx_entities_occurrence_count ON entities (occurrence_count DESC);

-- Index for confidence (filter high-confidence entities)
CREATE INDEX idx_entities_confidence ON entities (confidence) WHERE confidence >= 0.7;

-- HNSW vector index for entity disambiguation
CREATE INDEX idx_entities_embedding_hnsw
ON entities
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- Indexes for entity_relations table
-- Index for finding relations of an entity (both source and target)
CREATE INDEX idx_entity_relations_source ON entity_relations (source_entity_id);
CREATE INDEX idx_entity_relations_target ON entity_relations (target_entity_id);

-- Composite index for relation type queries
CREATE INDEX idx_entity_relations_user_type ON entity_relations (user_id, relation_type);

-- Composite index for strength filtering
CREATE INDEX idx_entity_relations_strength ON entity_relations (strength DESC) WHERE strength >= 0.5;

-- Comments for documentation
COMMENT ON TABLE entities IS 'Layer 2: Structured Memory - Entities (objects, people, concepts)';
COMMENT ON COLUMN entities.entity_id IS 'Unique identifier for each entity';
COMMENT ON COLUMN entities.user_id IS 'User who owns this entity';
COMMENT ON COLUMN entities.canonical_name IS 'Standard name (e.g., "Apple" for "苹果")';
COMMENT ON COLUMN entities.entity_type IS 'Type: person/place/object/concept/organization';
COMMENT ON COLUMN entities.attributes IS 'Dynamic attributes (JSONB): {color: "red", category: "fruit"}';
COMMENT ON COLUMN entities.first_seen IS 'First time this entity appeared in events';
COMMENT ON COLUMN entities.last_seen IS 'Most recent time this entity appeared';
COMMENT ON COLUMN entities.occurrence_count IS 'Number of times entity has been seen';
COMMENT ON COLUMN entities.confidence IS 'Confidence in entity classification (0-1)';
COMMENT ON COLUMN entities.embedding IS 'Vector embedding for entity disambiguation (512 dimensions, nomic-embed-text)';

COMMENT ON TABLE entity_relations IS 'Relationships between entities (simulates graph structure)';
COMMENT ON COLUMN entity_relations.relation_id IS 'Unique identifier for each relation';
COMMENT ON COLUMN entity_relations.source_entity_id IS 'Entity that is the source of the relation';
COMMENT ON COLUMN entity_relations.target_entity_id IS 'Entity that is the target of the relation';
COMMENT ON COLUMN entity_relations.relation_type IS 'Type: belongs_to/related_to/located_at/etc';
COMMENT ON COLUMN entity_relations.confidence IS 'Confidence in relationship (0-1)';
COMMENT ON COLUMN entity_relations.strength IS 'Relationship strength (based on co-occurrence frequency)';
