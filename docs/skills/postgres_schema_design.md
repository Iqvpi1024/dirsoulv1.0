# Skill: Postgres Schema Design

> **Purpose**: Guide database schema design and migrations, ensuring partitioning, index optimization, and support for 10+ years of data growth without degradation.

---

## Declarative Schema Patterns

### JSONB Usage Rules

```sql
-- ✅ CORRECT: JSONB for flexible metadata
CREATE TABLE raw_memories (
    memory_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    content_type TEXT NOT NULL,
    content TEXT,
    encrypted BYTEA,
    metadata JSONB DEFAULT '{}',  -- Flexible schema-free storage
    embedding VECTOR(768)
);

-- ✅ Query JSONB with GIN index
CREATE INDEX idx_raw_memories_metadata_gin ON raw_memories USING GIN (metadata);

-- ✅ Query patterns
SELECT * FROM raw_memories
WHERE metadata->>'source' = 'voice_input'
  AND metadata->>'language' = 'zh-CN';

-- ❌ AVOID: JSONB for frequently queried fields
CREATE TABLE bad_example (
    event_id UUID PRIMARY KEY,
    action JSONB,  -- Bad: action is queried constantly
    target JSONB   -- Bad: target needs indexing
);

-- ✅ CORRECT: Separate columns for query-heavy fields
CREATE TABLE event_memories (
    event_id UUID PRIMARY KEY,
    action TEXT NOT NULL,      -- Indexed, queryable
    target TEXT NOT NULL,      -- Indexed, queryable
    metadata JSONB DEFAULT '{}' -- For flexible additional data
);
```

### Partitioning Strategy (Monthly)

```sql
-- ✅ CORRECT: Partition by month for 10+ year scale
CREATE TABLE raw_memories (
    memory_id UUID PRIMARY KEY,
    user_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    content TEXT,
    metadata JSONB,
    embedding VECTOR(768)
) PARTITION BY RANGE (created_at);

-- Create monthly partitions
CREATE TABLE raw_memories_2026_01 PARTITION OF raw_memories
    FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');

CREATE TABLE raw_memories_2026_02 PARTITION OF raw_memories
    FOR VALUES FROM ('2026-02-01') TO ('2026-03-01');

-- ✅ Automated partition creation function
CREATE OR REPLACE FUNCTION create_monthly_partition(table_name TEXT, start_date DATE)
RETURNS void AS $$
DECLARE
    partition_name TEXT;
    end_date DATE;
BEGIN
    partition_name := table_name || '_' || to_char(start_date, 'YYYY_MM');
    end_date := start_date + interval '1 month';

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF %I
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, table_name, start_date, end_date
    );
END;
$$ LANGUAGE plpgsql;

-- ✅ Automatic future partition creation
CREATE EXTENSION IF NOT EXISTS pg_cron;

SELECT cron.schedule(
    'create_monthly_partitions',
    '0 0 1 * *',  -- First day of every month
    $$SELECT create_monthly_partition('raw_memories', date_trunc('month', NOW() + interval '1 month')::date)$$
);
```

---

## pgvector Integration

### HNSW vs IVFFlat Selection

| Index Type | Build Speed | Query Speed | Memory Use | Best For |
|------------|-------------|-------------|------------|----------|
| HNSW | Slow | Fast | High | Real-time queries, frequent updates |
| IVFFlat | Fast | Medium | Medium | Batch loading, read-heavy workloads |

```sql
-- ✅ HNSW for real-time querying (recommended for DirSoul)
CREATE INDEX idx_raw_memories_embedding_hnsw
ON raw_memories
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- ✅ IVFFlat for batch scenarios
CREATE INDEX idx_raw_memories_embedding_ivf
ON raw_memories
USING ivfflat (embedding vector_cosine_ops)
WITH (lists = 100);

-- ✅ Vector similarity search with threshold
SELECT memory_id, content,
       1 - (embedding <=> $1) as similarity
FROM raw_memories
WHERE embedding <=> $1 < 0.2  -- Cosine distance < 0.2 = similarity > 0.8
  AND user_id = $2
ORDER BY embedding <=> $1
LIMIT 10;
```

### Vector Index Maintenance

```sql
-- ✅ Rebuild index when data distribution changes
REINDEX INDEX CONCURRENTLY idx_raw_memories_embedding_hnsw;

-- ✅ Monitor index effectiveness
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan as index_scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched
FROM pg_stat_user_indexes
WHERE tablename = 'raw_memories';
```

---

## Performance Rules

### Composite Index Design

```sql
-- ✅ CORRECT: Composite index for time-range queries per user
CREATE INDEX idx_events_user_time
ON event_memories (user_id, timestamp DESC);

-- Queries this index supports:
SELECT * FROM event_memories
WHERE user_id = 'user123'
  AND timestamp >= NOW() - interval '7 days'
ORDER BY timestamp DESC;

-- ✅ CORRECT: Action-target composite for pattern detection
CREATE INDEX idx_events_action_target
ON event_memories (action, target, timestamp DESC);

-- Supports: "How many times did I eat [target]?"
SELECT count(*), target
FROM event_memories
WHERE user_id = 'user123'
  AND action = 'eat'
  AND timestamp >= NOW() - interval '30 days'
GROUP BY target;

-- ❌ AVOID: Low-selectivity leading column
CREATE INDEX idx_bad_example
ON event_memories (timestamp, user_id);  -- timestamp has low selectivity

-- ✅ CORRECT: Put user_id first (higher selectivity)
CREATE INDEX idx_good_example
ON event_memories (user_id, timestamp);
```

