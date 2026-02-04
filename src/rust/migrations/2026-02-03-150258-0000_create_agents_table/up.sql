-- Agents Table - Plugin and Agent Management
--
-- This table stores agents and plugins with their permissions,
-- enabling the plugin system with permission hierarchy.
--
-- # Design Principles (HEAD.md)
-- - **插件沙箱隔离**: 插件崩溃不影响系统
-- - **权限分级**: ReadOnly / ReadWriteDerived / ReadWriteEvents
-- - **最小权限原则**: 只授予必要的访问权限
--
-- # Skill Reference
-- - docs/skills/plugin_permission_system.md

-- Create agents table
CREATE TABLE IF NOT EXISTS agents (
    -- Primary key
    agent_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Agent identity
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '1.0.0',

    -- Description
    description TEXT,
    author TEXT NOT NULL DEFAULT 'system',

    -- Permission configuration
    permissions JSONB NOT NULL DEFAULT '{}'::JSONB,

    -- Status
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMP WITH TIME ZONE,

    -- Metadata
    metadata JSONB DEFAULT '{}'::JSONB,
    tags JSONB DEFAULT '[]'::JSONB
);

-- Unique constraint for built-in agents
CREATE UNIQUE INDEX IF NOT EXISTS idx_agents_user_builtin_type
    ON agents(user_id, agent_type)
    WHERE is_builtin = TRUE;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_agents_user_type ON agents(user_id, agent_type);
CREATE INDEX IF NOT EXISTS idx_agents_active ON agents(user_id, is_active);
CREATE INDEX IF NOT EXISTS idx_agents_last_used ON agents(last_used_at DESC) WHERE last_used_at IS NOT NULL;

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_agents_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_agents_updated_at
    BEFORE UPDATE ON agents
    FOR EACH ROW
    EXECUTE FUNCTION update_agents_updated_at();

-- Insert system agents
INSERT INTO agents (
    user_id, name, agent_type, version,
    description, author, permissions, is_builtin, metadata
) VALUES
(
    'system',
    'Cognitive Assistant',
    'cognitive',
    '1.0.0',
    'Cognitive memory assistant for pattern analysis and view generation',
    'system',
    '{
        "memory_level": 2,
        "can_create_events": false,
        "can_modify_views": true,
        "can_read_entities": true,
        "allowed_operations": ["query_stats", "generate_view", "read_views"]
    }'::JSONB,
    TRUE,
    '{"system_agent": true, "category": "analysis"}'::JSONB
),
(
    'system',
    'Decision Helper',
    'decision',
    '1.0.0',
    'Decision support agent for analyzing options and recommendations',
    'system',
    '{
        "memory_level": 2,
        "can_create_events": true,
        "can_modify_views": false,
        "can_read_entities": true,
        "allowed_operations": ["query_stats", "log_recommendation", "read_views"]
    }'::JSONB,
    TRUE,
    '{"system_agent": true, "category": "decision"}'::JSONB
)
ON CONFLICT (user_id, agent_type) WHERE is_builtin = TRUE
DO NOTHING;

-- Comments
COMMENT ON TABLE agents IS 'Agents and plugins with permission-controlled memory access';
COMMENT ON COLUMN agents.permissions IS 'JSONB defining memory access permissions (1=ReadOnly, 2=ReadWriteDerived, 3=ReadWriteEvents)';
COMMENT ON COLUMN agents.is_builtin IS 'TRUE for pre-built system agents';
