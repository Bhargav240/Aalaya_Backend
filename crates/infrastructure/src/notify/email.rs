use async_trait::async_trait;
use pg_application::ports::{
    EmailSender, InvitationEmail, InvitationEmailKind, InvitationEmailSender,
};
use pg_application::AppError;

pub struct LogEmailSender {
    expose_code: bool,
}

impl LogEmailSender {
    pub fn new(expose_code: bool) -> Self {
        Self {
            expose_code: cfg!(debug_assertions) && expose_code,
        }
    }
}
#[async_trait]
impl EmailSender for LogEmailSender {
    async fn send_login_otp(
        &self,
        _email: &str,
        _recipient_name: Option<&str>,
        code: &str,
        ttl_secs: i64,
    ) -> Result<(), AppError> {
        if self.expose_code {
            tracing::warn!(otp_code = code, ttl_secs, "development email OTP");
        } else {
            tracing::info!(ttl_secs, "email login OTP requested (development sender)");
        }
        Ok(())
    }
}

pub struct ResendEmailSender {
    api_key: String,
    from: String,
    pg_name: String,
    client: reqwest::Client,
}
impl ResendEmailSender {
    pub fn new(api_key: String, from: String, pg_name: String) -> Self {
        Self {
            api_key,
            from,
            pg_name,
            client: reqwest::Client::new(),
        }
    }
}
#[async_trait]
impl EmailSender for ResendEmailSender {
    async fn send_login_otp(
        &self,
        email: &str,
        recipient_name: Option<&str>,
        code: &str,
        ttl_secs: i64,
    ) -> Result<(), AppError> {
        let recipient = recipient_name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("there");
        tracing::warn!(otp_code = code, email, "Resend login OTP");
        let minutes = (ttl_secs + 59) / 60;
        let body = serde_json::json!({
            "from": self.from,
            "to": [email],
            "subject": format!("OTP for Secure Login - {}", self.pg_name),
            "text": format!(
                "Dear {recipient},\n\n{code} is your OTP for {}.\nIt is valid for {minutes} minutes.\n\nRegards,\n{}",
                self.pg_name, self.pg_name
            ),
            "html": format!(
                "<p>Dear {},</p><p><strong>{}</strong> is your OTP for <strong>{}</strong>.<br>It is valid for {} minutes.</p><p>Regards,<br>{}</p>",
                recipient, code, self.pg_name, minutes, self.pg_name
            )
        });
        let response = self
            .client
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                tracing::error!(error = %error, "Resend login email request failed");
                AppError::Internal("email delivery unavailable".into())
            })?;
        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!(%status, response = %body, "Resend rejected login email");
            Err(AppError::Internal("email delivery unavailable".into()))
        }
    }
}

pub struct LogInvitationEmailSender;

#[async_trait]
impl InvitationEmailSender for LogInvitationEmailSender {
    async fn send(&self, email: InvitationEmail) -> Result<(), AppError> {
        tracing::info!(
            invitation_id = %email.invitation_id,
            notification = email.kind.as_str(),
            "tenant invitation email queued (development sender)"
        );
        Ok(())
    }
}

pub struct ResendInvitationEmailSender {
    api_key: String,
    from: String,
    deep_link_base: String,
    client: reqwest::Client,
}

impl ResendInvitationEmailSender {
    pub fn new(api_key: String, from: String, deep_link_base: String) -> Self {
        Self {
            api_key,
            from,
            deep_link_base,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl InvitationEmailSender for ResendInvitationEmailSender {
    async fn send(&self, email: InvitationEmail) -> Result<(), AppError> {
        let link = format!(
            "{}/{}",
            self.deep_link_base.trim_end_matches('/'),
            email.invitation_id
        );
        let content = invitation_content(email.kind, &link, email.invitation_id);
        let body = serde_json::json!({
            "from": self.from,
            "to": [email.recipient],
            "subject": content.subject,
            "text": content.text,
            "html": content.html,
        });
        let response = self
            .client
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                tracing::error!(error = %error, "Resend invitation email request failed");
                AppError::Internal("email delivery unavailable".into())
            })?;
        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!(%status, response = %body, "Resend rejected invitation email");
            Err(AppError::Internal("email delivery unavailable".into()))
        }
    }
}

struct InvitationContent {
    subject: &'static str,
    text: String,
    html: String,
}

/// Whether the configured deep-link base is something a mail client will
/// actually render as a link.
///
/// Gmail only linkifies well-known schemes, and its sanitizer drops `href`s
/// that use anything else — so a `hamsavilas://` URL reaches the reader as
/// inert text no matter whether it is sent as plain text or as HTML. When the
/// base is a custom app scheme we therefore do not show a dead link at all;
/// we route the reader through the Invites tab instead, which reaches the same
/// screen via GET /me/tenant-invitations. Point APP_DEEP_LINK_BASE at an
/// https:// App Link and this file starts emitting real buttons with no
/// further change.
fn is_web_link(base: &str) -> bool {
    let base = base.trim().to_ascii_lowercase();
    base.starts_with("https://") || base.starts_with("http://")
}

