-- DirSoul Migration: Rollback entities and entity_relations tables
-- Phase 4, Task 4.1

-- Drop indexes (PostgreSQL will drop them automatically with TABLE DROP, but explicit for safety)
DROP INDEX IF EXISTS idx_entity_relations_strength;
DROP INDEX IF EXISTS idx_entity_relations_user_type;
DROP INDEX IF EXISTS idx_entity_relations_target;
DROP INDEX IF EXISTS idx_entity_relations_source;
DROP INDEX IF EXISTS idx_entities_confidence;
DROP INDEX IF EXISTS idx_entities_occurrence_count;
DROP INDEX IF EXISTS idx_entities_canonical_name;
DROP INDEX IF EXISTS idx_entities_user_type;

-- Drop tables (entity_relations first due to foreign key constraints)
DROP TABLE IF EXISTS entity_relations;
DROP TABLE IF EXISTS entities;
