use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The company running the marketplace — above every PG org.
    PlatformOperator,
    Owner,
    Manager,
    Tenant,
    /// A light identity: enquired on the marketplace, not moved in yet.
    Prospect,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::PlatformOperator => "platform_operator",
            Role::Owner => "owner",
            Role::Manager => "manager",
            Role::Tenant => "tenant",
            Role::Prospect => "prospect",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "platform_operator" => Some(Role::PlatformOperator),
            "owner" => Some(Role::Owner),
            "manager" => Some(Role::Manager),
            "tenant" => Some(Role::Tenant),
            "prospect" => Some(Role::Prospect),
            _ => None,
        }
    }
    /// Owner/manager run tenant operations; operator/prospect do not.
    pub fn is_tenant_staff(&self) -> bool {
        matches!(self, Role::Owner | Role::Manager)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Suspended,
}

impl UserStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserStatus::Active => "active",
            UserStatus::Suspended => "suspended",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(UserStatus::Active),
            "suspended" => Some(UserStatus::Suspended),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanKind {
    Monthly,
    Weekly,
    Daily,
}

impl PlanKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanKind::Monthly => "monthly",
            PlanKind::Weekly => "weekly",
            PlanKind::Daily => "daily",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "monthly" => Some(PlanKind::Monthly),
            "weekly" => Some(PlanKind::Weekly),
            "daily" => Some(PlanKind::Daily),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplaintState {
    Open,
    Assigned,
    InProgress,
    Resolved,
    Closed,
}

impl ComplaintState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ComplaintState::Open => "open",
            ComplaintState::Assigned => "assigned",
            ComplaintState::InProgress => "in_progress",
            ComplaintState::Resolved => "resolved",
            ComplaintState::Closed => "closed",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "open" => Some(ComplaintState::Open),
            "assigned" => Some(ComplaintState::Assigned),
            "in_progress" => Some(ComplaintState::InProgress),
            "resolved" => Some(ComplaintState::Resolved),
            "closed" => Some(ComplaintState::Closed),
            _ => None,
        }
    }
    /// Allowed transitions, including the Resolved -> InProgress bounce-back.
    pub fn can_transition_to(self, to: ComplaintState) -> bool {
        use ComplaintState::*;
        matches!(
            (self, to),
            (Open, Assigned)
                | (Open, InProgress)
                | (Assigned, InProgress)
                | (InProgress, Resolved)
                | (Resolved, Closed)
                | (Resolved, InProgress)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeRequestStatus {
    Pending,
    Approved,
    Rejected,
}

/// Lifecycle of a tenant invitation and its onboarding process.
///
/// The enum is the single source of truth for persisted status values and
/// legal state changes. Transport and persistence layers must not invent
/// their own free-form status strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatus {
    Invited,
    EmailVerified,
    KycPending,
    KycVerified,
    KycFailed,
    ApprovalPending,
    Approved,
    Rejected,
    Cancelled,
    Expired,
    Active,
}

impl InvitationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Invited => "invited",
            Self::EmailVerified => "email_verified",
            Self::KycPending => "kyc_pending",
            Self::KycVerified => "kyc_verified",
            Self::KycFailed => "kyc_failed",
            Self::ApprovalPending => "approval_pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
            Self::Active => "active",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "invited" => Some(Self::Invited),
            "email_verified" => Some(Self::EmailVerified),
            "kyc_pending" => Some(Self::KycPending),
            "kyc_verified" => Some(Self::KycVerified),
            "kyc_failed" => Some(Self::KycFailed),
            "approval_pending" => Some(Self::ApprovalPending),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            "cancelled" => Some(Self::Cancelled),
            "expired" => Some(Self::Expired),
            "active" => Some(Self::Active),
            _ => None,
        }
    }

    pub fn can_transition_to(self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Invited, Self::EmailVerified)
                | (Self::Invited, Self::Cancelled)
                | (Self::Invited, Self::Expired)
                | (Self::EmailVerified, Self::KycPending)
                | (Self::EmailVerified, Self::ApprovalPending)
                | (Self::EmailVerified, Self::Rejected)
                | (Self::EmailVerified, Self::Cancelled)
                | (Self::KycPending, Self::KycVerified)
                | (Self::KycPending, Self::KycFailed)
                | (Self::KycVerified, Self::ApprovalPending)
                | (Self::ApprovalPending, Self::Approved)
                | (Self::ApprovalPending, Self::Rejected)
                | (Self::Approved, Self::Active)
        )
    }
}

#[cfg(test)]
mod invitation_status_tests {
    use super::InvitationStatus;

    #[test]
    fn persisted_values_round_trip() {
        let statuses = [
            InvitationStatus::Invited,
            InvitationStatus::EmailVerified,
            InvitationStatus::KycPending,
            InvitationStatus::KycVerified,
            InvitationStatus::KycFailed,
            InvitationStatus::ApprovalPending,
            InvitationStatus::Approved,
            InvitationStatus::Rejected,
            InvitationStatus::Cancelled,
            InvitationStatus::Expired,
            InvitationStatus::Active,
        ];

        for status in statuses {
            assert_eq!(InvitationStatus::parse(status.as_str()), Some(status));
        }
    }

    #[test]
    fn terminal_states_cannot_transition() {
        for status in [
            InvitationStatus::KycFailed,
            InvitationStatus::Rejected,
            InvitationStatus::Cancelled,
            InvitationStatus::Expired,
            InvitationStatus::Active,
        ] {
            assert!(!status.can_transition_to(InvitationStatus::Active));
        }
    }
}
