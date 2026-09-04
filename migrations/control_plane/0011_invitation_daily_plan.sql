ALTER TABLE public.tenant_invitations
    DROP CONSTRAINT IF EXISTS tenant_invitations_plan_kind_check;
ALTER TABLE public.tenant_invitations
    ADD CONSTRAINT tenant_invitations_plan_kind_check
    CHECK (plan_kind IN ('daily', 'weekly', 'monthly'));
