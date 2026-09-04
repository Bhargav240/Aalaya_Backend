CREATE TABLE IF NOT EXISTS public.tenant_invitations (
    id uuid PRIMARY KEY, org_id uuid NOT NULL REFERENCES public.organizations(id), branch_id uuid NOT NULL,
    intended_room_id uuid, intended_bed_id uuid, email text NOT NULL, full_name text NOT NULL,
    monthly_rent_paise bigint, deposit_paise bigint, planned_move_in_date date,
    require_identity_verification boolean NOT NULL DEFAULT false, status text NOT NULL DEFAULT 'invited',
    invited_by_user_id uuid NOT NULL REFERENCES public.platform_users(id), accepted_by_user_id uuid REFERENCES public.platform_users(id),
    expires_at timestamptz NOT NULL, accepted_at timestamptz, approved_at timestamptz,
    approved_by_user_id uuid REFERENCES public.platform_users(id), created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS tenant_invitations_org_status_idx ON public.tenant_invitations (org_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS tenant_invitations_email_idx ON public.tenant_invitations (lower(email), status);
