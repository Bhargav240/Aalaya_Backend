-- Run only against a disposable local Supabase database after the baseline
-- migrations. It proves the database barrier rejects cross-org reads/writes.
BEGIN;
SET LOCAL search_path TO pool, public, extensions;

INSERT INTO public.organizations (id, name) VALUES
 ('00000000-0000-0000-0000-0000000000a1', 'RLS Org A'),
 ('00000000-0000-0000-0000-0000000000b1', 'RLS Org B');

SELECT set_config('app.current_org', '00000000-0000-0000-0000-0000000000a1', true);
INSERT INTO branches (id, org_id, name) VALUES
 ('00000000-0000-0000-0000-0000000000a2', '00000000-0000-0000-0000-0000000000a1', 'A');

-- postgres/service roles bypass RLS. Test through the ordinary authenticated
-- role, granting only within this transaction so no client access survives.
GRANT USAGE ON SCHEMA pool TO authenticated;
GRANT SELECT, INSERT ON pool.branches TO authenticated;
SET LOCAL ROLE authenticated;
SELECT set_config('app.current_org', '00000000-0000-0000-0000-0000000000b1', true);
DO $$ BEGIN
  BEGIN
    INSERT INTO branches (id, org_id, name) VALUES
      ('00000000-0000-0000-0000-0000000000b2', '00000000-0000-0000-0000-0000000000a1', 'must fail');
    RAISE EXCEPTION 'cross-org INSERT unexpectedly passed RLS';
  EXCEPTION WHEN insufficient_privilege THEN NULL;
  END;
END $$;

DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM branches WHERE id = '00000000-0000-0000-0000-0000000000a2') THEN
    RAISE EXCEPTION 'cross-org SELECT unexpectedly passed RLS';
  END IF;
END $$;
RESET ROLE;
ROLLBACK;
