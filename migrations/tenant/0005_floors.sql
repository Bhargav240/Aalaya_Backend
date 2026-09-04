-- Floors: the level between a branch and its rooms.
--
-- Rooms already implied a floor through their numbering ("201" is on floor 2),
-- and the apps read it that way. That convention cannot carry a name, cannot
-- exist before its first room, and cannot be renamed — so an owner had no way
-- to manage floors as things in their own right. This gives them a table.
--
-- The link from rooms is deliberately NULLable: every room that predates this
-- migration keeps working untouched, and its floor continues to be derived
-- from the room number until someone assigns it explicitly.
CREATE TABLE IF NOT EXISTS floors (
    id          uuid PRIMARY KEY,
    org_id      uuid NOT NULL,
    branch_id   uuid NOT NULL,
    name        text NOT NULL,
    -- Display order. Floors are not reliably sortable by name ("Ground" before
    -- "1st"), so the owner's intended order is stored rather than inferred.
    sort_order  integer NOT NULL DEFAULT 0,
    created_at  timestamptz NOT NULL DEFAULT now(),
    deleted_at  timestamptz
);

CREATE INDEX IF NOT EXISTS floors_org_branch_idx ON floors (org_id, branch_id);

-- One "Floor 2" per branch. Case-insensitive, and only among live rows so a
-- deleted floor's name can be reused.
CREATE UNIQUE INDEX IF NOT EXISTS floors_branch_name_uidx
    ON floors (branch_id, lower(name)) WHERE deleted_at IS NULL;

ALTER TABLE rooms ADD COLUMN IF NOT EXISTS floor_id uuid REFERENCES floors (id);
CREATE INDEX IF NOT EXISTS rooms_floor_idx ON rooms (floor_id) WHERE floor_id IS NOT NULL;

-- Same org isolation every other tenant table carries.
ALTER TABLE floors ENABLE ROW LEVEL SECURITY;
ALTER TABLE floors FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS floors_org_isolation ON floors;
CREATE POLICY floors_org_isolation ON floors
    USING (org_id = current_setting('app.current_org', true)::uuid)
    WITH CHECK (org_id = current_setting('app.current_org', true)::uuid);
