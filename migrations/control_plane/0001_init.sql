-- Control plane & routing. Lives in the `public` schema, which is always on the
-- search_path -- so the resolver can read it before any tenant context exists.
--
-- Revision 3 model: phone is THE identity. No passwords, no invite codes.
-- Registration (by a manager/owner, or by enquiring on the marketplace) is the
-- invitation: once a phone is registered, an SMS code signs it in.

CREATE TABLE IF NOT EXISTS public.organizations (
    id          uuid PRIMARY KEY,
    name        text NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    deleted_at  timestamptz
);

-- NOTE: `platform_users.org_id` is INSERT-SIDE PROVENANCE only — never read it
-- for auth/RBAC. Trust `user_org_roles.org_id` exclusively (B12: dual-truth fix).
CREATE TABLE IF NOT EXISTS public.platform_users (
    id            uuid PRIMARY KEY,
    org_id        uuid NOT NULL REFERENCES public.organizations(id),
    phone         text NOT NULL,
    full_name     text,
    verified      boolean NOT NULL DEFAULT false,
    status        text NOT NULL DEFAULT 'active',   -- active | suspended
    google_sub    text,
    created_at    timestamptz NOT NULL DEFAULT now(),
    deleted_at    timestamptz
);

CREATE UNIQUE INDEX IF NOT EXISTS platform_users_phone_uidx
    ON public.platform_users (right(regexp_replace(phone, '[^0-9]', '', 'g'), 10))
    WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS platform_users_google_sub_uidx
    ON public.platform_users (google_sub) WHERE google_sub IS NOT NULL AND deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS public.user_org_roles (
    user_id    uuid NOT NULL REFERENCES public.platform_users(id),
    org_id     uuid NOT NULL REFERENCES public.organizations(id),
    role       text NOT NULL,          -- platform_operator | owner | manager | tenant
    branch_id  uuid,                   -- set for managers
    PRIMARY KEY (user_id, org_id, role)
);

-- The routing table the resolver reads (behind a cache) on every request.
CREATE TABLE IF NOT EXISTS public.tenant_placement (
    org_id       uuid PRIMARY KEY REFERENCES public.organizations(id),
    mode         text NOT NULL DEFAULT 'pooled',   -- pooled | siloed
    schema_name  text NOT NULL DEFAULT 'pool',
    status       text NOT NULL DEFAULT 'active',
    updated_at   timestamptz NOT NULL DEFAULT now()
);

-- Per-org receipt branding template (control plane). Default used when absent.
CREATE TABLE IF NOT EXISTS public.receipt_templates (
    org_id        uuid PRIMARY KEY REFERENCES public.organizations(id),
    display_name  text NOT NULL,
    address       text,
    footer        text,
    accent        text,
    logo_key      text,
    updated_at    timestamptz NOT NULL DEFAULT now()
);

-- Identity verification records (control plane). One row per attempt.
CREATE TABLE IF NOT EXISTS public.verifications (
    id                 uuid PRIMARY KEY,
    user_id            uuid NOT NULL REFERENCES public.platform_users(id),
    org_id             uuid NOT NULL REFERENCES public.organizations(id),
    method             text NOT NULL,
    status             text NOT NULL,          -- verified | failed
    verified_name      text,
    phone              text,
    emergency_contact  text,
    reason             text,
    created_at         timestamptz NOT NULL DEFAULT now()
);

