-- Supabase baseline: control-plane data. The Rust backend remains the sole
-- application client during this phase; Android does not query these tables.
-- Mirrors migrations/control_plane/0001_init.sql.

CREATE TABLE IF NOT EXISTS public.organizations (
    id uuid PRIMARY KEY, name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS public.platform_users (
    id uuid PRIMARY KEY, org_id uuid NOT NULL REFERENCES public.organizations(id),
    phone text NOT NULL, full_name text, verified boolean NOT NULL DEFAULT false,
    status text NOT NULL DEFAULT 'active', google_sub text,
    created_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz
);
CREATE UNIQUE INDEX IF NOT EXISTS platform_users_phone_uidx ON public.platform_users
  (right(regexp_replace(phone, '[^0-9]', '', 'g'), 10)) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS platform_users_google_sub_uidx ON public.platform_users
  (google_sub) WHERE google_sub IS NOT NULL AND deleted_at IS NULL;
CREATE TABLE IF NOT EXISTS public.user_org_roles (
    user_id uuid NOT NULL REFERENCES public.platform_users(id),
    org_id uuid NOT NULL REFERENCES public.organizations(id), role text NOT NULL,
    branch_id uuid, PRIMARY KEY (user_id, org_id, role)
);
CREATE TABLE IF NOT EXISTS public.tenant_placement (
    org_id uuid PRIMARY KEY REFERENCES public.organizations(id),
    mode text NOT NULL DEFAULT 'pooled', schema_name text NOT NULL DEFAULT 'pool',
    status text NOT NULL DEFAULT 'active', updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS public.receipt_templates (
    org_id uuid PRIMARY KEY REFERENCES public.organizations(id), display_name text NOT NULL,
    address text, footer text, accent text, logo_key text, updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS public.verifications (
    id uuid PRIMARY KEY, user_id uuid NOT NULL REFERENCES public.platform_users(id),
    org_id uuid NOT NULL REFERENCES public.organizations(id), method text NOT NULL, status text NOT NULL,
    verified_name text, phone text, emergency_contact text, reason text,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS public.public_listings (
    branch_id uuid PRIMARY KEY, org_id uuid NOT NULL REFERENCES public.organizations(id),
    title text NOT NULL, area text, city text, lat double precision, lng double precision,
    description text, amenities text, status text NOT NULL DEFAULT 'draft', rejection_note text,
    review_request jsonb, published_at timestamptz, created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS public_listings_city_idx ON public.public_listings (city) WHERE status = 'live';
CREATE INDEX IF NOT EXISTS public_listings_status_idx ON public.public_listings (status);
CREATE TABLE IF NOT EXISTS public.listing_vacancy (
    branch_id uuid PRIMARY KEY, org_id uuid NOT NULL REFERENCES public.organizations(id),
    beds_total integer NOT NULL DEFAULT 0, beds_available integer NOT NULL DEFAULT 0,
    starting_price_paise bigint, updated_at timestamptz NOT NULL DEFAULT now(), stale boolean NOT NULL DEFAULT false
);
CREATE TABLE IF NOT EXISTS public.enquiries (
    id uuid PRIMARY KEY, org_id uuid NOT NULL REFERENCES public.organizations(id), branch_id uuid NOT NULL,
    prospect_id uuid NOT NULL REFERENCES public.platform_users(id), phone text NOT NULL, note text,
    status text NOT NULL DEFAULT 'new', created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS enquiries_branch_idx ON public.enquiries (branch_id);
CREATE INDEX IF NOT EXISTS enquiries_org_idx ON public.enquiries (org_id);
CREATE TABLE IF NOT EXISTS public.schema_migrations_tenant (
    schema_name text NOT NULL, version text NOT NULL, applied_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (schema_name, version)
);
CREATE TABLE IF NOT EXISTS public.refresh_tokens (
    token_hash text PRIMARY KEY, user_id uuid NOT NULL REFERENCES public.platform_users(id),
    org_id uuid NOT NULL, expires_at timestamptz NOT NULL, created_at timestamptz NOT NULL DEFAULT now(), revoked_at timestamptz
);
CREATE INDEX IF NOT EXISTS refresh_tokens_user_idx ON public.refresh_tokens (user_id);
CREATE TABLE IF NOT EXISTS public.otp_codes (
    id uuid PRIMARY KEY, user_id uuid REFERENCES public.platform_users(id), phone text NOT NULL,
    code_hash text NOT NULL, attempts integer NOT NULL DEFAULT 0, consumed_at timestamptz,
    expires_at timestamptz NOT NULL, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS otp_codes_phone_idx ON public.otp_codes (phone);
CREATE TABLE IF NOT EXISTS public.device_tokens (
    id uuid PRIMARY KEY, user_id uuid NOT NULL REFERENCES public.platform_users(id), org_id uuid NOT NULL,
    token text NOT NULL, platform text NOT NULL DEFAULT 'android', created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(), UNIQUE (user_id, token)
);
CREATE INDEX IF NOT EXISTS device_tokens_user_idx ON public.device_tokens (user_id);
CREATE TABLE IF NOT EXISTS public.trusted_devices (
    id uuid PRIMARY KEY, user_id uuid NOT NULL REFERENCES public.platform_users(id), device_id text NOT NULL,
    name text, created_at timestamptz NOT NULL DEFAULT now(), last_seen_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (user_id, device_id)
);
CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA extensions;
CREATE INDEX IF NOT EXISTS public_listings_search_trgm ON public.public_listings USING gin
  ((title || ' ' || COALESCE(area,'') || ' ' || COALESCE(city,'') || ' ' || COALESCE(description,'')) extensions.gin_trgm_ops);
CREATE INDEX IF NOT EXISTS public_listings_geo_bbox_idx ON public.public_listings (lat, lng) WHERE status = 'live';
CREATE INDEX IF NOT EXISTS otp_codes_expiry_idx ON public.otp_codes (expires_at);
CREATE INDEX IF NOT EXISTS refresh_tokens_expiry_idx ON public.refresh_tokens (expires_at);
CREATE INDEX IF NOT EXISTS enquiries_recent_idx ON public.enquiries (org_id, created_at DESC);

-- Nothing in public is exposed directly through Supabase Data API in phase 1.
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM anon, authenticated;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM anon, authenticated;
ALTER DEFAULT PRIVILEGES IN SCHEMA public REVOKE ALL ON TABLES FROM anon, authenticated;
