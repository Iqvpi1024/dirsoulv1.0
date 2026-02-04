-- Rollback: Remove agents table and system agents

DROP TRIGGER IF EXISTS trigger_update_agents_updated_at ON agents;
DROP FUNCTION IF EXISTS update_agents_updated_at();

DROP INDEX IF EXISTS idx_agents_last_used;
DROP INDEX IF EXISTS idx_agents_active;
DROP INDEX IF EXISTS idx_agents_user_type;
DROP INDEX IF NOT EXISTS idx_agents_user_builtin_type;

DELETE FROM agents
WHERE is_builtin = TRUE AND agent_type IN ('cognitive', 'decision');

DROP TABLE IF EXISTS agents;
