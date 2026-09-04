use crate::enums::UserStatus;
use crate::ids::{OrgId, UserId};

#[derive(Debug, Clone)]
pub struct Organization {
    pub id: OrgId,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub org_id: OrgId,
    pub email: Option<String>,
    pub status: UserStatus,
}
