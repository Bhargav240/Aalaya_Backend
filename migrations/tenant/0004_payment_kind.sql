-- Distinguishes a rent payment from a security deposit.
--
-- Every payment was implicitly rent, and the deposit existed only as
-- residents.deposit_paise with no payment or receipt behind it. Recording a
-- deposit without this column would be a money bug: rent dues are computed as
-- `accrued - SUM(payments)`, so a deposit would silently cancel rent the
-- resident has not paid, and would distort their move-out refund too.
--
-- Existing rows are rent by definition — that is all the system could record
-- until now.
ALTER TABLE payments
    ADD COLUMN IF NOT EXISTS kind text NOT NULL DEFAULT 'rent';

ALTER TABLE payments DROP CONSTRAINT IF EXISTS payments_kind_check;
ALTER TABLE payments
    ADD CONSTRAINT payments_kind_check CHECK (kind IN ('rent', 'deposit'));

-- Rent totals filter on this, so it is on the hot path for every dues read.
CREATE INDEX IF NOT EXISTS payments_org_resident_kind_idx
    ON payments (org_id, resident_id, kind) WHERE deleted_at IS NULL;
