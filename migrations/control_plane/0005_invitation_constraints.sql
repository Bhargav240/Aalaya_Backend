-- Normalize the only legacy spelling used by the partial invitation routes.
UPDATE public.tenant_invitations
SET status = 'cancelled', updated_at = now()
WHERE status = 'declined';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'tenant_invitations_status_check'
          AND conrelid = 'public.tenant_invitations'::regclass
    ) THEN
        ALTER TABLE public.tenant_invitations
            ADD CONSTRAINT tenant_invitations_status_check CHECK (
                status IN (
                    'invited', 'email_verified', 'kyc_pending', 'kyc_verified',
                    'kyc_failed', 'approval_pending', 'approved', 'rejected',
                    'cancelled', 'expired', 'active'
                )
            );
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'tenant_invitations_monthly_rent_check'
          AND conrelid = 'public.tenant_invitations'::regclass
    ) THEN
        ALTER TABLE public.tenant_invitations
            ADD CONSTRAINT tenant_invitations_monthly_rent_check
            CHECK (monthly_rent_paise IS NULL OR monthly_rent_paise >= 0);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'tenant_invitations_deposit_check'
          AND conrelid = 'public.tenant_invitations'::regclass
    ) THEN
        ALTER TABLE public.tenant_invitations
            ADD CONSTRAINT tenant_invitations_deposit_check
            CHECK (deposit_paise IS NULL OR deposit_paise >= 0);
    END IF;
END
$$;
