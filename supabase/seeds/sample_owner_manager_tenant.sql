-- ============================================================================
-- Sample dataset: one owner, one manager, one tenant.
--
--   owner   bhargavraj911@gmail.com   Bhargavraj   (org-wide)
--   manager manager@aalaya.in      Manager   (scoped to the HSR branch)
--   tenant  kriahkunar2@gmail.com      Krish     (bed 102-A)
--
-- All three sign in with the normal email-OTP flow; no passwords exist in this
-- system. The dataset lives in its own organization ("Aalaya PG") so the
-- legacy "Supabase Development PG" org is left structurally intact.
--
-- Every id is derived with md5('hv:<key>')::uuid, so the script is
-- deterministic and safe to re-run: re-running updates rows in place rather
-- than duplicating them.
--
-- Run with:
--   export MIGRATION_DATABASE_URL='postgresql://...'
--   ./scripts/seed-supabase-demo.sh
-- Then generate receipt PDFs with:
--   cargo run -p pg-infrastructure --example seed_sample_receipts
-- ============================================================================

BEGIN;

-- Tenant tables are intentionally unqualified below; keep them in the pooled
-- schema regardless of the database role's default search path.
SET LOCAL search_path TO pool, public, extensions;

-- ─────────────────────────────────────────────────────────────────────────────
-- 0. Organization, placement and receipt template
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO public.organizations (id, name, created_at)
VALUES (md5('hv:org')::uuid, 'Aalaya PG', TIMESTAMPTZ '2026-01-05 09:30:00+05:30')
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name, deleted_at = NULL;

INSERT INTO public.tenant_placement (org_id, mode, schema_name, status)
VALUES (md5('hv:org')::uuid, 'pooled', 'pool', 'active')
ON CONFLICT (org_id) DO UPDATE
SET mode = 'pooled', schema_name = 'pool', status = 'active', updated_at = now();

INSERT INTO public.receipt_templates (org_id, display_name, address, footer, accent, logo_key)
VALUES (
    md5('hv:org')::uuid,
    'Aalaya PG',
    '#42, 27th Main Road, HSR Layout Sector 2, Bengaluru, Karnataka 560102',
    'Computer-generated receipt; no signature required. Queries: +91 90000 10101 / hello@hamsavilas.in',
    '#2E7D32',
    NULL  -- no logo asset uploaded; a key here without a stored object would dangle
)
ON CONFLICT (org_id) DO UPDATE
SET display_name = EXCLUDED.display_name, address = EXCLUDED.address,
    footer = EXCLUDED.footer, accent = EXCLUDED.accent, updated_at = now();

-- ─────────────────────────────────────────────────────────────────────────────
-- 2. The three people
--    Bhargavraj and Bhargav already exist as Explore prospects, so their existing
--    rows are updated in place; Krish is created if he has not signed in yet.
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO public.platform_users
    (id, org_id, email, phone, full_name, status, verified, email_verified_at, created_at)
SELECT md5('hv:user:krish')::uuid,
       '00000000-0000-0000-0000-000000000001',
       'kriahkunar2@gmail.com', '9000010103', 'Krish',
       'active', true,
       TIMESTAMPTZ '2026-05-20 18:40:00+05:30',
       TIMESTAMPTZ '2026-05-20 18:35:00+05:30'
WHERE NOT EXISTS (
    SELECT 1 FROM public.platform_users
    WHERE lower(email) = 'kriahkunar2@gmail.com' AND deleted_at IS NULL
);

UPDATE public.platform_users SET
    full_name         = 'Bhargavraj',
    phone             = COALESCE(phone, '9000010101'),
    status            = 'active',
    verified          = true,
    email_verified_at = COALESCE(email_verified_at, now())
WHERE lower(email) = 'bhargavraj911@gmail.com' AND deleted_at IS NULL;

UPDATE public.platform_users SET
    full_name         = 'Bhargav',
    phone             = COALESCE(phone, '9000010102'),
    status            = 'active',
    verified          = true,
    email_verified_at = COALESCE(email_verified_at, now())
WHERE lower(email) = 'manager@aalaya.in' AND deleted_at IS NULL;

UPDATE public.platform_users SET
    full_name         = 'Krish',
    phone             = COALESCE(phone, '9000010103'),
    status            = 'active',
    verified          = true,
    email_verified_at = COALESCE(email_verified_at, TIMESTAMPTZ '2026-05-20 18:40:00+05:30')
WHERE lower(email) = 'kriahkunar2@gmail.com' AND deleted_at IS NULL;

-- Every id the rest of the script needs, resolved once.
CREATE TEMP TABLE ids ON COMMIT DROP AS
SELECT
    md5('hv:org')::uuid                          AS org_id,
    '00000000-0000-0000-0000-000000000001'::uuid AS platform_org_id,
    md5('hv:branch')::uuid                       AS branch_id,
    md5('hv:resident:krish')::uuid               AS krish_resident_id,
    md5('hv:resident:naveen')::uuid              AS naveen_resident_id,
    md5('hv:invitation:krish')::uuid             AS krish_invitation_id,
    md5('hv:invitation:pending')::uuid           AS pending_invitation_id,
    md5('hv:membership:tenant')::uuid            AS tenant_membership_id,
    (SELECT id FROM public.platform_users
      WHERE lower(email) = 'bhargavraj911@gmail.com' AND deleted_at IS NULL)      AS owner_id,
    (SELECT id FROM public.platform_users
      WHERE lower(email) = 'manager@aalaya.in'  AND deleted_at IS NULL)      AS manager_id,
    (SELECT id FROM public.platform_users
      WHERE lower(email) = 'kriahkunar2@gmail.com'    AND deleted_at IS NULL)      AS tenant_id,
    (SELECT id FROM public.platform_users
      WHERE lower(email) = 'otp.test.20260812@example.com' AND deleted_at IS NULL) AS prospect_id;

