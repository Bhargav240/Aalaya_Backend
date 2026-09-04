SET search_path TO pool, public, extensions;

-- Mirrors migrations/tenant/0004_payment_kind.sql. Separates a security
-- deposit from rent so that `accrued - SUM(payments)` cannot be satisfied by
-- deposit money the resident is owed back.
ALTER TABLE payments
    ADD COLUMN IF NOT EXISTS kind text NOT NULL DEFAULT 'rent';

ALTER TABLE payments DROP CONSTRAINT IF EXISTS payments_kind_check;
ALTER TABLE payments
    ADD CONSTRAINT payments_kind_check CHECK (kind IN ('rent', 'deposit'));

CREATE INDEX IF NOT EXISTS payments_org_resident_kind_idx
    ON payments (org_id, resident_id, kind) WHERE deleted_at IS NULL;
