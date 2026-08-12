-- 002_app_users.sql
-- User accounts + tenant registry for the auth system.
--
-- NOTE: `tenants` is created here (idempotently) because the schema has no
-- other definition of it, yet `roles`, `user_roles` (001_rbac.sql) and
-- `app_users` all reference `tenants(id)`.

CREATE TABLE IF NOT EXISTS tenants (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT NOT NULL,
    code       TEXT,
    is_active  BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (code)
);

CREATE TABLE app_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    email TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    full_name TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_superadmin BOOLEAN NOT NULL DEFAULT FALSE,
    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(tenant_id, email)
);

-- Login looks users up by email (optionally scoped to a tenant).
CREATE INDEX IF NOT EXISTS idx_app_users_email ON app_users (email);
CREATE INDEX IF NOT EXISTS idx_app_users_tenant ON app_users (tenant_id);
