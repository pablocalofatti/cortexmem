CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS accounts (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE IF NOT EXISTS api_keys (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID REFERENCES accounts(id) ON DELETE CASCADE,
    key_hash    TEXT NOT NULL,
    prefix      TEXT NOT NULL,
    created_at  TIMESTAMPTZ DEFAULT now(),
    revoked_at  TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS sync_mutations (
    seq         BIGSERIAL PRIMARY KEY,
    account_id  UUID REFERENCES accounts(id) ON DELETE CASCADE,
    entity      TEXT NOT NULL,
    entity_key  TEXT NOT NULL,
    op          TEXT NOT NULL,
    payload     JSONB NOT NULL,
    project     TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    acked_at    TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS enrolled_projects (
    account_id  UUID REFERENCES accounts(id) ON DELETE CASCADE,
    project     TEXT NOT NULL,
    enrolled_at TIMESTAMPTZ DEFAULT now(),
    PRIMARY KEY (account_id, project)
);

CREATE INDEX IF NOT EXISTS idx_sync_mutations_account ON sync_mutations(account_id, seq);
CREATE INDEX IF NOT EXISTS idx_sync_mutations_project ON sync_mutations(account_id, project, seq);
CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(prefix);