-- Marketplace listings (marketing projection). Publishing is gated by the
-- Platform Operator: owners/managers PREPARE (draft -> pending), only the
-- operator publishes or requests changes.
CREATE TABLE IF NOT EXISTS public.public_listings (
    branch_id   uuid PRIMARY KEY,
    org_id      uuid NOT NULL REFERENCES public.organizations(id),
    title       text NOT NULL,
    area        text,
    city        text,
    lat         double precision,
    lng         double precision,
    description text,
    amenities   text,
    status      text NOT NULL DEFAULT 'draft',   -- draft | pending | approved | live | rejected | unlisted
    rejection_note text,
    review_request jsonb,                         -- operator's requested edits/changes
    published_at timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS public_listings_city_idx ON public.public_listings (city) WHERE status = 'live';
CREATE INDEX IF NOT EXISTS public_listings_status_idx ON public.public_listings (status);

-- Live vacancy snapshot per listing's branch, DERIVED from tenant bed data
-- by the vacancy projector (never hand-typed). Discovery reads this.
CREATE TABLE IF NOT EXISTS public.listing_vacancy (
    branch_id           uuid PRIMARY KEY,
    org_id              uuid NOT NULL REFERENCES public.organizations(id),
    beds_total          integer NOT NULL DEFAULT 0,
    beds_available      integer NOT NULL DEFAULT 0,
    starting_price_paise bigint,
    updated_at          timestamptz NOT NULL DEFAULT now(),
    stale               boolean NOT NULL DEFAULT false
);

-- Marketplace enquiries: an anonymous seeker's phone + OTP becomes a Prospect
-- (a light platform user) and the lead lands in the branch manager's inbox.
CREATE TABLE IF NOT EXISTS public.enquiries (
    id           uuid PRIMARY KEY,
    org_id       uuid NOT NULL REFERENCES public.organizations(id),
    branch_id    uuid NOT NULL,
    prospect_id  uuid NOT NULL REFERENCES public.platform_users(id),
    phone        text NOT NULL,
    note         text,
    status       text NOT NULL DEFAULT 'new',    -- new | contacted | closed
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS enquiries_branch_idx ON public.enquiries (branch_id);
CREATE INDEX IF NOT EXISTS enquiries_org_idx ON public.enquiries (org_id);

CREATE TABLE IF NOT EXISTS public.schema_migrations_tenant (
    schema_name  text NOT NULL,
    version      text NOT NULL,
    applied_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (schema_name, version)
);

-- Refresh-token store: rotation + revocation. Only the SHA-256 hash is kept.
CREATE TABLE IF NOT EXISTS public.refresh_tokens (
    token_hash  text PRIMARY KEY,
    user_id     uuid NOT NULL REFERENCES public.platform_users(id),
    org_id      uuid NOT NULL,
    expires_at  timestamptz NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    revoked_at  timestamptz
);
CREATE INDEX IF NOT EXISTS refresh_tokens_user_idx ON public.refresh_tokens (user_id);

-- Phone+OTP sign-in challenges. Only the SHA-256 hash of the code is kept.
-- user_id is nullable for prospect challenges: a seeker is not a platform user
-- until their enquiry code verifies.
CREATE TABLE IF NOT EXISTS public.otp_codes (
    id           uuid PRIMARY KEY,
    user_id      uuid REFERENCES public.platform_users(id),
    phone        text NOT NULL,
    code_hash    text NOT NULL,
    attempts     integer NOT NULL DEFAULT 0,
    consumed_at  timestamptz,
    expires_at   timestamptz NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS otp_codes_phone_idx ON public.otp_codes (phone);

-- Push notification device registration (FCM).
CREATE TABLE IF NOT EXISTS public.device_tokens (
    id           uuid PRIMARY KEY,
    user_id      uuid NOT NULL REFERENCES public.platform_users(id),
    org_id       uuid NOT NULL,
    token        text NOT NULL,
    platform     text NOT NULL DEFAULT 'android',
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    UNIQUE (user_id, token)
);
CREATE INDEX IF NOT EXISTS device_tokens_user_idx ON public.device_tokens (user_id);

-- Trusted devices: hardware the user has confirmed once. A sign-in from
-- unknown hardware flips is_new_device on the login response so the app can
-- ask for an extra confirmation.
CREATE TABLE IF NOT EXISTS public.trusted_devices (
    id          uuid PRIMARY KEY,
    user_id     uuid NOT NULL REFERENCES public.platform_users(id),
    device_id   text NOT NULL,
    name        text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (user_id, device_id)
);

-- Full-text search on listings (B23). Trigram GIN index, safe on empty/missing rows.
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS public_listings_search_trgm
    ON public.public_listings USING gin (
        (title || ' ' || COALESCE(area,'') || ' ' || COALESCE(city,'') || ' ' || COALESCE(description,'')) gin_trgm_ops
    );

-- Bounding-box pre-filter for /discover/nearby (approximate; haversine still re-checks).
CREATE INDEX IF NOT EXISTS public_listings_geo_bbox_idx
    ON public.public_listings (lat, lng) WHERE status = 'live';

-- Cleanup scans (B18).
CREATE INDEX IF NOT EXISTS otp_codes_expiry_idx    ON public.otp_codes (expires_at);
CREATE INDEX IF NOT EXISTS refresh_tokens_expiry_idx ON public.refresh_tokens (expires_at);
CREATE INDEX IF NOT EXISTS enquiries_recent_idx    ON public.enquiries (org_id, created_at DESC);
