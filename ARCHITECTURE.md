# PG Management System — Backend Architecture Plan

**Project:** Hamsa Vilas PG Management System — backend service
**Scope:** Backend only. One backend serving many frontends (web, Android, iPhone) now, and reusable for other products later.
**Sources:** the PRD (`PG Management System Architecture and Requirements.md`) and the Figma design-system file.
**Status:** Proposed design, pending your review before scaffolding.

---

## 0. Decisions locked in

| Area | Decision | Why |
| :--- | :--- | :--- |
| Language / runtime | **Rust + Tokio (async)** | No GC pauses, predictable latency, strong compile-time guarantees. |
| Web framework | **Axum** | Tower-based, minimal magic, trait/middleware friendly, first-class Tokio. |
| Data access | **SQLx** (compile-time-checked SQL, async, Postgres) | Type-safe queries without a heavy ORM; fits the trait-repository approach. |
| Database host | **Supabase (Postgres)** — free tier first, upgrade to scale | Managed Postgres + Storage + pooler; nothing proprietary we can't leave. |
| **Multi-tenancy** | **Hybrid, three-schema.** `public` = control-plane/routing; a named `pool` schema = pooled small/single-branch orgs (isolated by `org_id` + RLS); `org_<id>` = one schema per siloed org. | Routing lives where the resolver can always reach it; pooled and siloed tenant schemas are symmetric; cheap density + hard isolation with a pool→silo upgrade path. |
| Auth | **Custom, trait-based** (own JWT, Argon2id, invite tokens, DigiLocker) | PRD flows are custom; avoids lock-in to Supabase Auth. |
| API style | **REST + OpenAPI** (`/api/v1`, `utoipa`-generated spec) | Simplest contract for three independent frontend teams. |
| Logging / observability | **`tracing` + `tracing-subscriber`** as the framework; JSON in prod, pretty in dev | Structured, span-based, context-propagating logs. |
| Request/response logging | Top-of-stack middleware logs **every** request and response, including failures | Full audit of traffic; PII redacted; bodies only on error/at debug. |
| Error handling | Typed errors + a **single top-level error boundary** (`?` propagation → `problem+json`) | Consistent responses, one place that logs failures. |
| Ambient context | **`tokio::task_local!`** RequestContext (request id, actor, org) | Cross-cutting data available without threading it through every function. |

> **Launch posture on tenancy:** the hybrid model is *designed in* from day one (placement registry, resolver, both `search_path` paths), but **every org launches pooled**. We only build and run the promotion job when a tenant actually needs a silo — so the door is open at zero upfront cost.

---

## 1. Guiding principles

The point of this backend is *move fast later*, which means keeping the cost of change low. Two ideas do that:

**Hexagonal architecture (Ports & Adapters).** Business rules sit in the middle and know nothing about Axum, Postgres, Supabase, JWTs, or DigiLocker. Everything external is reached through a **trait** (a "port"); each concrete technology is an **adapter** behind it. Swapping stores, storage backends, or a mock verifier in tests is a new adapter, not a rewrite. This is also exactly what makes the hybrid tenancy model possible without disturbing business logic.

**SOLID, concretely:** single responsibility (rent math lives in the billing service, not an HTTP handler); open/closed (new payment method or storage backend = new adapter); Liskov (real repo ↔ in-memory test repo are interchangeable); interface segregation (many small repository traits, not one giant `Database`); dependency inversion (application depends on trait abstractions, concretes injected at startup).

**Everything that touches the outside world is a trait** — db, storage engine, auth, identity verification, clock, id generation, notifications — so the core is fully testable in memory with zero infrastructure.

---

## 2. Repository layout & the Cargo workspace

Intended top-level layout (Android lives beside the backend, versioned independently):

```
hamsa-vilas/                     # project root (create when you grant the parent folder)
├── backend/                     # this Rust workspace
└── android/                     # Android app (separate build, added later)
```

The backend is a Cargo **workspace** of small crates; inner crates cannot import outer ones, so the dependency direction is enforced by the compiler:

