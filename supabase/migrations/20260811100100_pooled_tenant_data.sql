-- Supabase baseline: operational data in the existing shared tenant schema.
-- The Rust backend sets both search_path and app.current_org in every tenant
-- transaction. Keep this schema out of the Data API exposed-schema list.

CREATE SCHEMA IF NOT EXISTS pool;
SET LOCAL search_path TO pool, public, extensions;

CREATE TABLE IF NOT EXISTS branches (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, name text NOT NULL, address text,
 created_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS rooms (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, branch_id uuid NOT NULL, number text NOT NULL,
 cooling_type text NOT NULL DEFAULT 'non_ac' CHECK (cooling_type IN ('ac', 'non_ac')),
 created_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS beds (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, room_id uuid NOT NULL, label text NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS residents (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, branch_id uuid NOT NULL, bed_id uuid,
 full_name text NOT NULL, phone text, plan_kind text NOT NULL DEFAULT 'monthly',
 status text NOT NULL DEFAULT 'invited', rent_paise bigint NOT NULL DEFAULT 0,
 deposit_paise bigint NOT NULL DEFAULT 0, user_id uuid, moved_out_at timestamptz,
 created_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS payments (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, resident_id uuid NOT NULL, amount_paise bigint NOT NULL,
 paid_at timestamptz NOT NULL DEFAULT now(), recorded_by uuid, method text NOT NULL DEFAULT 'cash',
 status text NOT NULL DEFAULT 'recorded', upi_ref text, reconciled_at timestamptz,
 idempotency_key text, created_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS receipts (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, resident_id uuid NOT NULL, payment_id uuid NOT NULL,
 receipt_number text NOT NULL, amount_paise bigint NOT NULL, storage_key text,
 status text NOT NULL DEFAULT 'pending', issued_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS invoice_sequences (
 period text PRIMARY KEY, next_number integer NOT NULL CHECK (next_number BETWEEN 1 AND 1000)
);
ALTER TABLE receipts ADD COLUMN IF NOT EXISTS invoice_number text;
UPDATE receipts SET invoice_number = receipt_number WHERE invoice_number IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS receipts_invoice_number_uidx ON receipts (invoice_number) WHERE invoice_number IS NOT NULL;
CREATE TABLE IF NOT EXISTS change_requests (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, entity_type text NOT NULL, entity_id uuid NOT NULL,
 operation text NOT NULL, proposed jsonb, requested_by uuid NOT NULL, status text NOT NULL DEFAULT 'pending',
 created_at timestamptz NOT NULL DEFAULT now(), decided_at timestamptz, decided_by uuid
);
CREATE TABLE IF NOT EXISTS audit_log (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, org_id uuid NOT NULL, actor_id uuid,
 action text NOT NULL, entity_type text, entity_id uuid, before jsonb, after jsonb, request_id uuid,
 created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS complaints (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, branch_id uuid, raised_by uuid NOT NULL, title text NOT NULL,
 description text, state text NOT NULL DEFAULT 'open', is_business_level boolean NOT NULL DEFAULT false,
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS complaint_events (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY, org_id uuid NOT NULL, complaint_id uuid NOT NULL,
 from_state text, to_state text NOT NULL, by_user uuid, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS notices (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, branch_id uuid, title text NOT NULL, body text,
 created_by uuid, created_at timestamptz NOT NULL DEFAULT now(), deleted_at timestamptz
);
CREATE TABLE IF NOT EXISTS archive_manifest (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, table_name text NOT NULL, object_key text NOT NULL,
 row_count bigint NOT NULL, archived_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS away_periods (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, resident_id uuid NOT NULL, from_date date NOT NULL,
 to_date date NOT NULL, note text, status text NOT NULL DEFAULT 'active',
 created_at timestamptz NOT NULL DEFAULT now(), cancelled_at timestamptz, deleted_at timestamptz,
 CONSTRAINT away_periods_range_ok CHECK (to_date >= from_date)
);
CREATE TABLE IF NOT EXISTS move_out_settlements (
 id uuid PRIMARY KEY, org_id uuid NOT NULL, resident_id uuid NOT NULL, last_day date NOT NULL,
 notice_days integer NOT NULL, rent_to_date_paise bigint NOT NULL, paid_total_paise bigint NOT NULL,
 outstanding_paise bigint NOT NULL, deposit_held_paise bigint NOT NULL,
 deductions_total_paise bigint NOT NULL, refund_paise bigint NOT NULL, forfeited boolean NOT NULL,
 note text, settled_by uuid, settled_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS move_out_settlements_resident_uidx ON move_out_settlements (resident_id);
CREATE TABLE IF NOT EXISTS move_out_deductions (
 id uuid PRIMARY KEY, settlement_id uuid NOT NULL REFERENCES move_out_settlements(id) ON DELETE CASCADE,
 reason text NOT NULL, amount_paise bigint NOT NULL
);

CREATE INDEX IF NOT EXISTS residents_org_branch_idx ON residents (org_id, branch_id);
CREATE INDEX IF NOT EXISTS residents_org_user_idx ON residents (org_id, user_id);
CREATE INDEX IF NOT EXISTS residents_org_bed_idx ON residents (org_id, bed_id);
CREATE UNIQUE INDEX IF NOT EXISTS residents_one_live_per_bed_uidx ON residents (org_id, bed_id)
 WHERE bed_id IS NOT NULL AND deleted_at IS NULL AND status <> 'moved_out';
CREATE INDEX IF NOT EXISTS residents_accrual_idx ON residents (org_id, branch_id) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS rooms_org_branch_idx ON rooms (org_id, branch_id);
CREATE INDEX IF NOT EXISTS beds_org_room_idx ON beds (org_id, room_id);
CREATE INDEX IF NOT EXISTS beds_org_room_idx2 ON beds (org_id, room_id) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS payments_org_resident_idx ON payments (org_id, resident_id);
CREATE UNIQUE INDEX IF NOT EXISTS payments_idempotency_uidx ON payments (org_id, idempotency_key)
 WHERE idempotency_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS receipts_org_resident_idx ON receipts (org_id, resident_id);
CREATE INDEX IF NOT EXISTS complaints_org_branch_idx ON complaints (org_id, branch_id);
CREATE INDEX IF NOT EXISTS notices_org_branch_idx ON notices (org_id, branch_id);
CREATE INDEX IF NOT EXISTS audit_log_org_id_idx ON audit_log (org_id, id DESC);
CREATE INDEX IF NOT EXISTS change_requests_org_status_idx ON change_requests (org_id, status);
CREATE INDEX IF NOT EXISTS away_periods_org_resident_idx ON away_periods (org_id, resident_id);
CREATE INDEX IF NOT EXISTS move_out_deductions_settlement_idx ON move_out_deductions (settlement_id);

DO $$
DECLARE t text;
BEGIN
 FOREACH t IN ARRAY ARRAY['branches','rooms','beds','residents','payments','receipts','change_requests','audit_log','complaints','complaint_events','notices','archive_manifest','away_periods','move_out_settlements'] LOOP
   EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY', t);
   EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY', t);
   EXECUTE format('CREATE POLICY %I ON %I USING (org_id = current_setting(''app.current_org'', true)::uuid) WITH CHECK (org_id = current_setting(''app.current_org'', true)::uuid)', t || '_org_isolation', t);
 END LOOP;
 ALTER TABLE move_out_deductions ENABLE ROW LEVEL SECURITY;
 ALTER TABLE move_out_deductions FORCE ROW LEVEL SECURITY;
 CREATE POLICY move_out_deductions_org_isolation ON move_out_deductions USING (EXISTS (
   SELECT 1 FROM move_out_settlements s WHERE s.id = settlement_id
   AND s.org_id = current_setting('app.current_org', true)::uuid
 ));
END $$;

-- In phase 1, no client role may access pooled tenant data directly.
REVOKE ALL ON ALL TABLES IN SCHEMA pool FROM anon, authenticated;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA pool FROM anon, authenticated;
ALTER DEFAULT PRIVILEGES IN SCHEMA pool REVOKE ALL ON TABLES FROM anon, authenticated;
