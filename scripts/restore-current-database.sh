#!/usr/bin/env bash
# Restore the checked-in application data snapshot into a migrated PostgreSQL DB.
#
# The target must already have this backend's migrations applied. By default the
# script protects non-empty targets. Pass --replace only when it is correct to
# remove every existing Hamsa Vilas row in the target before restoring.
#
# Example (password is requested without echoing it):
#   ./scripts/restore-current-database.sh \
#     --host aws-0-<region>.pooler.supabase.com --port 5432 \
#     --database postgres --user postgres.<project-ref> --replace

set -euo pipefail

readonly script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly backend_dir="$(cd -- "${script_dir}/.." && pwd)"
readonly dump_file="${backend_dir}/supabase/seeds/current_database_data.sql"

host=""
port="5432"
database="postgres"
user_name=""
replace_existing=false

usage() {
  cat <<'EOF'
Usage:
  ./scripts/restore-current-database.sh --host HOST --user USER [options]

Required:
  --host HOST       PostgreSQL/Supabase pooler host
  --user USER       PostgreSQL user

Options:
  --port PORT       PostgreSQL port (default: 5432)
  --database NAME   Database name (default: postgres)
  --replace         Remove existing application data before restoring
  --help            Show this message

The password is requested securely at runtime. Set PGPASSWORD only for
non-interactive automation; do not put passwords in this script or Git.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host) host="${2:?missing value for --host}"; shift 2 ;;
    --port) port="${2:?missing value for --port}"; shift 2 ;;
    --database) database="${2:?missing value for --database}"; shift 2 ;;
    --user) user_name="${2:?missing value for --user}"; shift 2 ;;
    --replace) replace_existing=true; shift ;;
    --help) usage; exit 0 ;;
    *) printf 'Error: unknown option: %s\n\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ -z "${host}" || -z "${user_name}" ]]; then
  printf '%s\n\n' 'Error: --host and --user are required.' >&2
  usage >&2
  exit 2
fi

if [[ ! "${port}" =~ ^[0-9]+$ ]]; then
  printf '%s\n' 'Error: --port must be numeric.' >&2
  exit 2
fi

if ! command -v psql >/dev/null 2>&1; then
  printf '%s\n' 'Error: psql is required. Install the PostgreSQL client and try again.' >&2
  exit 1
fi

if [[ ! -s "${dump_file}" ]]; then
  printf 'Error: data snapshot not found at %s\n' "${dump_file}" >&2
  exit 1
fi

if [[ -z "${PGPASSWORD:-}" ]]; then
  read -r -s -p "PostgreSQL password for ${user_name}: " PGPASSWORD
  printf '\n'
fi
export PGPASSWORD

psql_args=(
  --host="${host}"
  --port="${port}"
  --username="${user_name}"
  --dbname="${database}"
  --no-psqlrc
  --set=ON_ERROR_STOP=1
)

# An organization is the root record for all application data, so this exact
# query is reliable even when PostgreSQL statistics have not been refreshed.
target_has_data="$(psql "${psql_args[@]}" --tuples-only --no-align --command "
  SELECT EXISTS (SELECT 1 FROM public.organizations LIMIT 1);
")"
target_has_data="${target_has_data//[[:space:]]/}"

if [[ "${target_has_data}" == "t" && "${replace_existing}" != true ]]; then
  printf '%s\n' 'Error: target database already contains application rows.' >&2
  printf '%s\n' 'Review the target, then rerun with --replace only if it is safe to overwrite its application data.' >&2
  exit 1
fi

if [[ "${replace_existing}" == true ]]; then
  printf '%s\n' 'Replacing existing application data in the target database...'
  # TRUNCATE only application tables; it does not touch Supabase auth/storage
  # system schemas. CASCADE handles the control-plane foreign-key graph.
  psql "${psql_args[@]}" --command "
    TRUNCATE TABLE
      pool.archive_manifest, pool.audit_log, pool.away_periods, pool.beds,
      pool.branches, pool.change_requests, pool.complaint_events, pool.complaints,
      pool.floors, pool.invoice_sequences, pool.move_out_deductions,
      pool.move_out_settlements, pool.notices, pool.payments, pool.receipts,
      pool.residents, pool.rooms,
      public.device_tokens, public.digilocker_oauth_sessions, public.email_otps,
      public.enquiries, public.invitation_identity_verifications,
      public.listing_vacancy, public.manual_kyc_reviews, public.otp_codes,
      public.platform_users, public.public_listings, public.receipt_templates,
      public.refresh_tokens, public.schema_migrations_tenant,
      public.tenant_invitations, public.tenant_placement, public.trusted_devices,
      public.user_org_roles, public.verifications, public.organizations
    RESTART IDENTITY CASCADE;"
fi

printf '%s\n' 'Restoring the current database snapshot...'
psql "${psql_args[@]}" --single-transaction --file="${dump_file}"

unset PGPASSWORD
printf '%s\n' 'Restore complete.'