```
backend/
├── Cargo.toml                   # [workspace] members
├── crates/
│   ├── domain/                  # pure business types & rules — NO external deps
│   │   ├── entities/            # Organization, Branch, Room, Bed, Resident, Payment...
│   │   ├── value_objects/       # Money, Rent, Deposit, PhoneNumber, AadhaarName...
│   │   ├── enums/               # Role, ComplaintState, PlanKind, ChangeRequestStatus
│   │   └── errors.rs            # DomainError (thiserror)
│   │
│   ├── application/             # use cases + PORTS (traits). Depends only on `domain`
│   │   ├── ports/               # repository traits, StorageEngine, AuthProvider,
│   │   │                        #   TenantResolver, ConnectionProvider, Clock, Notifier...
│   │   ├── services/            # use cases: RegisterResident, RecordPayment,
│   │   │                        #   ApproveChangeRequest, SettleMoveOut, RaiseComplaint...
│   │   └── context.rs           # RequestContext (ambient, task-local)
│   │
│   ├── infrastructure/          # ADAPTERS. Depends on `application` + `domain`
│   │   ├── persistence/         # SQLx repos, UnitOfWork, tenancy resolver, migration runner
│   │   ├── storage/             # Supabase Storage / S3 / local-FS ObjectStore adapters
│   │   ├── auth/                # Argon2 hasher, JWT token service, invite tokens
│   │   ├── identity/            # DigiLocker client + mock verifier
│   │   ├── observability/       # tracing setup, log middleware helpers
│   │   └── config/              # env/secret loading
│   │
│   ├── api/                     # Axum HTTP layer. Depends on `application`
│   │   ├── routes/              # /auth /orgs /branches /residents /payments ...
│   │   ├── dto/                 # request/response bodies (serde) + validation
│   │   ├── middleware/          # logging, context, auth extraction, tenant resolution, RBAC, errors
│   │   ├── error.rs             # AppError + IntoResponse (the top-level boundary)
│   │   └── openapi.rs           # utoipa spec + Swagger UI
│   │
│   └── worker/                  # background jobs: monthly archive, billing schedules, promotion job
│
├── bin/
│   └── server/                  # composition root: build adapters, inject, run Axum
│
└── migrations/
    ├── control_plane/           # builds the routing/control-plane tables in `public`
    └── tenant/                  # tenant table migrations, applied to `pool` AND every `org_<id>` schema
```

Dependency arrows point inward only: `api` → `application` → `domain`; `infrastructure` → `application` → `domain`; `domain` depends on nothing. The `server` binary is the single composition root that knows every concrete type.

---

## 3. Multi-tenancy: the three-schema routing model

Three kinds of schema, each with one job:

- **`public` — control plane & routing.** The tables the resolver reads *before* it knows the tenant. `public` is always on the `search_path`, which is exactly why routing belongs here: the resolver's first query ("which schema does this org live in?") must run before any tenant context exists, so it has to hit a schema that's reachable with no setup.
- **`pool` — the pooled tenant schema.** All small/single-branch orgs' tenant data, every row tagged `org_id`, isolated by `org_id` filtering **and** Postgres RLS. This is the default placement.
- **`org_<uuid>` — one schema per siloed org.** Same tenant tables, physically separated. For orgs that need or earn isolation.

`pool` and every `org_<uuid>` are **symmetric** — identical tables, identical migration set. The only difference is `pool` holds many orgs (so RLS matters) and each silo holds one. A single-branch PG is just an org with one `branch` row, in either placement — no special-casing anywhere.

### Control plane in `public`

- `organizations` — one row per PG business.
- `platform_users` — global identities (email/phone, Argon2 hash, status).
- `user_org_roles` — role per user, and (for managers) the branch they're scoped to.
- `tenant_placement` — **the routing table: org_id → mode (`pooled` | `siloed`) + schema_name + status**. New orgs default to `pooled` / `pool`.
- `schema_migrations_tenant` — which tenant-migration versions each tenant schema (`pool` + every silo) is on.

### Tenant tables (in `pool` and every `org_<uuid>`)

Defined once in `migrations/tenant/*`: `branches`, `rooms`, `beds`, `residents`, `resident_verifications`, `rent_plans`, `charges`, `payments`, `receipts`, `deposits`, `complaints`, `complaint_events`, `notices`, `change_requests`, `audit_log`, `archive_manifest`. Every one carries `org_id` (essential in `pool`, harmless in a silo) and `deleted_at`.

### The resolver + connection ports

```rust
pub enum Placement { Pooled, Siloed { schema: String } }  // Pooled → schema "pool"
pub struct TenantContext { pub org_id: OrgId, pub placement: Placement }

#[async_trait]
pub trait TenantResolver: Send + Sync {
    async fn resolve(&self, org_id: OrgId) -> Result<TenantContext, TenancyError>;
}

#[async_trait]
pub trait ConnectionProvider: Send + Sync {
    async fn begin(&self, ctx: &TenantContext) -> Result<UnitOfWork, PersistenceError>;
}
```

