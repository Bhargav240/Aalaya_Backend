# Android screen-to-API map

Android follows Compose UI → ViewModel → feature Repository → Retrofit
`ApiService` → Rust API. No screen contains SQL or Supabase queries.

| Screen | API calls | Notes |
|---|---|---|
| Login | `POST /auth/email/send-otp`, `POST /auth/email/verify-otp`, `GET /me` | Six-digit email OTP; tokens stay in DataStore. |
| Explore | `GET /discover/listings`, `/discover/search`, `/discover/nearby`, `/discover/listings/{id}` | Requires login; available to prospects and tenants. |
| Explore enquiry | `POST /listings/{branch}/enquire` | Uses logged-in user; no second OTP. |
| Your invitations | `GET /me/tenant-invitations` | Only verified-email matches. |
| Invitation detail | `GET /tenant-invitations/{id}`, onboarding status; accept/decline | Deep link `hamsavilas://onboarding/{invitationId}` contains no credential. |
| Onboarding status | `GET .../onboarding-status`, `GET .../verification` | Refreshes/polls while screen is active. |
| DigiLocker verification | `POST .../digilocker/start`, `GET .../verification` | Opens Chrome Custom Tabs; backend callback deep-links back. |
| Invitation management | `GET /tenant-invitations`, get/resend/cancel | Owner sees org; manager sees active branch only. |
| Create invitation | `POST /tenant-invitations` plus branches/rooms/beds | Room and bed are optional; manager pickers remain branch-scoped. |
| Invitation review | get/manual-review/approve/reject | Owner and correctly scoped manager only. |
| Profile | `GET /me/memberships`, `POST /auth/switch-organization`, logout | Context switch rotates refresh token. |
| Tenant home | `GET /me/ledger`, `GET /me/receipts` | Requires active tenant membership. |
| Occupancy | branches, rooms, branch beds, residents | Empty collections show a meaningful empty state. |
| Payments | branches, rooms, branch beds, residents, branch dues | Batch endpoints avoid room/resident request fan-out. |

Token refresh is centralized in `TokenAuthenticator`. Concurrent 401 responses
are single-flight under one lock because refresh tokens rotate; refresh failure
clears the session. Access tokens are attached by the OkHttp interceptor and
neither token type is logged.
