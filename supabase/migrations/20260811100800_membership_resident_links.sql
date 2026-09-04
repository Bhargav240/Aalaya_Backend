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
DECLARE
    tenant_schema text;
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

    FOR tenant_schema IN
        SELECT 'pool'
        UNION
        SELECT DISTINCT schema_name
        FROM public.tenant_placement
        WHERE mode = 'siloed' AND schema_name IS NOT NULL
    LOOP
        IF to_regclass(format('%I.residents', tenant_schema)) IS NULL THEN
            CONTINUE;
        END IF;
        EXECUTE format(
            'ALTER TABLE %I.residents ADD COLUMN IF NOT EXISTS membership_id uuid REFERENCES public.user_org_roles(id)',
            tenant_schema
        );
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS residents_org_membership_idx ON %I.residents (org_id, membership_id)',
            tenant_schema
        );
        EXECUTE format(
            'CREATE UNIQUE INDEX IF NOT EXISTS residents_one_live_per_membership_uidx ON %I.residents (org_id, membership_id) WHERE membership_id IS NOT NULL AND deleted_at IS NULL AND status <> ''moved_out''',
            tenant_schema
        );
    END LOOP;
END
$$;
