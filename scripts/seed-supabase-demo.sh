#!/usr/bin/env bash
# Seed the Supabase database with a complete, re-runnable Aalaya demo.
#
# Usage:
#   export MIGRATION_DATABASE_URL='postgresql://...'
#   ./scripts/seed-supabase-demo.sh
#
# `DATABASE_URL` is also accepted. The database URL is intentionally never
# stored in this repository or printed by this script.

set -euo pipefail

readonly script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly backend_dir="$(cd -- "${script_dir}/.." && pwd)"
readonly seed_file="${backend_dir}/supabase/seeds/sample_owner_manager_tenant.sql"

database_url="${MIGRATION_DATABASE_URL:-${DATABASE_URL:-}}"

if ! command -v psql >/dev/null 2>&1; then
  printf 'Error: psql is required. Install the PostgreSQL client and try again.\n' >&2
  exit 1
fi

if [[ -z "${database_url}" ]]; then
  printf '%s\n' 'Error: set MIGRATION_DATABASE_URL (or DATABASE_URL) to your Supabase PostgreSQL connection string.' >&2
  printf '%s\n' "Example: export MIGRATION_DATABASE_URL='postgresql://postgres:<password>@db.<project-ref>.supabase.co:5432/postgres'" >&2
  exit 1
fi

if [[ ! -f "${seed_file}" ]]; then
  printf 'Error: seed SQL was not found at %s\n' "${seed_file}" >&2
  exit 1
fi

printf '%s\n' 'Seeding the Aalaya demo organization...'
printf '%s\n' 'The seed is idempotent: re-running it updates its own fixed demo rows only.'

# -X prevents local ~/.psqlrc settings from changing seed behavior.
# The SQL file owns one explicit transaction; ON_ERROR_STOP ensures a failed
# statement aborts that transaction instead of leaving partial sample data.
psql --dbname="${database_url}" \
  --no-psqlrc \
  --set=ON_ERROR_STOP=1 \
  --file="${seed_file}"

printf '%s\n' 'Demo data inserted successfully.'
printf '%s\n' 'Demo sign-in emails: bhargavraj911@gmail.com (owner), bhargavraj911@gmail.com (manager), kriahkunar2@gmail.com (tenant).'
