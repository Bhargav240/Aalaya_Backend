-- Development-only E2E fixture for the Supabase Development PG organization.
-- Idempotent: rerunning it does not duplicate active rooms or bed labels.
BEGIN;
SET LOCAL search_path TO pool, public, extensions;
SELECT set_config('app.current_org', '32f33aef-dc6a-4caf-9f05-794f65f30c8a', true);

WITH target_branch AS (
    SELECT id FROM branches
    WHERE org_id = '32f33aef-dc6a-4caf-9f05-794f65f30c8a'
      AND name = 'Main Branch' AND deleted_at IS NULL
    LIMIT 1
), room_specs(number, cooling_type, bed_count) AS (
    VALUES
      ('101', 'non_ac', 2), ('102', 'ac', 3), ('103', 'non_ac', 4),
      ('201', 'ac', 2),     ('202', 'non_ac', 3), ('203', 'ac', 4),
      ('301', 'non_ac', 2), ('302', 'ac', 3), ('303', 'non_ac', 4)
)
INSERT INTO rooms (id, org_id, branch_id, number, cooling_type)
SELECT gen_random_uuid(), '32f33aef-dc6a-4caf-9f05-794f65f30c8a', b.id, s.number, s.cooling_type
FROM target_branch b CROSS JOIN room_specs s
WHERE NOT EXISTS (
    SELECT 1 FROM rooms r WHERE r.org_id = '32f33aef-dc6a-4caf-9f05-794f65f30c8a'
      AND r.branch_id = b.id AND r.number = s.number AND r.deleted_at IS NULL
);

WITH room_specs(number, bed_count) AS (
    VALUES ('101', 2), ('102', 3), ('103', 4), ('201', 2), ('202', 3),
           ('203', 4), ('301', 2), ('302', 3), ('303', 4)
), bed_specs AS (
    SELECT r.id AS room_id, chr(64 + series.n) AS label
    FROM rooms r JOIN room_specs s ON s.number = r.number
    CROSS JOIN LATERAL generate_series(1, s.bed_count) AS series(n)
    WHERE r.org_id = '32f33aef-dc6a-4caf-9f05-794f65f30c8a' AND r.deleted_at IS NULL
)
INSERT INTO beds (id, org_id, room_id, label)
SELECT gen_random_uuid(), '32f33aef-dc6a-4caf-9f05-794f65f30c8a', room_id, label
FROM bed_specs s
WHERE NOT EXISTS (
    SELECT 1 FROM beds b WHERE b.org_id = '32f33aef-dc6a-4caf-9f05-794f65f30c8a'
      AND b.room_id = s.room_id AND b.label = s.label AND b.deleted_at IS NULL
);

-- Six occupied beds across AC and Non-AC rooms. The remaining 21 beds stay
-- vacant, which lets the owner test both occupied and available states.
WITH resident_specs(full_name, phone, room_number, bed_label, rent_paise, deposit_paise) AS (
    VALUES
      ('Asha Kumar',   '9000010001', '101', 'A', 850000, 1700000),
      ('Bharat Singh', '9000010002', '102', 'A', 950000, 1900000),
      ('Chitra Rao',   '9000010003', '102', 'B', 950000, 1900000),
      ('Divya Nair',   '9000010004', '202', 'A', 850000, 1700000),
      ('Eshan Shah',   '9000010005', '203', 'A', 950000, 1900000),
      ('Farah Ali',    '9000010006', '302', 'A', 950000, 1900000)
), placements AS (
    SELECT s.*, b.id AS bed_id
    FROM resident_specs s
    JOIN rooms r ON r.org_id = '32f33aef-dc6a-4caf-9f05-794f65f30c8a'
      AND r.number = s.room_number AND r.deleted_at IS NULL
    JOIN beds b ON b.org_id = r.org_id AND b.room_id = r.id
      AND b.label = s.bed_label AND b.deleted_at IS NULL
)
INSERT INTO residents (
    id, org_id, branch_id, bed_id, full_name, phone, plan_kind, status, rent_paise, deposit_paise
)
SELECT gen_random_uuid(), '32f33aef-dc6a-4caf-9f05-794f65f30c8a',
       (SELECT id FROM branches WHERE org_id = '32f33aef-dc6a-4caf-9f05-794f65f30c8a'
        AND name = 'Main Branch' AND deleted_at IS NULL LIMIT 1),
       p.bed_id, p.full_name, p.phone, 'monthly', 'active', p.rent_paise, p.deposit_paise
FROM placements p
WHERE NOT EXISTS (
    SELECT 1 FROM residents r WHERE r.org_id = '32f33aef-dc6a-4caf-9f05-794f65f30c8a'
      AND r.phone = p.phone AND r.deleted_at IS NULL
);
COMMIT;