-- ─────────────────────────────────────────────────────────────────────────────
-- 3. Memberships. Role precedence at sign-in is owner > manager > tenant >
--    prospect, so each person lands in the right context automatically and can
--    switch via /auth/switch-organization.
-- ─────────────────────────────────────────────────────────────────────────────

-- Owner: org-wide. branch_id stays NULL on purpose — an owner is not scoped to
-- a branch, and require_branch_management_scope() short-circuits for Owner.
INSERT INTO public.user_org_roles
    (id, user_id, org_id, role, branch_id, membership_status, activated_at, ended_at)
SELECT md5('hv:membership:owner')::uuid, owner_id, org_id, 'owner', NULL,
       'active', TIMESTAMPTZ '2026-01-05 09:30:00+05:30', NULL
FROM ids
ON CONFLICT (user_id, org_id, role) DO UPDATE
SET branch_id = EXCLUDED.branch_id, membership_status = 'active',
    activated_at = EXCLUDED.activated_at, ended_at = NULL;

-- Manager: scoped to the HSR branch. This branch_id is what makes the
-- residents/rooms/enquiries endpoints branch-scoped for Bhargav.
INSERT INTO public.user_org_roles
    (id, user_id, org_id, role, branch_id, membership_status, activated_at, ended_at)
SELECT md5('hv:membership:manager')::uuid, manager_id, org_id, 'manager', branch_id,
       'active', TIMESTAMPTZ '2026-02-01 10:00:00+05:30', NULL
FROM ids
ON CONFLICT (user_id, org_id, role) DO UPDATE
SET branch_id = EXCLUDED.branch_id, membership_status = 'active',
    activated_at = EXCLUDED.activated_at, ended_at = NULL;

-- Tenant: created by the invitation-approval flow in real life.
INSERT INTO public.user_org_roles
    (id, user_id, org_id, role, branch_id, membership_status, activated_at, ended_at)
SELECT tenant_membership_id, tenant_id, org_id, 'tenant', branch_id,
       'active', TIMESTAMPTZ '2026-05-26 12:15:00+05:30', NULL
FROM ids
ON CONFLICT (user_id, org_id, role) DO UPDATE
SET branch_id = EXCLUDED.branch_id, membership_status = 'active',
    activated_at = EXCLUDED.activated_at, ended_at = NULL;

-- Krish also keeps a prospect context so Explore stays usable for him.
INSERT INTO public.user_org_roles
    (id, user_id, org_id, role, branch_id, membership_status, activated_at, ended_at)
SELECT md5('hv:membership:krish-prospect')::uuid, tenant_id, platform_org_id, 'prospect', NULL,
       'active', TIMESTAMPTZ '2026-05-20 18:40:00+05:30', NULL
FROM ids
ON CONFLICT (user_id, org_id, role) DO UPDATE
SET membership_status = 'active', ended_at = NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 4. Branch, rooms and beds  (pool schema — RLS keyed on app.current_org)
-- ─────────────────────────────────────────────────────────────────────────────
SELECT set_config('app.current_org', (SELECT org_id::text FROM ids), true);

INSERT INTO branches (id, org_id, name, address, created_at)
SELECT branch_id, org_id, 'HSR Layout',
       '#42, 27th Main Road, HSR Layout Sector 2, Bengaluru, Karnataka 560102',
       TIMESTAMPTZ '2026-01-05 09:35:00+05:30'
FROM ids
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name, address = EXCLUDED.address, deleted_at = NULL;

-- 6 rooms / 15 beds, mixed AC and non-AC.
INSERT INTO rooms (id, org_id, branch_id, number, cooling_type, created_at)
SELECT md5('hv:room:' || s.number)::uuid, i.org_id, i.branch_id, s.number, s.cooling_type,
       TIMESTAMPTZ '2026-01-05 09:40:00+05:30'
FROM ids i
CROSS JOIN (VALUES
    ('101', 'non_ac'), ('102', 'ac'), ('103', 'non_ac'),
    ('201', 'ac'),     ('202', 'non_ac'), ('203', 'ac')
) AS s(number, cooling_type)
ON CONFLICT (id) DO UPDATE
SET number = EXCLUDED.number, cooling_type = EXCLUDED.cooling_type, deleted_at = NULL;

INSERT INTO beds (id, org_id, room_id, label, created_at)
SELECT md5('hv:bed:' || s.room || ':' || s.label)::uuid, i.org_id,
       md5('hv:room:' || s.room)::uuid, s.label,
       TIMESTAMPTZ '2026-01-05 09:45:00+05:30'
