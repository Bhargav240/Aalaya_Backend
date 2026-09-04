ALTER TABLE public.tenant_invitations
    ADD COLUMN IF NOT EXISTS last_sent_at timestamptz;

UPDATE public.tenant_invitations
SET last_sent_at = COALESCE(last_sent_at, created_at);

ALTER TABLE public.tenant_invitations
    ALTER COLUMN last_sent_at SET DEFAULT now(),
    ALTER COLUMN last_sent_at SET NOT NULL;