### Partial Indexes for Hot Data

```sql
-- ✅ Index only recent data (hot partition)
CREATE INDEX idx_events_recent
ON event_memories (user_id, timestamp DESC)
WHERE timestamp >= NOW() - interval '3 months';

-- ✅ Index only high-confidence events
CREATE INDEX idx_events_confident
ON event_memories (action, target, quantity)
WHERE confidence >= 0.8;
```

### Covering Indexes for Hot Queries

```sql
-- ✅ INCLUDE for avoiding heap access
CREATE INDEX idx_events_covering
ON event_memories (user_id, timestamp DESC)
INCLUDE (action, target, quantity);

-- Now this query only touches index, no heap access
SELECT action, target, quantity
FROM event_memories
WHERE user_id = 'user123'
  AND timestamp >= NOW() - interval '7 days'
ORDER BY timestamp DESC;
```

---

## Constraints and Triggers

### Data Integrity Constraints

```sql
-- ✅ CHECK constraints for business rules
ALTER TABLE event_memories
ADD CONSTRAINT chk_confidence_range
CHECK (confidence >= 0 AND confidence <= 1);

ALTER TABLE event_memories
ADD CONSTRAINT chk_quantity_positive
CHECK (quantity IS NULL OR quantity > 0);

ALTER TABLE raw_memories
ADD CONSTRAINT chk_content_or_encrypted
CHECK (
    (content IS NOT NULL AND encrypted IS NULL) OR
    (content IS NULL AND encrypted IS NOT NULL)
);

-- ✅ EXCLUDE constraints for uniqueness within partition
ALTER TABLE event_memories
ADD CONSTRAINT uq_event_per_memory
EXCLUDE USING gist (
    memory_id WITH =
);
```

### Automated Maintenance Triggers

```sql
-- ✅ Auto-update derived view expiration
CREATE OR REPLACE FUNCTION check_view_expiration()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.expires_at <= NOW() AND NEW.status = 'active' THEN
        NEW.status := 'expired';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_check_expiration
BEFORE UPDATE ON cognitive_views
FOR EACH ROW
EXECUTE FUNCTION check_view_expiration();

-- ✅ Auto-update entity metadata stats
CREATE OR REPLACE FUNCTION update_entity_stats()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE entities
    SET mention_count = mention_count + 1,
        last_seen = NOW()
    WHERE entity_id = NEW.entity_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_entity_stats
AFTER INSERT ON entity_mentions
FOR EACH ROW
EXECUTE FUNCTION update_entity_stats();
```

---

## 10+ Year Scalability Patterns

### Data Lifecycle Management

```sql
-- ✅ Archive old data to separate schema
CREATE SCHEMA archive;

-- Move data older than 2 years
INSERT INTO archive.raw_memories_archive
SELECT * FROM raw_memories
WHERE created_at < NOW() - interval '2 years';

DELETE FROM raw_memories
WHERE created_at < NOW() - interval '2 years';

-- ✅ Use hypertables for automatic partitioning (TimescaleDB extension)
CREATE EXTENSION IF NOT EXISTS timescaledb;

SELECT create_hypertable('event_memories', 'timestamp',
    chunk_time_interval => interval '1 month');
```

### Vacuum and Analyze Strategy

```sql
-- ✅ Auto-vacuum tuning for write-heavy workload
ALTER TABLE raw_memories SET (
    autovacuum_vacuum_scale_factor = 0.1,     -- 10% changes trigger vacuum
    autovacuum_analyze_scale_factor = 0.05,   -- 5% changes trigger analyze
    autovacuum_vacuum_threshold = 1000        -- Or 1000 rows
);

-- ✅ Manual vacuum for large operations
VACUUM FULL ANALYZE raw_memories;

-- ✅ Reclaim space after deletes
VACUUM (VERBOSE, INDEX_CLEANUP ON) raw_memories;
```

---

## Migration Best Practices

```sql
-- ✅ Idempotent migrations (can run multiple times)
CREATE OR REPLACE FUNCTION migrate_add_metadata_column()
RETURNS void AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'raw_memories' AND column_name = 'metadata'
    ) THEN
        ALTER TABLE raw_memories ADD COLUMN metadata JSONB DEFAULT '{}';
    END IF;
END;
$$ LANGUAGE plpgsql;

SELECT migrate_add_metadata_column();

-- ✅ Backward-compatible additions
ALTER TABLE event_memories
ADD COLUMN IF NOT EXISTS source TEXT DEFAULT 'manual';

-- ✅ Safe index creation (CONCURRENTLY)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_events_user_time
ON event_memories (user_id, timestamp DESC);
```

---

## Monitoring Queries

```sql
-- ✅ Table size monitoring
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- ✅ Partition size distribution
SELECT
    tablename,
    pg_size_pretty(pg_total_relation_size(tablename::regclass)) AS size
FROM pg_tables
WHERE tablename LIKE 'raw_memories_%'
ORDER BY tablename DESC;

-- ✅ Index usage statistics
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    pg_size_pretty(pg_relation_size(indexrelid::regclass)) AS index_size
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan ASC;  -- Least used indexes first
```

---

## Recommended Combinations

Use this skill together with:
- **EventExtractionPatterns**: For event table structure
- **EntityResolution**: For entity schema design
- **CognitiveViewGeneration**: For derived views storage
- **TestingAndDebugging**: For schema migration testing