FROM ids i
CROSS JOIN (VALUES
    ('101','A'), ('101','B'), ('101','C'),
    ('102','A'), ('102','B'),
    ('103','A'), ('103','B'), ('103','C'),
    ('201','A'), ('201','B'),
    ('202','A'), ('202','B'), ('202','C'),
    ('203','A'), ('203','B')
) AS s(room, label)
ON CONFLICT (id) DO UPDATE
SET label = EXCLUDED.label, deleted_at = NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 5. Residents
--    Krish is the live, app-using tenant. Naveen is a closed historical record
--    that gives the move-out settlement and finance history something real to
--    show; he has no login account.
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO residents (id, org_id, branch_id, bed_id, full_name, phone, plan_kind,
                       status, rent_paise, deposit_paise, user_id, membership_id,
                       moved_out_at, created_at)
SELECT krish_resident_id, org_id, branch_id, md5('hv:bed:102:A')::uuid,
       'Krish', '9000010103', 'monthly', 'active',
       1250000, 2500000, tenant_id, tenant_membership_id, NULL,
       TIMESTAMPTZ '2026-06-01 11:00:00+05:30'
FROM ids
ON CONFLICT (id) DO UPDATE
SET bed_id = EXCLUDED.bed_id, full_name = EXCLUDED.full_name, phone = EXCLUDED.phone,
    status = 'active', rent_paise = EXCLUDED.rent_paise,
    deposit_paise = EXCLUDED.deposit_paise, user_id = EXCLUDED.user_id,
    membership_id = EXCLUDED.membership_id, moved_out_at = NULL,
    created_at = EXCLUDED.created_at, deleted_at = NULL;

INSERT INTO residents (id, org_id, branch_id, bed_id, full_name, phone, plan_kind,
                       status, rent_paise, deposit_paise, user_id, membership_id,
                       moved_out_at, created_at)
SELECT naveen_resident_id, org_id, branch_id, NULL,
       'Naveen Rao', '9000010104', 'monthly', 'moved_out',
       1200000, 2400000, NULL, NULL,
       TIMESTAMPTZ '2026-05-31 00:00:00+00',
       TIMESTAMPTZ '2026-01-15 10:00:00+05:30'
FROM ids
ON CONFLICT (id) DO UPDATE
SET status = 'moved_out', bed_id = NULL, moved_out_at = EXCLUDED.moved_out_at,
    created_at = EXCLUDED.created_at, deleted_at = NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 6. Money: invoice sequence, payments, receipts
--    invoice_sequences is keyed by organization and YYMM; next_number is
--    "the next number to hand out", so a period with one issued receipt is 2.
--    receipt_number and invoice_number are the same value by design.
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO invoice_sequences (org_id, period, next_number)
SELECT i.org_id, s.period, 2
FROM ids i
CROSS JOIN (VALUES ('2602'), ('2603'), ('2604'), ('2605'), ('2606'), ('2607'))
    AS s(period)
ON CONFLICT (org_id, period) DO UPDATE
SET next_number = GREATEST(invoice_sequences.next_number, EXCLUDED.next_number);

INSERT INTO payments (id, org_id, resident_id, amount_paise, paid_at, recorded_by,
                      method, status, upi_ref, reconciled_at, idempotency_key, created_at)
SELECT md5('hv:payment:' || p.key)::uuid, i.org_id,
       CASE p.who WHEN 'krish' THEN i.krish_resident_id ELSE i.naveen_resident_id END,
       p.amount, p.paid_at,
       CASE p.recorder WHEN 'owner' THEN i.owner_id ELSE i.manager_id END,
       p.method, 'recorded', p.upi_ref,
       CASE WHEN p.upi_ref IS NOT NULL THEN p.paid_at + interval '2 hours' END,
       'seed-' || p.key, p.paid_at
FROM ids i
CROSS JOIN (VALUES
    ('naveen:2602', 'naveen', 1200000, TIMESTAMPTZ '2026-02-03 19:20:00+05:30', 'owner',   'cash', NULL),
    ('naveen:2603', 'naveen', 1200000, TIMESTAMPTZ '2026-03-04 18:05:00+05:30', 'owner',   'upi',  '403712889145'),
    ('naveen:2604', 'naveen', 1200000, TIMESTAMPTZ '2026-04-03 20:10:00+05:30', 'manager', 'cash', NULL),
    ('naveen:2605', 'naveen', 1200000, TIMESTAMPTZ '2026-05-05 17:45:00+05:30', 'manager', 'upi',  '412908773261'),
    ('krish:2606',  'krish',  1250000, TIMESTAMPTZ '2026-06-05 18:30:00+05:30', 'owner',   'upi',  '426512873094'),
    ('krish:2607',  'krish',  1250000, TIMESTAMPTZ '2026-07-04 19:15:00+05:30', 'manager', 'cash', NULL)
) AS p(key, who, amount, paid_at, recorder, method, upi_ref)
ON CONFLICT (id) DO UPDATE
SET amount_paise = EXCLUDED.amount_paise, paid_at = EXCLUDED.paid_at,
    recorded_by = EXCLUDED.recorded_by, method = EXCLUDED.method,
    upi_ref = EXCLUDED.upi_ref, reconciled_at = EXCLUDED.reconciled_at,
    status = 'recorded', deleted_at = NULL;

INSERT INTO receipts (id, org_id, resident_id, payment_id, receipt_number, invoice_number,
                      amount_paise, storage_key, status, issued_at)
