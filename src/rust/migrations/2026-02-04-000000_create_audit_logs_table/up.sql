-- Create audit_logs table for security and compliance
-- Records who did what, when, and the result

CREATE TABLE audit_logs (
    id SERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT,
    result_count INTEGER,
    ip_address TEXT,
    metadata JSONB
);

-- Indexes for common queries
CREATE INDEX idx_audit_logs_user_timestamp ON audit_logs(user_id, timestamp DESC);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX idx_audit_logs_success ON audit_logs(success, timestamp DESC);

-- Index for GIN queries on metadata
CREATE INDEX idx_audit_logs_metadata ON audit_logs USING GIN(metadata);

-- Comment for documentation
COMMENT ON TABLE audit_logs IS 'Audit log for security and compliance - records all data access';
COMMENT ON COLUMN audit_logs.user_id IS 'User who performed the action';
COMMENT ON COLUMN audit_logs.action IS 'Action performed (query, insert, update, delete, export, etc.)';
COMMENT ON COLUMN audit_logs.target IS 'Target resource (events, views, entities, etc.)';
COMMENT ON COLUMN audit_logs.result_count IS 'Number of results returned';
COMMENT ON COLUMN audit_logs.ip_address IS 'Client IP address (for remote access)';
