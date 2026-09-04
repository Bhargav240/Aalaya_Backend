//! Deterministic `TokenService`: issues opaque tokens keyed by their inner claims.
use pg_application::error::AppError;
use pg_application::ports::{Claims, TokenPair, TokenService};
use pg_domain::{OrgId, Role, UserId};
use std::collections::HashMap;
use std::sync::Mutex;

/// Token strings are their own lookup keys: `access:<n>` / `refresh:<n>`, so a
/// test can mint a pair, pass the refresh string back through `verify`, and get
/// the same claims out. `verify` honors `kind` exactly like the JWT adapter.
pub struct FakeTokenService {
    counter: Mutex<u64>,
    claims: Mutex<HashMap<String, Claims>>,
}

impl FakeTokenService {
    pub fn new() -> Self {
        Self {
            counter: Mutex::new(0),
            claims: Mutex::new(HashMap::new()),
        }
    }

    fn mint(&self, user_id: UserId, org_id: OrgId, role: Role, kind_str: &str) -> String {
        let mut n = self.counter.lock().expect("poisoned");
        *n += 1;
        let token = format!("{}:{}", kind_str, *n);
        let kind = if kind_str == "access" {
            pg_application::ports::TokenKind::Access
        } else {
            pg_application::ports::TokenKind::Refresh
        };
        self.claims.lock().expect("poisoned").insert(
            token.clone(),
            Claims {
                user_id,
                org_id,
                role,
                kind,
                jti: format!("jti-{}", *n),
                exp: i64::MAX,
            },
        );
        token
    }
}

impl Default for FakeTokenService {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenService for FakeTokenService {
    fn issue_pair(
        &self,
        user_id: UserId,
        org_id: OrgId,
        role: Role,
    ) -> Result<TokenPair, AppError> {
        Ok(TokenPair {
            access_token: self.mint(user_id, org_id, role, "access"),
            refresh_token: self.mint(user_id, org_id, role, "refresh"),
        })
    }

    fn issue_access(&self, user_id: UserId, org_id: OrgId, role: Role) -> Result<String, AppError> {
        Ok(self.mint(user_id, org_id, role, "access"))
    }

    fn verify(&self, token: &str) -> Result<Claims, AppError> {
        self.claims
            .lock()
            .expect("poisoned")
            .get(token)
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}