SELECT md5('hv:receipt:' || r.key)::uuid, i.org_id,
       CASE r.who WHEN 'krish' THEN i.krish_resident_id ELSE i.naveen_resident_id END,
       md5('hv:payment:' || r.key)::uuid,
       r.invoice, r.invoice, r.amount,
       'receipts/' || i.org_id || '/'
         || CASE r.who WHEN 'krish' THEN i.krish_resident_id ELSE i.naveen_resident_id END
         || '/' || md5('hv:receipt:' || r.key)::uuid || '.pdf',
       'ready', r.issued_at
FROM ids i
CROSS JOIN (VALUES
    ('naveen:2602', 'naveen', 'INV-2602-001', 1200000, TIMESTAMPTZ '2026-02-03 19:20:00+05:30'),
    ('naveen:2603', 'naveen', 'INV-2603-001', 1200000, TIMESTAMPTZ '2026-03-04 18:05:00+05:30'),
    ('naveen:2604', 'naveen', 'INV-2604-001', 1200000, TIMESTAMPTZ '2026-04-03 20:10:00+05:30'),
    ('naveen:2605', 'naveen', 'INV-2605-001', 1200000, TIMESTAMPTZ '2026-05-05 17:45:00+05:30'),
    ('krish:2606',  'krish',  'INV-2606-001', 1250000, TIMESTAMPTZ '2026-06-05 18:30:00+05:30'),
    ('krish:2607',  'krish',  'INV-2607-001', 1250000, TIMESTAMPTZ '2026-07-04 19:15:00+05:30')
) AS r(key, who, invoice, amount, issued_at)
ON CONFLICT (id) DO UPDATE
SET receipt_number = EXCLUDED.receipt_number, invoice_number = EXCLUDED.invoice_number,
    amount_paise = EXCLUDED.amount_paise, storage_key = EXCLUDED.storage_key,
    status = 'ready', issued_at = EXCLUDED.issued_at, deleted_at = NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 7. Move-out settlement for Naveen
--    Mirrors services::move_out exactly:
--      rent_to_date = rent * days(joined..last_day) / 30 = 1200000 * 136 / 30
--      outstanding  = rent_to_date - paid  = 5440000 - 4800000
--      refund       = deposit - outstanding - deductions
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO move_out_settlements
    (id, org_id, resident_id, last_day, notice_days, rent_to_date_paise,
     paid_total_paise, outstanding_paise, deposit_held_paise, deductions_total_paise,
     refund_paise, forfeited, note, settled_by, settled_at)
SELECT md5('hv:settlement:naveen')::uuid, org_id, naveen_resident_id,
       DATE '2026-05-31', 30, 5440000, 4800000, 640000, 2400000, 240000,
       1520000, false, 'refund = deposit - outstanding - deductions',
       owner_id, TIMESTAMPTZ '2026-06-02 16:00:00+05:30'
FROM ids
ON CONFLICT (id) DO UPDATE
SET rent_to_date_paise = EXCLUDED.rent_to_date_paise,
    paid_total_paise = EXCLUDED.paid_total_paise,
    outstanding_paise = EXCLUDED.outstanding_paise,
    deductions_total_paise = EXCLUDED.deductions_total_paise,
    refund_paise = EXCLUDED.refund_paise, note = EXCLUDED.note;

INSERT INTO move_out_deductions (id, settlement_id, reason, amount_paise)
SELECT md5('hv:deduction:' || d.key)::uuid, md5('hv:settlement:naveen')::uuid,
       d.reason, d.amount
FROM (VALUES
    ('naveen:paint',   'Wall repainting after nail damage', 150000),
    ('naveen:utility', 'Unpaid electricity share (Apr-May)', 90000)
) AS d(key, reason, amount)
ON CONFLICT (id) DO UPDATE
SET reason = EXCLUDED.reason, amount_paise = EXCLUDED.amount_paise;

-- ─────────────────────────────────────────────────────────────────────────────
-- 8. Complaints and their state history
--    complaint_events uses a generated identity key, so this org's rows are
--    replaced wholesale to keep the script re-runnable.
-- ─────────────────────────────────────────────────────────────────────────────
DELETE FROM complaint_events WHERE org_id = (SELECT org_id FROM ids);

INSERT INTO complaints (id, org_id, branch_id, raised_by, title, description, state,
                        is_business_level, created_at, updated_at)
SELECT md5('hv:complaint:' || c.key)::uuid, i.org_id,
       CASE WHEN c.business THEN NULL ELSE i.branch_id END,
       CASE c.by WHEN 'tenant' THEN i.tenant_id ELSE i.manager_id END,
       c.title, c.description, c.state, c.business, c.created_at, c.updated_at
