-- Seed the organization that owns authenticated Explorer/prospect contexts.
-- The UUID matches the backend's default PLATFORM_ORG_ID. Both inserts are
-- deliberately non-destructive so migration replay cannot overwrite operator
-- customizations on an existing installation.
INSERT INTO public.organizations (id, name)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'Hamsa Vilas Platform'
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO public.tenant_placement (org_id, mode, schema_name, status)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'pooled',
    'pool',
    'active'
)
ON CONFLICT (org_id) DO NOTHING;
