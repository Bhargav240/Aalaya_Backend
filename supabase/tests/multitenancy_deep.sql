-- Comprehensive shared-schema multi-tenancy verification.
--
-- Run with a privileged maintenance connection.  The entire fixture setup and
-- the temporary grants are rolled back.  This intentionally assumes the
-- ordinary `authenticated` role while exercising RLS, rather than testing as
-- postgres/service_role (which can bypass policies).
BEGIN;
SET LOCAL search_path TO pool, public, extensions;

DO $$
DECLARE
  table_name text;
  protected_tables constant text[] := ARRAY[
    'branches', 'rooms', 'beds', 'residents', 'payments', 'receipts',
    'change_requests', 'audit_log', 'complaints', 'complaint_events',
    'notices', 'archive_manifest', 'away_periods', 'move_out_settlements',
    'move_out_deductions'
  ];
BEGIN
  FOREACH table_name IN ARRAY protected_tables LOOP
    IF NOT EXISTS (
      SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
      WHERE n.nspname = 'pool' AND c.relname = table_name
        AND c.relrowsecurity AND c.relforcerowsecurity
    ) THEN
      RAISE EXCEPTION 'pool.% must have enabled and forced RLS', table_name;
    END IF;
    IF has_table_privilege('authenticated', format('pool.%I', table_name), 'SELECT')
       OR has_table_privilege('anon', format('pool.%I', table_name), 'SELECT') THEN
      RAISE EXCEPTION 'client role unexpectedly has persistent SELECT on pool.%', table_name;
    END IF;
  END LOOP;
END $$;

-- Control-plane rows are only fixtures.  ROLLBACK removes them.
INSERT INTO public.organizations (id, name) VALUES
 ('10000000-0000-0000-0000-0000000000a1', 'Deep QA Org A'),
 ('20000000-0000-0000-0000-0000000000b1', 'Deep QA Org B');

-- These grants exist only for this transaction. Production client roles retain
-- no direct access to the private pool schema.
GRANT USAGE ON SCHEMA pool TO authenticated;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA pool TO authenticated;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA pool TO authenticated;
SET LOCAL ROLE authenticated;

-- Seed a complete Org-A graph through the ordinary client role.
SELECT set_config('app.current_org', '10000000-0000-0000-0000-0000000000a1', true);
INSERT INTO branches (id, org_id, name) VALUES
 ('10000000-0000-0000-0000-0000000000a2', '10000000-0000-0000-0000-0000000000a1', 'A branch');
INSERT INTO rooms (id, org_id, branch_id, number, cooling_type) VALUES
 ('10000000-0000-0000-0000-0000000000a3', '10000000-0000-0000-0000-0000000000a1', '10000000-0000-0000-0000-0000000000a2', 'A-101', 'ac');
INSERT INTO beds (id, org_id, room_id, label) VALUES
 ('10000000-0000-0000-0000-0000000000a4', '10000000-0000-0000-0000-0000000000a1', '10000000-0000-0000-0000-0000000000a3', 'A-101-1');
INSERT INTO residents (id, org_id, branch_id, bed_id, full_name, status) VALUES
 ('10000000-0000-0000-0000-0000000000a5', '10000000-0000-0000-0000-0000000000a1', '10000000-0000-0000-0000-0000000000a2', '10000000-0000-0000-0000-0000000000a4', 'Org A resident', 'active');
INSERT INTO payments (id, org_id, resident_id, amount_paise) VALUES
 ('10000000-0000-0000-0000-0000000000a6', '10000000-0000-0000-0000-0000000000a1', '10000000-0000-0000-0000-0000000000a5', 10000);
INSERT INTO receipts (id, org_id, resident_id, payment_id, receipt_number, amount_paise) VALUES
 ('10000000-0000-0000-0000-0000000000a7', '10000000-0000-0000-0000-0000000000a1', '10000000-0000-0000-0000-0000000000a5', '10000000-0000-0000-0000-0000000000a6', 'DEEP-A-1', 10000);
INSERT INTO change_requests (id, org_id, entity_type, entity_id, operation, requested_by) VALUES
 ('10000000-0000-0000-0000-0000000000a8', '10000000-0000-0000-0000-0000000000a1', 'room', '10000000-0000-0000-0000-0000000000a3', 'update', '10000000-0000-0000-0000-0000000000a5');
INSERT INTO audit_log (org_id, action) VALUES ('10000000-0000-0000-0000-0000000000a1', 'deep_qa');
INSERT INTO complaints (id, org_id, branch_id, raised_by, title) VALUES
 ('10000000-0000-0000-0000-0000000000a9', '10000000-0000-0000-0000-0000000000a1', '10000000-0000-0000-0000-0000000000a2', '10000000-0000-0000-0000-0000000000a5', 'Deep QA');
INSERT INTO complaint_events (org_id, complaint_id, to_state) VALUES
 ('10000000-0000-0000-0000-0000000000a1', '10000000-0000-0000-0000-0000000000a9', 'open');
