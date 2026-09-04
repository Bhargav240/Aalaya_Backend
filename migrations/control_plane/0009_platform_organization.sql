-- Seed the control-plane organization used by authenticated Explorer accounts.
--
-- This identifier is the default PLATFORM_ORG_ID in application configuration.
-- Keep the insert non-destructive: an operator may rename the organization or
-- customize its placement after the first deployment, and a migration replay
-- must not overwrite those choices.
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
