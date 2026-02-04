-- DirSoul Migration: Rollback event_memories table
-- Phase 3, Task 3.1

-- Drop indexes (PostgreSQL will drop them automatically with TABLE DROP, but explicit for safety)
DROP INDEX IF EXISTS idx_event_memories_user_action_time;
DROP INDEX IF EXISTS idx_event_memories_confidence;
DROP INDEX IF EXISTS idx_event_memories_actor;
DROP INDEX IF EXISTS idx_event_memories_action_target;
DROP INDEX IF EXISTS idx_event_memories_user_time;

-- Drop event_memories table
DROP TABLE IF EXISTS event_memories;
