ALTER TABLE public.user_org_roles
    ADD COLUMN IF NOT EXISTS id uuid;

UPDATE public.user_org_roles
SET id = gen_random_uuid()
WHERE id IS NULL;

ALTER TABLE public.user_org_roles
    ALTER COLUMN id SET DEFAULT gen_random_uuid(),
    ALTER COLUMN id SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS user_org_roles_id_uidx
    ON public.user_org_roles (id);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'user_org_roles_membership_status_check'
          AND conrelid = 'public.user_org_roles'::regclass
    ) THEN
        ALTER TABLE public.user_org_roles
            ADD CONSTRAINT user_org_roles_membership_status_check
            CHECK (membership_status IN ('pending', 'active', 'moved_out'));
    END IF;
END
$$;
