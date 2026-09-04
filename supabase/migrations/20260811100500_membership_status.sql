ALTER TABLE public.user_org_roles
    ADD COLUMN IF NOT EXISTS membership_status text NOT NULL DEFAULT 'active',
    ADD COLUMN IF NOT EXISTS activated_at timestamptz,
    ADD COLUMN IF NOT EXISTS ended_at timestamptz;
CREATE INDEX IF NOT EXISTS user_org_roles_status_idx ON public.user_org_roles (org_id, role, membership_status);
