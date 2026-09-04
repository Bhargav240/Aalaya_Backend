-- Verifies pooled-schema routing and the RLS tenant boundary without leaving
-- data behind. Run through a privileged maintenance connection; the test
-- temporarily assumes the ordinary authenticated application role.
BEGIN;

GRANT USAGE ON SCHEMA pool TO authenticated;
GRANT SELECT, INSERT ON pool.branches, pool.rooms TO authenticated;
SET LOCAL ROLE authenticated;
SET LOCAL search_path TO pool, public, extensions;

-- Existing Supabase Development PG (Org A) contains the E2E room fixtures.
SELECT set_config('app.current_org', '32f33aef-dc6a-4caf-9f05-794f65f30c8a', true);
SELECT count(*) AS org_a_rooms_visible
FROM rooms
WHERE org_id = '32f33aef-dc6a-4caf-9f05-794f65f30c8a';

-- A different tenant context must not see Org A's fixture rooms, even when it
-- supplies Org A's id in the query.
SELECT set_config('app.current_org', '11111111-2222-3333-4444-555555555555', true);
SELECT count(*) AS org_a_rooms_visible_from_org_b_context
FROM rooms
WHERE org_id = '32f33aef-dc6a-4caf-9f05-794f65f30c8a';

-- Insert a temporary Org-B-scoped branch and room, then prove Org A cannot
-- read it. These rows are rolled back at the end of the test.
INSERT INTO branches (id, org_id, name)
VALUES ('11111111-2222-3333-4444-555555555556', '11111111-2222-3333-4444-555555555555', 'Routing QA Branch');
INSERT INTO rooms (id, org_id, branch_id, number, cooling_type)
VALUES ('11111111-2222-3333-4444-555555555557', '11111111-2222-3333-4444-555555555555',
        '11111111-2222-3333-4444-555555555556', '999', 'non_ac');
SELECT count(*) AS org_b_rooms_visible
FROM rooms
WHERE org_id = '11111111-2222-3333-4444-555555555555';

SELECT set_config('app.current_org', '32f33aef-dc6a-4caf-9f05-794f65f30c8a', true);
SELECT count(*) AS org_b_rooms_visible_from_org_a_context
FROM rooms
WHERE org_id = '11111111-2222-3333-4444-555555555555';

RESET ROLE;
ROLLBACK;