FROM ids i
CROSS JOIN (VALUES
    ('geyser', 'tenant', false,
     'Geyser in 102 bathroom not heating',
     'The geyser trips within a minute of switching on. Water stays cold in the mornings.',
     'in_progress',
     TIMESTAMPTZ '2026-08-02 07:45:00+05:30', TIMESTAMPTZ '2026-08-04 11:20:00+05:30'),
    ('wifi', 'tenant', false,
     'Wi-Fi drops between 9pm and 11pm',
     'Connection on the second floor drops repeatedly during peak hours. Speed test shows under 1 Mbps.',
     'resolved',
     TIMESTAMPTZ '2026-07-18 21:30:00+05:30', TIMESTAMPTZ '2026-07-22 15:10:00+05:30'),
    ('vendor', 'manager', true,
     'Water tanker vendor overcharging since July',
     'The tanker vendor raised the per-trip rate twice in one month without notice. Needs an owner decision on switching vendors.',
     'open',
     TIMESTAMPTZ '2026-08-10 10:05:00+05:30', TIMESTAMPTZ '2026-08-10 10:05:00+05:30')
) AS c(key, by, business, title, description, state, created_at, updated_at)
ON CONFLICT (id) DO UPDATE
SET title = EXCLUDED.title, description = EXCLUDED.description, state = EXCLUDED.state,
    branch_id = EXCLUDED.branch_id, is_business_level = EXCLUDED.is_business_level,
    created_at = EXCLUDED.created_at, updated_at = EXCLUDED.updated_at, deleted_at = NULL;

INSERT INTO complaint_events (org_id, complaint_id, from_state, to_state, by_user, created_at)
SELECT i.org_id, md5('hv:complaint:' || e.complaint)::uuid, e.from_state, e.to_state,
       CASE e.by WHEN 'tenant' THEN i.tenant_id
                 WHEN 'manager' THEN i.manager_id
                 ELSE i.owner_id END,
       e.at
FROM ids i
CROSS JOIN (VALUES
    ('geyser', NULL,          'open',        'tenant',  TIMESTAMPTZ '2026-08-02 07:45:00+05:30'),
    ('geyser', 'open',        'assigned',    'manager', TIMESTAMPTZ '2026-08-02 09:10:00+05:30'),
    ('geyser', 'assigned',    'in_progress', 'manager', TIMESTAMPTZ '2026-08-04 11:20:00+05:30'),
    ('wifi',   NULL,          'open',        'tenant',  TIMESTAMPTZ '2026-07-18 21:30:00+05:30'),
    ('wifi',   'open',        'assigned',    'manager', TIMESTAMPTZ '2026-07-19 09:00:00+05:30'),
    ('wifi',   'assigned',    'in_progress', 'manager', TIMESTAMPTZ '2026-07-20 14:25:00+05:30'),
    ('wifi',   'in_progress', 'resolved',    'manager', TIMESTAMPTZ '2026-07-22 15:10:00+05:30'),
    ('vendor', NULL,          'open',        'manager', TIMESTAMPTZ '2026-08-10 10:05:00+05:30')
) AS e(complaint, from_state, to_state, by, at);

-- ─────────────────────────────────────────────────────────────────────────────
-- 9. Notices
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO notices (id, org_id, branch_id, title, body, created_by, created_at)
SELECT md5('hv:notice:' || n.key)::uuid, i.org_id, i.branch_id, n.title, n.body,
       CASE n.by WHEN 'owner' THEN i.owner_id ELSE i.manager_id END,
       n.created_at
FROM ids i
CROSS JOIN (VALUES
    ('rent-reminder', 'owner',
     'Rent due on the 5th',
     'A reminder that monthly rent is due by the 5th. UPI and cash are both accepted; collect your receipt from the manager on the same day.',
     TIMESTAMPTZ '2026-08-01 09:00:00+05:30'),
    ('water', 'manager',
     'Water supply maintenance on 14 Aug',
     'The overhead tank will be cleaned on 14 August between 10am and 2pm. Please store drinking water in advance.',
     TIMESTAMPTZ '2026-08-08 18:20:00+05:30'),
    ('dinner', 'manager',
     'Independence Day special dinner',
     'Special dinner menu on 15 August at 8pm in the common dining area. Let the kitchen know by 13 August if you will be away.',
     TIMESTAMPTZ '2026-08-11 17:00:00+05:30')
) AS n(key, by, title, body, created_at)
ON CONFLICT (id) DO UPDATE
SET title = EXCLUDED.title, body = EXCLUDED.body, created_at = EXCLUDED.created_at,
    deleted_at = NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 10. Maker-checker change requests
--     Bhargav (manager) is the maker; edits never apply immediately for him,
--     they queue here for Bhargavraj (owner) to approve. Payload keys match the
--     columns apply_update() understands.
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO change_requests (id, org_id, entity_type, entity_id, operation, proposed,
                             requested_by, status, created_at, decided_at, decided_by)
SELECT md5('hv:change:rent-revision')::uuid, org_id, 'resident', krish_resident_id,
       'update', jsonb_build_object('rent_paise', 1300000),
       manager_id, 'pending', TIMESTAMPTZ '2026-08-11 12:30:00+05:30', NULL, NULL
FROM ids
ON CONFLICT (id) DO UPDATE
SET proposed = EXCLUDED.proposed, status = 'pending', decided_at = NULL, decided_by = NULL;

INSERT INTO change_requests (id, org_id, entity_type, entity_id, operation, proposed,
                             requested_by, status, created_at, decided_at, decided_by)
SELECT md5('hv:change:room-201-cooling')::uuid, org_id, 'room', md5('hv:room:201')::uuid,
       'update', jsonb_build_object('cooling_type', 'non_ac'),
       manager_id, 'pending', TIMESTAMPTZ '2026-08-12 09:15:00+05:30', NULL, NULL
FROM ids
ON CONFLICT (id) DO UPDATE
SET proposed = EXCLUDED.proposed, status = 'pending', decided_at = NULL, decided_by = NULL;

