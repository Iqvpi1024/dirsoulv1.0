-- DirSoul Migration: Rollback raw_memories table
-- Phase 2, Task 2.1

-- Drop the main table (CASCADE will drop indexes automatically)
DROP TABLE IF EXISTS raw_memories CASCADE;
