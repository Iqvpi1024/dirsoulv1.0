-- Rollback cognitive_views table

-- Drop the table ( CASCADE will drop the foreign key constraint)
DROP TABLE IF EXISTS cognitive_views CASCADE;

-- Drop the function
DROP FUNCTION IF EXISTS mark_expired_views CASCADE;

-- Drop the trigger function
DROP FUNCTION IF EXISTS update_cognitive_views_updated_at CASCADE;