fn invitation_content(
    kind: InvitationEmailKind,
    link: &str,
    invitation_id: uuid::Uuid,
) -> InvitationContent {
    let (subject, lead, cta) = match kind {
        InvitationEmailKind::Created => (
            "You are invited to join Hamsa Vilas",
            "You have a tenant invitation waiting for you.",
            "Open your invitation",
        ),
        InvitationEmailKind::Resent => (
            "Your Hamsa Vilas invitation",
            "Your tenant invitation is ready.",
            "Open your invitation",
        ),
        InvitationEmailKind::KycRequired => (
            "Identity verification is required",
            "Your invitation needs identity verification before it can be approved.",
            "Complete verification",
        ),
        InvitationEmailKind::Approved => (
            "Your tenant onboarding is approved",
            "Your onboarding was approved.",
            "Open the app",
        ),
        InvitationEmailKind::Rejected => (
            "Tenant onboarding update",
            "Your onboarding request was not approved.",
            "View the update",
        ),
    };

    let (text, html) = if is_web_link(link) {
        (
            format!(
                "{lead}\n\n{cta}:\n{link}\n\n\
                 If that link does not open the app, sign in to the Hamsa Vilas app \
                 with this email address and open the Invites tab.\n\n\
                 Invitation reference: {invitation_id}"
            ),
            format!(
                "<p>{lead}</p>\
                 <p><a href=\"{link}\" style=\"background:#2E7D32;color:#ffffff;\
                 padding:12px 20px;border-radius:6px;text-decoration:none;\
                 display:inline-block;font-weight:600\">{cta}</a></p>\
                 <p style=\"color:#555\">If that button does not open the app, sign in to the \
                 Hamsa Vilas app with this email address and open the <strong>Invites</strong> \
                 tab.</p>\
                 <p style=\"color:#888;font-size:12px\">Invitation reference: {invitation_id}</p>"
            ),
        )
    } else {
        // No link is shown on purpose: a custom-scheme URL is not clickable in
        // Gmail, and printing one only invites the "the link does nothing"
        // report this copy exists to prevent.
        (
            format!(
                "{lead}\n\n\
                 To continue, open the Hamsa Vilas app, sign in with this email address \
                 using the one-time code, and open the Invites tab.\n\n\
                 Invitation reference: {invitation_id}"
            ),
            format!(
                "<p>{lead}</p>\
                 <p>To continue, open the <strong>Hamsa Vilas</strong> app, sign in with this \
                 email address using the one-time code, and open the <strong>Invites</strong> \
                 tab.</p>\
                 <p style=\"color:#888;font-size:12px\">Invitation reference: {invitation_id}</p>"
            ),
        )
    };

    InvitationContent {
        subject,
        text,
        html,
    }
}

#[cfg(test)]
mod invitation_content_tests {
    use super::*;

    const ID: uuid::Uuid = uuid::Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);

    #[test]
    fn custom_scheme_base_never_emits_a_dead_link() {
        // The regression this guards: Gmail renders `hamsavilas://...` as inert
        // text, so an invitation whose only call to action is that URL leaves
        // the reader with nothing to tap.
        let content = invitation_content(
            InvitationEmailKind::Created,
            "hamsavilas://onboarding/0123",
            ID,
        );
        for body in [&content.text, &content.html] {
            assert!(!body.contains("hamsavilas://"), "dead link leaked: {body}");
            assert!(body.contains("Invites"), "no usable path offered: {body}");
            assert!(body.contains(&ID.to_string()));
        }
    }

    #[test]
    fn web_base_emits_a_real_link_in_both_bodies() {
        let link = "https://app.example.com/onboarding/0123";
        let content = invitation_content(InvitationEmailKind::Created, link, ID);
        assert!(content.text.contains(link));
        assert!(content.html.contains(&format!("href=\"{link}\"")));
        // The in-app route stays as the fallback for uninstalled/unverified apps.
        assert!(content.html.contains("Invites"));
    }

    #[test]
    fn every_kind_produces_an_actionable_body() {
        for kind in [
            InvitationEmailKind::Created,
            InvitationEmailKind::Resent,
            InvitationEmailKind::KycRequired,
            InvitationEmailKind::Approved,
            InvitationEmailKind::Rejected,
        ] {
            let content = invitation_content(kind, "hamsavilas://onboarding/0123", ID);
            assert!(!content.subject.is_empty());
            assert!(content.text.contains("Invites"), "{kind:?}");
            assert!(content.html.contains("Invites"), "{kind:?}");
        }
    }

    #[test]
    fn scheme_detection_is_case_insensitive_and_trims() {
        assert!(is_web_link("HTTPS://app.example.com"));
        assert!(is_web_link("  https://app.example.com  "));
        assert!(is_web_link("http://localhost:8080/onboarding"));
        assert!(!is_web_link("hamsavilas://onboarding"));
        assert!(!is_web_link(""));
    }
}