-- One decided request for history. Rejected, so nothing was applied and the
-- underlying resident row stays consistent with this record.
INSERT INTO change_requests (id, org_id, entity_type, entity_id, operation, proposed,
                             requested_by, status, created_at, decided_at, decided_by)
SELECT md5('hv:change:deposit-waiver')::uuid, org_id, 'resident', krish_resident_id,
       'update', jsonb_build_object('deposit_paise', 0),
       manager_id, 'rejected', TIMESTAMPTZ '2026-08-05 14:40:00+05:30',
       TIMESTAMPTZ '2026-08-06 10:05:00+05:30', owner_id
FROM ids
ON CONFLICT (id) DO UPDATE
SET status = 'rejected', decided_at = EXCLUDED.decided_at, decided_by = EXCLUDED.decided_by;

-- ─────────────────────────────────────────────────────────────────────────────
-- 11. Away periods (tenant self-service)
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO away_periods (id, org_id, resident_id, from_date, to_date, note, status,
                          created_at, cancelled_at)
SELECT md5('hv:away:' || a.key)::uuid, i.org_id, i.krish_resident_id,
       a.from_date, a.to_date, a.note, a.status, a.created_at, a.cancelled_at
FROM ids i
CROSS JOIN (VALUES
    ('onam', DATE '2026-08-20', DATE '2026-08-25', 'Travelling home for Onam', 'active',
     TIMESTAMPTZ '2026-08-09 20:15:00+05:30', NULL::timestamptz),
    ('offsite', DATE '2026-07-10', DATE '2026-07-14', 'Office offsite in Goa', 'active',
     TIMESTAMPTZ '2026-07-05 13:00:00+05:30', NULL::timestamptz),
    ('cancelled-trip', DATE '2026-06-20', DATE '2026-06-22', 'Weekend trip to Mysuru', 'cancelled',
     TIMESTAMPTZ '2026-06-15 19:30:00+05:30', TIMESTAMPTZ '2026-06-18 08:45:00+05:30')
) AS a(key, from_date, to_date, note, status, created_at, cancelled_at)
ON CONFLICT (id) DO UPDATE
SET from_date = EXCLUDED.from_date, to_date = EXCLUDED.to_date, note = EXCLUDED.note,
    status = EXCLUDED.status, cancelled_at = EXCLUDED.cancelled_at, deleted_at = NULL;

-- ─────────────────────────────────────────────────────────────────────────────
-- 12. Audit trail. Identity primary key, so replaced wholesale per org.
--     Action/entity_type pairs match what the repositories actually write.
-- ─────────────────────────────────────────────────────────────────────────────
DELETE FROM audit_log WHERE org_id = (SELECT org_id FROM ids);

INSERT INTO audit_log (org_id, actor_id, action, entity_type, entity_id, before, after,
                       request_id, created_at)
SELECT i.org_id,
       CASE a.by WHEN 'owner' THEN i.owner_id ELSE i.manager_id END,
       a.action, a.entity_type,
       CASE a.entity_type
           WHEN 'branch'    THEN i.branch_id
           WHEN 'room'      THEN md5('hv:room:102')::uuid
           WHEN 'bed'       THEN md5('hv:bed:102:A')::uuid
           WHEN 'resident'  THEN i.krish_resident_id
           WHEN 'payment'   THEN md5('hv:payment:' || a.detail)::uuid
           WHEN 'complaint' THEN md5('hv:complaint:' || a.detail)::uuid
           WHEN 'notice'    THEN md5('hv:notice:' || a.detail)::uuid
       END,
       NULL, a.after, md5('hv:request:' || a.key)::uuid, a.created_at
FROM ids i
CROSS JOIN (VALUES
    ('branch-create', 'owner',   'create',              'branch',    '',
     '{"name":"HSR Layout"}'::jsonb,                    TIMESTAMPTZ '2026-01-05 09:35:00+05:30'),
    ('room-create', 'owner',     'create',              'room',      '',
     '{"number":"102","cooling_type":"ac"}'::jsonb,     TIMESTAMPTZ '2026-01-05 09:40:00+05:30'),
    ('bed-create', 'owner',      'create',              'bed',       '',
     '{"label":"A"}'::jsonb,                            TIMESTAMPTZ '2026-01-05 09:45:00+05:30'),
    ('naveen-moveout', 'owner',  'move_out',            'resident',  '',
     '{"resident":"Naveen Rao","last_day":"2026-05-31"}'::jsonb,
                                                        TIMESTAMPTZ '2026-06-02 16:00:00+05:30'),
    ('krish-activate', 'owner',  'activate_tenant',     'resident',  '',
     '{"invitation":"accepted","role":"tenant"}'::jsonb, TIMESTAMPTZ '2026-05-26 12:15:00+05:30'),
    ('krish-link', 'owner',      'link_user',           'resident',  '',
     '{"linked":"kriahkunar2@gmail.com"}'::jsonb,       TIMESTAMPTZ '2026-05-26 12:15:05+05:30'),
    ('krish-bed', 'manager',     'assign_bed',          'resident',  '',
     '{"room":"102","bed":"A"}'::jsonb,                 TIMESTAMPTZ '2026-06-01 11:00:00+05:30'),
    ('pay-jun', 'owner',         'payment',             'payment',   'krish:2606',
     '{"amount_paise":1250000,"method":"upi"}'::jsonb,  TIMESTAMPTZ '2026-06-05 18:30:00+05:30'),
    ('pay-jul', 'manager',       'payment',             'payment',   'krish:2607',
     '{"amount_paise":1250000,"method":"cash"}'::jsonb, TIMESTAMPTZ '2026-07-04 19:15:00+05:30'),
    ('complaint-wifi', 'manager','create',              'complaint', 'wifi',
     '{"title":"Wi-Fi drops between 9pm and 11pm"}'::jsonb,
                                                        TIMESTAMPTZ '2026-07-18 21:30:00+05:30'),
    ('complaint-geyser-move', 'manager', 'complaint_transition', 'complaint', 'geyser',
     '{"from":"assigned","to":"in_progress"}'::jsonb,   TIMESTAMPTZ '2026-08-04 11:20:00+05:30'),
    ('notice-water', 'manager',  'create',              'notice',    'water',
     '{"title":"Water supply maintenance on 14 Aug"}'::jsonb,
                                                        TIMESTAMPTZ '2026-08-08 18:20:00+05:30')
) AS a(key, by, action, entity_type, detail, after, created_at);

