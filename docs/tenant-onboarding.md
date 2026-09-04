# Tenant onboarding lifecycle

## Roles and contexts

An account has one global identity (verified email) and zero or more active
organization memberships. Role belongs to a membership, not permanently to the
person. The same account can therefore be:

- platform organization → `prospect`, for Explore and enquiries;
- PG A → `tenant`, for PG A private resident features;
- another organization → a separate role, if explicitly granted.

The access JWT carries the selected organization and role. `GET /me/memberships`
lists valid contexts; `POST /auth/switch-organization` rotates the session into
one selected active context. A tenant can switch back to prospect context and
continue exploring, and Explore also remains visible in tenant context.

## Lifecycle

```text
invited
  ├─ cancel/expire ───────────────> cancelled | expired
  └─ tenant accepts by verified email
                    └─────────────> email_verified
                                      ├─ KYC not required -> approval_pending
                                      └─ KYC required ----> kyc_pending
                                                              ├─ failed -> kyc_failed
                                                              └─ verified -> kyc_verified
                                                                               └─ approval_pending
approval_pending
  ├─ reject ──────────────────────> rejected
  └─ owner/scoped manager approves
                    └─────────────> active membership + resident stay
```

Invalid transitions return HTTP 409. Invitation acceptance requires a logged-in
account whose verified email matches; email is never trusted from a request
body. Platform operators cannot inspect onboarding records by default.

## Atomic activation

Approval locks the invitation and performs all membership, resident, optional
bed, invitation, and audit writes in one database transaction. It rechecks KYC,
email, organization, branch, room, and bed availability after acquiring locks.
A conflict or failure leaves no partial tenant membership or resident.

Only `role=tenant` plus `membership_status=active` unlocks private tenant APIs.
Pending invitations/memberships do not. Moving out changes the membership to
`moved_out`. There can be only one live resident stay per membership and one
live resident per bed.

## Identity verification

The default production provider is DigiLocker. The backend stores only a safe
provider reference, document type, status, timestamp, and safe failure reason.
It does not store raw Aadhaar data or full documents. When organization config
permits it, an owner or correctly scoped manager can record a reasoned manual
review; reviewer ID and request ID are retained for auditability.
