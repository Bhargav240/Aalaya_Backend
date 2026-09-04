# Aalaya PG Management System — Backend

High-performance, secure Rust backend for the **Aalaya PG Management System**. Built with Axum, SQLx, Tokio, and Supabase Postgres. Designed with a modular hexagonal architecture (ports and adapters), SOLID principles, and hybrid multi-tenancy.

---

## 🏛 Architecture & Tenancy Model

The backend leverages a **hybrid multi-tenant** architecture:
- **Control Plane (`public` schema)**: Manages platform metadata, user authentication, organization registry, and public PG exploration listings.
- **Pooled Tenancy (`pool` schema)**: Cost-effective pooled multi-tenancy with strict PostgreSQL Row Level Security (RLS) for standard organizations.
- **Siloed Tenancy (`org_<id>` schemas)**: Dedicated isolated schemas for enterprise properties requiring complete data separation.

```
crates/
├── domain/            # Pure domain models and business invariants (zero external I/O)
├── application/       # Use-case services, request context, and ports (traits)
├── infrastructure/    # Adapters: SQLx Postgres repositories, Argon2, JWT, SMS/FCM, Digilocker
└── api/               # Axum HTTP layer: REST routes, auth extractors, middleware, problem+json
bin/
└── server/            # Application entrypoint and dependency composition root
migrations/
├── control_plane/     # Public schema migrations
└── tenant/            # Tenant-scoped schema migrations
```

---

## 🛠 Tech Stack

- **Language & Runtime**: [Rust](https://www.rust-lang.org/) (2021 edition) + [Tokio](https://tokio.rs/) async runtime
- **Web Framework**: [Axum](https://github.com/tokio-rs/axum)
- **Database Access**: [SQLx](https://github.com/launchbadge/sqlx) with compile-time checked queries
- **Database**: [PostgreSQL 17 / Supabase](https://supabase.com/)
- **Authentication**: Passwordless Email OTP + Argon2id hashing + Rotating refresh tokens (HS256 access JWT)
- **Object Storage**: Local filesystem adapter or Supabase S3-compatible storage (swappable behind `ObjectStore` port)

---

## 🚀 Getting Started

### Prerequisites
- [Rust & Cargo](https://rustup.rs/) (1.80+)
- PostgreSQL 17 or a [Supabase](https://supabase.com) project
- (Optional) Docker & Docker Compose

### Environment Setup
1. Copy the example environment file:
   ```bash
   cp .env.example .env
   ```
2. Configure the essential environment variables in `.env`:
   - `DATABASE_URL`: Supabase Transaction Pooler connection string (port 6543)
   - `MIGRATION_DATABASE_URL`: Direct database connection (port 5432) for startup migrations
   - `JWT_SECRET`: Random 64-character secret
   - `PROVISIONING_KEY`: Key for organization provisioning

### Running the Server
```bash
cargo run -p pg-server
```
Check server health:
```bash
curl http://127.0.0.1:8080/health
```

### Docker Local Development
To launch local PostgreSQL and the backend with migrations applied automatically:
```bash
docker compose -f compose.dev.yml up -d --build
```

---

## 📡 API Overview (`/api/v1`)

### Authentication & Sessions
- `POST /auth/email/send-otp`: Request secure sign-in OTP
- `POST /auth/email/verify-otp`: Verify OTP and receive rotating tokens `{access_token, refresh_token}`
- `POST /auth/token/refresh`: Rotate refresh token and fetch fresh access token
- `POST /auth/logout`: Server-side token invalidation

### Operations & Management
- `GET  /orgs/me`: Current organization and user context
- `GET  /branches`: List branches in the organization
- `POST /branches`: Create a new branch
- `GET  /branches/{id}/beds`: Real-time bed occupancy and assigned resident lookup
- `GET  /branches/{id}/dues`: Aggregated rent, accrued fees, and outstanding ledger summary

### Resident & Ledger Management
- `GET  /residents`: List active/pending residents
- `POST /residents`: Onboard resident with daily/weekly/monthly billing plan
- `POST /residents/{id}/payments`: Record rent/deposit payment (with `Idempotency-Key` protection)
- `GET  /receipts/{id}/download`: Fetch or redirect to signed PDF receipt
- `POST /residents/{id}/move-out`: Settle ledger, calculate deductions, and free room/bed

### Public Discovery & Marketplace
- `GET /discover/listings`: Browse approved public PG accommodations
- `GET /discover/nearby?lat=&lng=&limit=`: Geo-proximity search (Haversine formula)
- `GET /discover/search?q=&city=`: Text and city filter for PG listings

---

## 🧪 Testing & Verification

Run the test suite:
```bash
# Unit & domain logic tests
cargo test --workspace

# Database integration tests (requires MIGRATION_DATABASE_URL)
cargo test -p pg-infrastructure --test tenant_activation_tests
```

---

## 📚 Documentation

Detailed architectural and setup runbooks are located under `docs/`:
- `docs/environment-configuration.md`: Comprehensive environment variables guide
- `docs/tenant-onboarding.md`: Full walkthrough of tenant invitation and activation flows
- `docs/android-screen-api-map.md`: Map of Android Jetpack Compose screens to backend endpoints
- `docs/digilocker-partner-setup.md`: Identity verification integration details
- `docs/resend-setup.md`: Production email delivery configuration