**Middleware flow, per request:** auth extracts `org_id` from the JWT → the resolver looks it up in `public.tenant_placement` → yields `pool` or `org_<uuid>` → the connection provider, inside the transaction, runs:

```sql
SET LOCAL search_path TO "<pool | org_uuid>", public;   -- tenant schema first, public for control-plane/extensions
SET LOCAL app.current_org = '<org_id>';
```

`SET LOCAL` is transaction-scoped, so it's safe with Supabase's transaction-mode pooler and never leaks to the next request on that connection. Repositories write **unqualified** table names and land in the resolved schema automatically — identical code for pooled and siloed. Control-plane queries reference `public.*` explicitly.

**Cache the routing lookup.** `tenant_placement` is read on every request but changes almost never (only on onboarding or a pool→silo promotion). The DB-backed resolver is wrapped by a caching decorator (in-memory `org_id → placement` map, populated lazily, invalidated when a placement changes) so routing costs no per-request round-trip. Because it's all behind the `TenantResolver` trait, the cache is just another adapter.

### Isolation, defense in depth

- **`pool`:** RLS policies `USING (org_id = current_setting('app.current_org')::uuid)` on every tenant table, *plus* the repository trait always filtering by `org_id`. Two independent barriers — a buggy query still can't cross orgs. (Same RLS mechanism Supabase itself is built on; pooled is logically isolated, not "insecure.") RLS on `pool` is kept **always**.
- **`org_<uuid>`:** physical schema separation; the same filter/RLS applies but is largely redundant, which is fine.

### Migrations & provisioning

- **New org** → insert into `public.organizations` + `public.tenant_placement` (mode `pooled` → schema `pool`). Nothing else — pooled orgs use the already-migrated `pool` tables.
- **Schema evolution** → the migration runner applies `migrations/control_plane/*` to `public`, then applies pending `migrations/tenant/*` to `pool` and iterates every siloed schema in `tenant_placement`. Tracked per schema in `schema_migrations_tenant`, idempotent, safe to re-run at deploy.
- **Promotion (pool → silo)** — a `worker` job: `CREATE SCHEMA org_<uuid>`, apply tenant migrations, copy that org's rows out of `pool` into it, verify row counts, flip `tenant_placement` to `siloed` + invalidate the cache, then purge the org's rows from `pool`. Resumable; run in a low-traffic window. **Built only when the first tenant needs it.**

> **Open policy to confirm:** what triggers promotion. Proposed default — **plan tier or explicit request** (premium/enterprise org, or one the owner asks to isolate), with an optional size threshold as a nudge. Say the word if you'd prefer purely size-based or purely manual.

---

## 4. Cross-cutting: logging, errors, and ambient context

These three are wired once, at the edge, and everything else inherits them.

### Logging & observability (`tracing`)
`tracing` + `tracing-subscriber` is the framework — structured JSON logs in prod, pretty logs in dev, levels via `EnvFilter`. A **top-of-stack Tower middleware** logs **every** request and response: method, path, request id, status, and latency, for successes *and* failures. Guardrails, because these payloads carry PII (Aadhaar names, phones, receipts):

- Response bodies are logged **only on error, or at debug level** — never full-body at info on every 200, which would be both a privacy liability and a cost problem.
- Known-sensitive fields are redacted by the logging layer.
- Every log line automatically carries `request_id`, `org_id`, and `actor` from the request span.

### Error propagation — one boundary at the top
Domain and application code return typed `Result`s and propagate with `?`. `DomainError` (via `thiserror`) lives in the domain; at the HTTP edge a single `AppError` enum implements `IntoResponse`, which does two things together: maps the error to a clean `application/problem+json` response **and** emits a `tracing` event with full context. Expected failures (4xx, domain violations) log at warn with a safe client message; unexpected failures (5xx) log at error with full internal detail but return a sanitized message. Handlers carry no logging or error-shaping boilerplate — it all happens in this one place.

### Ambient context — task-local, not thread-local
A `tokio::task_local!` holds a `RequestContext { request_id, actor, org_id, … }`. The context middleware sets it once per request (scoping the rest of the handling inside it) and opens a matching `tracing` span. Anything downstream — the **audit-log writer** especially, which needs "who is acting" — reads it without it ever appearing in a function signature.

Thread-local is deliberately **rejected**: Tokio moves tasks between worker threads at every `.await`, so a thread-local set before an await can read back another task's value. A task-local is bound to the task and follows it across threads.

