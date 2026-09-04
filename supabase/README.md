# Supabase database baseline

This directory provisions the current Rust backend schema in Supabase. It does
not change the Android application's Retrofit API, custom OTP/JWT flow, or
tenant-routing behavior.

## Security boundary

The Android application must not use a Supabase database URL, database password,
or service-role key. The Rust backend remains the only application client.
Operational data stays in `pool` and is protected by the existing transaction
scoped `app.current_org` RLS policy. `pool` is deliberately not exposed by the
Supabase Data API.

## Local validation

The full local Supabase stack needs more memory than the current 2 GB Colima
profile. Increase it before starting the stack:

```sh
colima stop
colima start --cpu 4 --memory 6 --disk 60
npx supabase start
npx supabase db reset --local
```

After the reset, run `tests/tenant_rls.sql` using a local Postgres client. The
test temporarily grants the ordinary `authenticated` role access inside one
rolled-back transaction and confirms that it cannot see or insert another
organization's data.

For the release-level check, run `tests/multitenancy_deep.sql` with a
privileged maintenance connection:

```sh
psql "$MIGRATION_DATABASE_URL" -v ON_ERROR_STOP=1 -f supabase/tests/multitenancy_deep.sql
```

It verifies forced RLS and absent persistent client grants for every tenant
table, cross-organization reads, updates, deletes, forged inserts, the
transitively-scoped move-out deductions table, and that `app.current_org` does
not survive a transaction. It always rolls back its fixtures and temporary
grants.

Never use `db reset --linked`. Do not run `db push` until the local migration
and tenant-isolation results have been reviewed.

## Current database snapshot

`seeds/current_database_data.sql` is a data-only export of the current
application database (`public` and `pool` schemas). It contains real operational
data, including personal data and authentication/session records; treat it as a
secret and do not publish it or commit it to a public repository.

Apply this project's migrations to the target first, then restore with one
command. The script requests the password without echoing it:

```sh
./scripts/restore-current-database.sh \
  --host aws-0-<region>.pooler.supabase.com --port 5432 \
  --database postgres --user postgres.<project-ref> --replace
```

`--replace` removes the existing application rows in the target before loading
the snapshot. Omit it to have the script refuse a nonempty target.

## Sample data

`seeds/sample_owner_manager_tenant.sql` builds a demo organization, **Hamsa
Vilas PG** (`cb68eae7-2955-5cae-e440-0a0190731fe1`), containing one account per
staff/resident role:

| role | email | scope |
| --- | --- | --- |
| owner | bhargavraj911@gmail.com | org-wide |
| manager | bhargavraj911@gmail.com | HSR Layout branch |
| tenant | kriahkunar2@gmail.com | bed 102-A |

All ids derive from `md5('hv:<key>')::uuid`, so the script is deterministic and
re-runnable; a second run updates rows in place instead of duplicating them.
Everything runs inside one transaction, so a failure leaves nothing behind.

```sh
export MIGRATION_DATABASE_URL='postgresql://postgres:<password>@db.<project-ref>.supabase.co:5432/postgres'
./scripts/seed-supabase-demo.sh
cargo run -p pg-infrastructure --example seed_sample_receipts
```

The second command matters: the seed inserts receipts already marked `ready`,
but only the payment route renders and stores their PDFs. The example fills that
gap with the same `SimpleReceiptRenderer` and object store the server uses, so
the seeded receipts download like real ones. Skip it and every receipt download
returns 404.

## Cloud rollout order

1. Validate locally and record the result.
2. Take a backup and verify the linked project is `pg-mags-dev`.
3. Run `npx supabase db push --dry-run` and review its output.
4. Obtain explicit approval before the actual `npx supabase db push`.
