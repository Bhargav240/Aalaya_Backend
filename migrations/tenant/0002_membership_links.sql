ALTER TABLE residents
    ADD COLUMN IF NOT EXISTS membership_id uuid
    REFERENCES public.user_org_roles(id);

CREATE INDEX IF NOT EXISTS residents_org_membership_idx
    ON residents (org_id, membership_id);

CREATE UNIQUE INDEX IF NOT EXISTS residents_one_live_per_membership_uidx
    ON residents (org_id, membership_id)
    WHERE membership_id IS NOT NULL
      AND deleted_at IS NULL
      AND status <> 'moved_out';
