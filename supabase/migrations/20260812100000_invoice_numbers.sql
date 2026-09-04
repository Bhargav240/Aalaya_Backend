SET search_path TO pool, public, extensions;
CREATE TABLE IF NOT EXISTS invoice_sequences (
 period text PRIMARY KEY, next_number integer NOT NULL CHECK (next_number BETWEEN 1 AND 1000)
);
ALTER TABLE receipts ADD COLUMN IF NOT EXISTS invoice_number text;
UPDATE receipts SET invoice_number = receipt_number WHERE invoice_number IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS receipts_invoice_number_uidx ON receipts (invoice_number) WHERE invoice_number IS NOT NULL;
