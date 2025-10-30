-- Create table for linking users to DIDs
CREATE TABLE IF NOT EXISTS user_dids (
  user_did_id UUID PRIMARY KEY,
  user_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
  did TEXT NOT NULL UNIQUE,
  method TEXT,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL,
  revoked_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS user_dids_user_id_idx ON user_dids (user_id);
CREATE INDEX IF NOT EXISTS user_dids_method_idx ON user_dids (method);