INSERT INTO notices (id, org_id, branch_id, title) VALUES
 ('10000000-0000-0000-0000-0000000000aa', '10000000-0000-0000-0000-0000000000a1', '10000000-0000-0000-0000-0000000000a2', 'Deep QA');
INSERT INTO archive_manifest (id, org_id, table_name, object_key, row_count) VALUES
 ('10000000-0000-0000-0000-0000000000ab', '10000000-0000-0000-0000-0000000000a1', 'rooms', 'deep-qa', 1);
INSERT INTO away_periods (id, org_id, resident_id, from_date, to_date) VALUES
 ('10000000-0000-0000-0000-0000000000ac', '10000000-0000-0000-0000-0000000000a1', '10000000-0000-0000-0000-0000000000a5', CURRENT_DATE, CURRENT_DATE);
INSERT INTO move_out_settlements (id, org_id, resident_id, last_day, notice_days, rent_to_date_paise, paid_total_paise, outstanding_paise, deposit_held_paise, deductions_total_paise, refund_paise, forfeited) VALUES
 ('10000000-0000-0000-0000-0000000000ad', '10000000-0000-0000-0000-0000000000a1', '10000000-0000-0000-0000-0000000000a5', CURRENT_DATE, 0, 0, 0, 0, 0, 0, 0, false);
INSERT INTO move_out_deductions (id, settlement_id, reason, amount_paise) VALUES
 ('10000000-0000-0000-0000-0000000000ae', '10000000-0000-0000-0000-0000000000ad', 'Deep QA', 0);

-- Org B must see none of Org A's rows, even if it explicitly filters by A's id.
SELECT set_config('app.current_org', '20000000-0000-0000-0000-0000000000b1', true);
DO $$
DECLARE table_name text; visible_count bigint;
BEGIN
  FOREACH table_name IN ARRAY ARRAY[
    'branches', 'rooms', 'beds', 'residents', 'payments', 'receipts',
    'change_requests', 'audit_log', 'complaints', 'complaint_events',
    'notices', 'archive_manifest', 'away_periods', 'move_out_settlements'
  ] LOOP
    EXECUTE format('SELECT count(*) FROM %I WHERE org_id = $1', table_name)
      INTO visible_count USING '10000000-0000-0000-0000-0000000000a1'::uuid;
    IF visible_count <> 0 THEN
      RAISE EXCEPTION 'Org B can read % Org-A row(s) from pool.%', visible_count, table_name;
    END IF;
  END LOOP;
  SELECT count(*) INTO visible_count FROM move_out_deductions
   WHERE settlement_id = '10000000-0000-0000-0000-0000000000ad';
  IF visible_count <> 0 THEN
    RAISE EXCEPTION 'Org B can read Org-A move_out_deductions';
  END IF;
END $$;

-- RLS filters cross-org mutations, and rejects a forged org_id on INSERT.
DO $$
DECLARE changed_count bigint;
BEGIN
  UPDATE rooms SET number = 'tampered' WHERE id = '10000000-0000-0000-0000-0000000000a3';
  GET DIAGNOSTICS changed_count = ROW_COUNT;
  IF changed_count <> 0 THEN RAISE EXCEPTION 'cross-org UPDATE unexpectedly changed rows'; END IF;
  DELETE FROM rooms WHERE id = '10000000-0000-0000-0000-0000000000a3';
  GET DIAGNOSTICS changed_count = ROW_COUNT;
  IF changed_count <> 0 THEN RAISE EXCEPTION 'cross-org DELETE unexpectedly changed rows'; END IF;
  BEGIN
    INSERT INTO branches (id, org_id, name) VALUES
      ('20000000-0000-0000-0000-0000000000b2', '10000000-0000-0000-0000-0000000000a1', 'forged org');
    RAISE EXCEPTION 'forged cross-org INSERT unexpectedly passed RLS';
  EXCEPTION WHEN insufficient_privilege THEN NULL;
  END;
  BEGIN
    INSERT INTO move_out_deductions (id, settlement_id, reason, amount_paise) VALUES
      ('20000000-0000-0000-0000-0000000000b3', '10000000-0000-0000-0000-0000000000ad', 'forged transitive row', 0);
    RAISE EXCEPTION 'cross-org move_out_deductions INSERT unexpectedly passed RLS';
  EXCEPTION WHEN insufficient_privilege THEN NULL;
  END;
END $$;

RESET ROLE;
ROLLBACK;

-- `SET LOCAL` must not leak an organization id when this connection is reused.
BEGIN;
DO $$
BEGIN
  -- PostgreSQL represents a reset custom setting as either NULL or an empty
  -- string, depending on how the session first encountered that setting.
  IF COALESCE(current_setting('app.current_org', true), '') <> '' THEN
    RAISE EXCEPTION 'app.current_org leaked past its tenant transaction';
  END IF;
END $$;
ROLLBACK;
