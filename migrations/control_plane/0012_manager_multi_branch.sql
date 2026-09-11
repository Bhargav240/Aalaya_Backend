-- Allow a user to be assigned to multiple branches within the same organization (e.g. a manager managing multiple branches),
-- or to have an org-wide manager role (branch_id IS NULL).

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'user_org_roles_pkey'
          AND conrelid = 'public.user_org_roles'::regclass
    ) THEN
        ALTER TABLE public.user_org_roles DROP CONSTRAINT user_org_roles_pkey;
        ALTER TABLE public.user_org_roles ADD PRIMARY KEY (id);
    END IF;
END
$$;

CREATE UNIQUE INDEX IF NOT EXISTS user_org_roles_multi_branch_uidx
    ON public.user_org_roles (user_id, org_id, role, branch_id)
    WHERE branch_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS user_org_roles_org_wide_uidx
    ON public.user_org_roles (user_id, org_id, role)
    WHERE branch_id IS NULL;