**The guardrail that protects the SOLID goal:** the task-local is only for genuinely *ambient* cross-cutting data (request id, actor, trace context). Real dependencies — repositories, config, the storage engine — stay explicitly injected through their traits, so the code remains testable. Ambient context in the task-local; real dependencies through the ports.

---

## 5. The core ports (traits)

Defined in `application/ports`, implemented in `infrastructure`:

```rust
// Persistence: one focused repository per aggregate (Interface Segregation)
#[async_trait] pub trait ResidentRepository  { /* create, by_id, active_in_branch, soft_delete */ }
#[async_trait] pub trait PaymentRepository   { /* record, ledger_for_resident */ }
#[async_trait] pub trait ComplaintRepository { /* open, transition, visible_to */ }
#[async_trait] pub trait ChangeRequestRepository { /* enqueue, pending_for_org, mark */ }
#[async_trait] pub trait AuditLog { async fn append(&self, entry: AuditEntry) -> Result<()>; }
// ...Branch/Room/Bed/Notice/Deposit repositories

pub trait UnitOfWork { /* the repos above, bound to one transaction; commit()/rollback() */ }

// Storage engine (receipts, archived records, verification docs)
#[async_trait] pub trait ObjectStore: Send + Sync {
    async fn put(&self, key: &str, bytes: Bytes, content_type: &str) -> Result<()>;
    async fn get(&self, key: &str) -> Result<Bytes>;
    async fn signed_url(&self, key: &str, ttl: Duration) -> Result<Url>;
    async fn delete(&self, key: &str) -> Result<()>;
}   // Adapters: SupabaseStorage (S3-compatible), AwsS3, LocalFs (dev)

// Auth & identity
pub trait PasswordHasher   { /* hash, verify — Argon2id */ }
pub trait TokenService     { /* issue_access, issue_refresh, verify — JWT */ }
#[async_trait] pub trait InviteService    { /* create, redeem — single-use, expiring */ }
#[async_trait] pub trait IdentityVerifier { /* verify — DigiLocker + mock */ }

// Small cross-cutting ports (keep the core deterministic & testable)
pub trait Clock       { fn now(&self) -> DateTime<Utc>; }
pub trait IdGenerator { fn new_id(&self) -> Uuid; }
#[async_trait] pub trait Notifier { /* notify — push/email/in-app */ }
#[async_trait] pub trait ReceiptRenderer { /* render → PDF */ }
pub trait BillingEngine { fn charges_for_period(&self, plan: &RentPlan, period: Period) -> Vec<Charge>; }
```

In tests these are in-memory fakes; in production they're the real adapters — same contract.

---

## 6. Domain rules that need real care

**Roles, scope & authorization.** Owner (org-wide, no approval), Manager (one branch), Tenant/resident (self only). A dedicated `AuthorizationService` answers `can(actor, action, scope)` where scope is Org / Branch / Self; every use case asks it before mutating. PRD §10's matrix is the single source of truth for this policy table.

**Maker-checker (PRD §5).** Manager *creates* apply immediately. Manager *edits/deletes* write a `change_requests` row (target entity + id, operation, proposed values as structured JSON, requester, timestamp, status `pending`) instead of mutating. Owner edits/deletes apply immediately. Owner **approve** runs the mutation inside one transaction + writes audit + flips status; **reject** leaves the record untouched. A typed applier dispatches per entity, so approvals stay type-safe rather than a blind JSON overwrite.

**Soft-delete + monthly archive + audit (PRD §9).** Every table has `deleted_at`; repositories filter `deleted_at IS NULL` by default. The **audit log** is append-only (actor, action, entity, before/after, approver, request id, timestamp) written *in the same transaction* as the mutation. A monthly `worker` job sweeps rows soft-deleted past a retention threshold to the `ObjectStore` as an archive manifest (cheap cold storage), then purges them from hot DB — reversible via the manifest.

**Billing & deposits (PRD §8).** Per-bed monthly/weekly/daily plans; part-months prorate by day; dues = charged − paid. Deposits are tracked separately ("Deposit held: ₹X"). `SettleMoveOut` computes rent-to-last-day + dues, applies itemized deductions (each with a reason), returns the balance, and forfeits when notice was insufficient (default 30 days) — producing the "collected ₹X, deducted ₹Y (reasons), refunded ₹Z" record.

**Payments → instant receipt (PRD §7.2).** `RecordPayment` (manager, cash): write payment → recompute dues → render `Receipt` (PDF) → store in `ObjectStore` → resident fetches a signed URL. One transaction + audit entry.

