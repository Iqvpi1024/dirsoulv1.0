-- Rollback stable_concepts table

-- Drop the table
DROP TABLE IF EXISTS stable_concepts CASCADE;

-- Drop the trigger function
DROP FUNCTION IF EXISTS update_stable_concepts_updated_at CASCADE;
