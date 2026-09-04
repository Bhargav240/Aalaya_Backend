# DigiLocker partner setup

Live DigiLocker verification requires approval outside this repository.

1. Register the organization as a DigiLocker Requester and complete the
   applicable legal/compliance review.
2. Register an HTTPS backend callback exactly matching:
   `https://<api-host>/api/v1/integrations/digilocker/callback`.
3. Request only the minimum approved scopes. Do not request document access
   merely because the provider supports it.
4. Store the client ID, client secret, redirect URI, and session encryption key
   in the backend secret manager. Never add them to Android or source control.
5. Set the `DIGILOCKER_*` environment values described in
   `environment-configuration.md` and restart the backend.
6. Configure Android for the `hamsavilas://onboarding` scheme and test both a
   successful callback and provider cancellation.

Security properties implemented by the backend:

- random OAuth state and nonce;
- PKCE S256 verifier;
- stored state/nonce hashes and encrypted verifier;
- ten-minute session expiry by default;
- single-use callback consumption and replay rejection;
- backend-only authorization-code exchange;
- nonce/provider validation;
- minimal result storage; no provider token returned to Android.

Before production, test wrong/expired state, wrong nonce, replay, provider
failure, Android app-not-installed behavior, and the approved manual fallback.
Confirm the provider's current production endpoints and policy during partner
onboarding rather than assuming sandbox endpoints are permanent.
