-- DirSoul Migration: Create raw_memories table (Layer 1 - Raw Memory)
-- Phase 2, Task 2.1
-- Follows: PostgresSchemaDesign, EncryptionBestPractices skills
--
-- NOTE: Partitioning deferred until data volume justifies it (Phase 7)
-- Initial implementation uses standard table with optimized indexes

-- Enable pgvector extension if not already enabled
CREATE EXTENSION IF NOT EXISTS vector;

-- Create raw_memories table (non-partitioned initially)
CREATE TABLE raw_memories (
    memory_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    content_type TEXT NOT NULL,
    content TEXT,
    encrypted BYTEA,
    metadata JSONB DEFAULT '{}',
    embedding VECTOR(512),

    -- Ensure mutual exclusivity: either plaintext OR encrypted, never both
    CONSTRAINT chk_content_or_encrypted CHECK (
        (content IS NOT NULL AND encrypted IS NULL) OR
        (content IS NULL AND encrypted IS NOT NULL)
    ),

    -- Valid content types
    CONSTRAINT chk_valid_content_type CHECK (
        content_type IN ('text', 'voice', 'image', 'document', 'action', 'external')
    )
);

-- Indexes for time-range queries (per user) - most critical for event queries
CREATE INDEX idx_raw_memories_user_time ON raw_memories (user_id, created_at DESC);

-- Index for metadata queries (JSONB GIN index)
CREATE INDEX idx_raw_memories_metadata_gin ON raw_memories USING GIN (metadata);

-- HNSW vector index for similarity search (real-time queries)
CREATE INDEX idx_raw_memories_embedding_hnsw
ON raw_memories
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- Comment for documentation
-- NOTE: Partial index for recent data deferred until Phase 7 (data lifecycle management)
-- PostgreSQL requires IMMUTABLE functions in index predicates
COMMENT ON TABLE raw_memories IS 'Layer 1: Raw Memory - append-only storage for all user inputs';
COMMENT ON COLUMN raw_memories.memory_id IS 'Unique identifier for each memory';
COMMENT ON COLUMN raw_memories.user_id IS 'User who owns this memory';
COMMENT ON COLUMN raw_memories.created_at IS 'Precise timestamp when memory was created';
COMMENT ON COLUMN raw_memories.content_type IS 'Type: text/voice/image/document/action/external';
COMMENT ON COLUMN raw_memories.content IS 'Plaintext content (for debugging/testing only)';
COMMENT ON COLUMN raw_memories.encrypted IS 'Encrypted content (BYTEA, Fernet encrypted)';
COMMENT ON COLUMN raw_memories.metadata IS 'Flexible metadata stored as JSONB';
COMMENT ON COLUMN raw_memories.embedding IS 'Vector embedding for semantic search (512 dimensions, nomic-embed-text)';