-- ─────────────────────────────────────────────────────────────────────────────
-- 13. Marketplace: public listing + vacancy projection (control plane)
--     1 of 15 beds is occupied, so 14 are available.
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO public.public_listings
    (branch_id, org_id, title, area, city, lat, lng, description, amenities,
     status, rejection_note, review_request, published_at, created_at, updated_at)
SELECT branch_id, org_id,
       'Aalaya PG - HSR Layout',
       'HSR Layout', 'Bengaluru', 12.9121, 77.6446,
       'Fully furnished co-living close to HSR Sector 2 bus stop and the Agara lake loop. Single, twin and triple sharing across AC and non-AC rooms, with three meals a day included.',
       'WiFi, Three meals (veg), Laundry, Power backup, CCTV, Hot water, Housekeeping, Refrigerator, Study desk',
       'live', NULL, NULL,
       TIMESTAMPTZ '2026-01-20 12:00:00+05:30',
       TIMESTAMPTZ '2026-01-18 15:00:00+05:30',
       TIMESTAMPTZ '2026-01-20 12:00:00+05:30'
FROM ids
ON CONFLICT (branch_id) DO UPDATE
SET title = EXCLUDED.title, area = EXCLUDED.area, city = EXCLUDED.city,
    lat = EXCLUDED.lat, lng = EXCLUDED.lng, description = EXCLUDED.description,
    amenities = EXCLUDED.amenities, status = 'live',
    published_at = EXCLUDED.published_at, updated_at = now();

INSERT INTO public.listing_vacancy
    (branch_id, org_id, beds_total, beds_available, starting_price_paise, updated_at, stale)
SELECT branch_id, org_id, 15, 14, 1250000, now(), false
FROM ids
ON CONFLICT (branch_id) DO UPDATE
SET beds_total = EXCLUDED.beds_total, beds_available = EXCLUDED.beds_available,
    starting_price_paise = EXCLUDED.starting_price_paise, updated_at = now(), stale = false;

