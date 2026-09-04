-- Invoice numbering, per organisation.
--
-- invoice_sequences was keyed by period alone, so every org in the pooled
-- schema drew from one counter: a single PG's invoices came out gappy
-- (001, 004, 009...) as other orgs interleaved, and the 999-per-month ceiling
-- was shared between them rather than being each PG's own.

ALTER TABLE invoice_sequences ADD COLUMN IF NOT EXISTS org_id uuid;
ALTER TABLE invoice_sequences DROP CONSTRAINT IF EXISTS invoice_sequences_pkey;

-- The old shared counters cannot be split apart after the fact, so they are
-- replaced by per-org counters derived from what each org has actually issued.
-- Starting at max+1 guarantees no org can re-mint a number already printed on
-- a receipt and sitting in a resident's PDF.
DELETE FROM invoice_sequences WHERE org_id IS NULL;

INSERT INTO invoice_sequences (org_id, period, next_number)
SELECT r.org_id,
       split_part(r.invoice_number, '-', 2),
       MAX(split_part(r.invoice_number, '-', 3)::int) + 1
FROM receipts r
WHERE r.invoice_number LIKE 'INV-%'
GROUP BY 1, 2;

ALTER TABLE invoice_sequences ALTER COLUMN org_id SET NOT NULL;
ALTER TABLE invoice_sequences ADD PRIMARY KEY (org_id, period);

-- Two PGs may now legitimately both hold INV-2609-001, so uniqueness has to be
-- per org. Without this the second PG to collect in a month fails on a
-- duplicate key.
DROP INDEX IF EXISTS receipts_invoice_number_uidx;
CREATE UNIQUE INDEX IF NOT EXISTS receipts_org_invoice_number_uidx
    ON receipts (org_id, invoice_number) WHERE invoice_number IS NOT NULL;

-- Now that it carries org_id, it gets the isolation every other tenant table
-- has. Enabled last: FORCE applies to the owner too, and the backfill above
-- runs without app.current_org set.
ALTER TABLE invoice_sequences ENABLE ROW LEVEL SECURITY;
ALTER TABLE invoice_sequences FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS invoice_sequences_org_isolation ON invoice_sequences;
CREATE POLICY invoice_sequences_org_isolation ON invoice_sequences
    USING (org_id = current_setting('app.current_org', true)::uuid)
    WITH CHECK (org_id = current_setting('app.current_org', true)::uuid);