**Complaints state machine (PRD §7.3).** `Open → Assigned → In progress → Resolved → Closed`, with a legal `Resolved → In progress` bounce-back, modeled as typed states with an explicit allowed-transitions table. Visibility: residents see only their own; managers their branch; owner sees all, incl. business-level complaints hidden from managers.

**Notices (PRD §7).** Manager → own branch; owner → all. Delivered via the `Notifier` port. Read-receipts/scheduling are PRD "later" items the port already accommodates.

**Identity verification (PRD §7.1).** First login runs DigiLocker + phone + emergency contact through `IdentityVerifier`; registration uses the exact Aadhaar name. The DigiLocker-failure / name-mismatch fallback is an open PRD question; the port returns a rich result so we can implement the policy once you decide it.

---

## 7. API surface (REST, `/api/v1`, OpenAPI via utoipa)

Grouped by resource, every endpoint gated by the authorization policy. Illustrative:

| Group | Endpoints (sample) |
| :--- | :--- |
| Auth | `POST /auth/invite/redeem`, `POST /auth/login`, `POST /auth/refresh`, `POST /auth/verify-identity` |
| Orgs | `POST /orgs`, `GET /orgs/me` |
| Branches | `GET/POST /branches`, `GET /branches/{id}`, `PATCH /branches/{id}` (→ maker-checker) |
| Rooms & Beds | `GET/POST /branches/{id}/rooms`, `.../beds`, availability summary |
| Residents | `POST /residents/invite`, `GET /residents`, `GET/PATCH /residents/{id}` |
| Payments | `POST /payments` (cash), `GET /residents/{id}/ledger`, `GET /receipts/{id}` (signed URL) |
| Complaints | `POST /complaints`, `GET /complaints`, `POST /complaints/{id}/transition` |
| Notices | `POST /notices`, `GET /notices` |
| Move-out | `POST /residents/{id}/move-out`, deposit settlement |
| Approvals | `GET /change-requests`, `POST /change-requests/{id}/approve\|reject` |
| Finance | `GET /finance/branch/{id}`, `GET /finance/org` (owner only) |
| Audit | `GET /audit` (scoped by role) |

Cross-cutting: versioned paths; `problem+json` errors; request ids + structured `tracing` logs; pagination/filtering conventions; OpenAPI generated from handlers so the three frontend teams always have an accurate contract to codegen clients from.

---

## 8. Testing, quality & CI

Domain/application: pure unit tests with in-memory fakes for every port — fast, no DB (the payoff of the trait design). Infrastructure: integration tests against real Postgres via **testcontainers**, including a full provisioning + migration round-trip for both a pooled and a siloed org. API: contract tests validating responses against the OpenAPI schema. CI gates on every PR: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo sqlx prepare --check`.

---

## 9. Build roadmap

| Phase | Deliverable |
| :--- | :--- |
| **0 · Foundation** | Workspace + crates, config/secrets, Supabase connection + pooler, `tracing` setup, logging + context + error middleware, health check, `public` control-plane + `pool` schema, cached tenancy resolver (pooled path) + routing table, migration runner, CI. |
| **1 · Auth & identity** | Custom auth (Argon2id, JWT access/refresh), invite → set-password → first-login, roles + RBAC middleware, `TenantResolver` wired into every request. |
| **2 · Core domain** | Org/Branch/Room/Bed/Resident lifecycle with maker-checker, audit log, soft-delete. |
| **3 · Money** | Billing engine (monthly/weekly/daily, proration), cash payments, PDF receipts, deposits. |
| **4 · Ops** | Complaints state machine + visibility, notices via `Notifier`. |
| **5 · Move-out & finance** | Move-out + deposit settlement, branch/org finance reports, DigiLocker integration. |
| **6 · Hardening & scale** | Monthly archive worker, **pool→silo promotion job**, observability polish, load testing, security review. |

---

## 10. Open questions carried from the PRD (§11)

Don't block Phase 0–2, but needed before the phases that touch them: (1) do daily/weekly guests pay a deposit and do full ID checks; (2) short stays paid upfront or at checkout; (3) DigiLocker-failure / Aadhaar-name-mismatch fallback policy; (4) notice read-receipts now or later; (5) archive retention period (confirm with your CA). Plus the tenancy item above: (6) the pool→silo promotion trigger.

---

*Next step once you're happy: I scaffold **Phase 0** — the compiling workspace skeleton with the trait contracts, `tracing`/logging/error/context middleware, config, the `platform` schema, the pooled tenancy resolver, and the migration runner.*