-- ─────────────────────────────────────────────────────────────────────────────
-- 14. Enquiries (the manager's lead inbox) — one per status.
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO public.enquiries (id, org_id, branch_id, prospect_id, phone, note, status, created_at)
SELECT md5('hv:enquiry:krish')::uuid, org_id, branch_id, tenant_id, '9000010103',
       'Looking for a single AC bed from the first week of June. Do you have anything on the first floor?',
       'closed', TIMESTAMPTZ '2026-05-20 18:50:00+05:30'
FROM ids
ON CONFLICT (id) DO UPDATE
SET note = EXCLUDED.note, status = EXCLUDED.status, created_at = EXCLUDED.created_at;

INSERT INTO public.enquiries (id, org_id, branch_id, prospect_id, phone, note, status, created_at)
SELECT md5('hv:enquiry:twin')::uuid, org_id, branch_id, prospect_id, '9000010199',
       'Is twin sharing available for two colleagues starting September?',
       'contacted', TIMESTAMPTZ '2026-08-05 11:25:00+05:30'
FROM ids WHERE prospect_id IS NOT NULL
ON CONFLICT (id) DO UPDATE
SET note = EXCLUDED.note, status = EXCLUDED.status, created_at = EXCLUDED.created_at;

INSERT INTO public.enquiries (id, org_id, branch_id, prospect_id, phone, note, status, created_at)
SELECT md5('hv:enquiry:food')::uuid, org_id, branch_id, prospect_id, '9000010199',
       'Are all three meals included in the quoted rent, or is food billed separately?',
       'new', TIMESTAMPTZ '2026-08-11 20:40:00+05:30'
FROM ids WHERE prospect_id IS NOT NULL
ON CONFLICT (id) DO UPDATE
SET note = EXCLUDED.note, status = EXCLUDED.status, created_at = EXCLUDED.created_at;

-- ─────────────────────────────────────────────────────────────────────────────
-- 15. Tenant invitations
--     Krish's ran the full lifecycle and ended at 'active'. A second one sits
--     at 'invited' so the owner/manager invitation screens have live work.
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO public.tenant_invitations
    (id, org_id, branch_id, intended_room_id, intended_bed_id, email, full_name,
     plan_kind, monthly_rent_paise, deposit_paise, planned_move_in_date,
     require_identity_verification, status, invited_by_user_id, accepted_by_user_id,
     expires_at, accepted_at, approved_at, approved_by_user_id,
     last_sent_at, created_at, updated_at)
SELECT krish_invitation_id, org_id, branch_id,
       md5('hv:room:102')::uuid, md5('hv:bed:102:A')::uuid,
       'kriahkunar2@gmail.com', 'Krish',
       'monthly', 1250000, 2500000, DATE '2026-06-01',
       true, 'active', owner_id, tenant_id,
       TIMESTAMPTZ '2026-05-29 10:00:00+05:30',
       TIMESTAMPTZ '2026-05-23 09:20:00+05:30',
       TIMESTAMPTZ '2026-05-26 12:15:00+05:30', owner_id,
       TIMESTAMPTZ '2026-05-22 10:00:00+05:30',
       TIMESTAMPTZ '2026-05-22 10:00:00+05:30',
       TIMESTAMPTZ '2026-05-26 12:15:00+05:30'
FROM ids
ON CONFLICT (id) DO UPDATE
SET status = 'active', accepted_by_user_id = EXCLUDED.accepted_by_user_id,
    accepted_at = EXCLUDED.accepted_at, approved_at = EXCLUDED.approved_at,
    approved_by_user_id = EXCLUDED.approved_by_user_id, updated_at = EXCLUDED.updated_at;

INSERT INTO public.tenant_invitations
    (id, org_id, branch_id, intended_room_id, intended_bed_id, email, full_name,
     plan_kind, monthly_rent_paise, deposit_paise, planned_move_in_date,
     require_identity_verification, status, invited_by_user_id, accepted_by_user_id,
     expires_at, accepted_at, approved_at, approved_by_user_id,
     last_sent_at, created_at, updated_at)
SELECT pending_invitation_id, org_id, branch_id,
       md5('hv:room:101')::uuid, md5('hv:bed:101:A')::uuid,
       'sample.tenant@example.com', 'Sample Tenant',
       'monthly', 1150000, 2300000, DATE '2026-09-01',
       true, 'invited', owner_id, NULL,
       now() + interval '7 days', NULL, NULL, NULL,
       now() - interval '1 day', now() - interval '1 day', now() - interval '1 day'
FROM ids
ON CONFLICT (id) DO UPDATE
SET status = 'invited', expires_at = now() + interval '7 days',
    last_sent_at = now() - interval '1 day', updated_at = now() - interval '1 day';

-- KYC history for Krish: the DigiLocker attempt failed on a name mismatch, and
-- the owner cleared him with an in-person manual review. The manual 'verified'
-- row is the latest one, which is what /verification returns and what the
-- activation guard checks for.
DELETE FROM public.invitation_identity_verifications
WHERE invitation_id = (SELECT krish_invitation_id FROM ids);
DELETE FROM public.manual_kyc_reviews
WHERE invitation_id = (SELECT krish_invitation_id FROM ids);

INSERT INTO public.invitation_identity_verifications
    (id, invitation_id, provider, provider_reference, document_type, status,
     verified_at, safe_failure_reason, created_at)
SELECT md5('hv:kyc:digilocker')::uuid, krish_invitation_id, 'digilocker',
       'DL-REF-8842013', 'AADHAAR', 'failed', NULL,
       'name on the document did not match the invitation',
       TIMESTAMPTZ '2026-05-24 15:05:00+05:30'
FROM ids;

INSERT INTO public.manual_kyc_reviews
    (id, invitation_id, reviewer_user_id, decision, reason, request_id, created_at)
SELECT md5('hv:kyc:manual-review')::uuid, krish_invitation_id, owner_id, 'approved',
       'Aadhaar and rental agreement checked in person at the property on 25 May 2026; the mismatch was an initial-versus-full-name difference.',
       md5('hv:request:manual-review')::uuid,
       TIMESTAMPTZ '2026-05-25 11:30:00+05:30'
FROM ids;

INSERT INTO public.invitation_identity_verifications
    (id, invitation_id, provider, provider_reference, document_type, status,
     verified_at, safe_failure_reason, created_at)
SELECT md5('hv:kyc:manual')::uuid, krish_invitation_id, 'manual',
       'MANUAL-2026-0525-01', 'AADHAAR', 'verified',
       TIMESTAMPTZ '2026-05-25 11:30:00+05:30', NULL,
       TIMESTAMPTZ '2026-05-25 11:30:00+05:30'
FROM ids;

-- ─────────────────────────────────────────────────────────────────────────────
-- 16. Identity verification record on the platform user
-- ─────────────────────────────────────────────────────────────────────────────
INSERT INTO public.verifications
    (id, user_id, org_id, method, status, verified_name, phone, emergency_contact,
     reason, created_at)
SELECT md5('hv:verification:krish')::uuid, tenant_id, org_id, 'manual', 'verified',
       'Krish', '9000010103', '9000010104',
       'Name on the Aadhaar matched after in-person review',
       TIMESTAMPTZ '2026-05-25 11:30:00+05:30'
FROM ids
ON CONFLICT (id) DO UPDATE
SET status = EXCLUDED.status, verified_name = EXCLUDED.verified_name,
    phone = EXCLUDED.phone, emergency_contact = EXCLUDED.emergency_contact,
    reason = EXCLUDED.reason;

COMMIT;
