# Resend email setup

1. Add a sending domain in Resend.
2. Copy the DNS records Resend supplies into the domain's DNS provider.
3. Wait until SPF and DKIM show verified. Add the recommended DMARC policy and
   tighten it gradually after monitoring delivery.
4. Create a production API key with only the permissions needed to send email.
5. Store it as `RESEND_API_KEY` in the backend secret manager.
6. Set `EMAIL_PROVIDER=resend` and a verified `EMAIL_FROM`, for example
   `Hamsa Vilas <no-reply@notifications.example.com>`.
7. Send test login, invitation, resend, KYC-required, approved, and rejected
   messages to multiple providers before launch.

The backend keeps login OTP delivery separate from invitation notifications.
Invitation links contain only an invitation UUID and still require email-OTP
login. They never contain an OTP, JWT, refresh token, email address, DigiLocker
token, or tenant details.

Monitor bounces, complaints, suppression lists, and domain reputation. Do not
log raw OTPs, email provider authorization headers, or full message bodies.
Production startup fails when Resend configuration is incomplete; this prevents
a deployment that accepts login requests but cannot deliver codes.
