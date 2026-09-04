CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS public.digilocker_oauth_sessions (
    id uuid PRIMARY KEY,
    invitation_id uuid NOT NULL REFERENCES public.tenant_invitations(id),
    user_id uuid NOT NULL REFERENCES public.platform_users(id),
    state_hash text NOT NULL UNIQUE,
    nonce_hash text NOT NULL,
    pkce_verifier_ciphertext bytea NOT NULL,
    expires_at timestamptz NOT NULL,
    consumed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS digilocker_sessions_expiry_idx
    ON public.digilocker_oauth_sessions (expires_at)
    WHERE consumed_at IS NULL;

CREATE TABLE IF NOT EXISTS public.invitation_identity_verifications (
    id uuid PRIMARY KEY,
    invitation_id uuid NOT NULL REFERENCES public.tenant_invitations(id),
    provider text NOT NULL,
    provider_reference text,
    document_type text,
    status text NOT NULL CHECK (status IN ('verified', 'failed')),
    verified_at timestamptz,
    safe_failure_reason text,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS invitation_identity_verification_idx
    ON public.invitation_identity_verifications (invitation_id, created_at DESC);

CREATE TABLE IF NOT EXISTS public.manual_kyc_reviews (
    id uuid PRIMARY KEY,
    invitation_id uuid NOT NULL REFERENCES public.tenant_invitations(id),
    reviewer_user_id uuid NOT NULL REFERENCES public.platform_users(id),
    decision text NOT NULL CHECK (decision IN ('approved', 'rejected')),
    reason text NOT NULL,
    request_id uuid,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS manual_kyc_reviews_invitation_idx
    ON public.manual_kyc_reviews (invitation_id, created_at DESC);
