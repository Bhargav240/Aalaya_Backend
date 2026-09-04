-- Email is the authentication identity. Phone remains optional contact data.
ALTER TABLE public.platform_users ALTER COLUMN phone DROP NOT NULL;
ALTER TABLE public.platform_users ADD COLUMN IF NOT EXISTS email text;
ALTER TABLE public.platform_users ADD COLUMN IF NOT EXISTS email_verified_at timestamptz;
CREATE UNIQUE INDEX IF NOT EXISTS platform_users_email_uidx
    ON public.platform_users (lower(email))
    WHERE email IS NOT NULL AND deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS public.email_otps (
    id           uuid PRIMARY KEY,
    user_id      uuid NOT NULL REFERENCES public.platform_users(id),
    email        text NOT NULL,
    purpose      text NOT NULL DEFAULT 'login',
    code_hash    text NOT NULL,
    attempts     integer NOT NULL DEFAULT 0,
    consumed_at  timestamptz,
    expires_at   timestamptz NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS email_otps_email_created_idx
    ON public.email_otps (lower(email), created_at DESC);
CREATE INDEX IF NOT EXISTS email_otps_expiry_idx ON public.email_otps (expires_at);
