use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use pg_application::ports::auth::{Claims, TokenKind, TokenPair, TokenService};
use pg_application::AppError;
use pg_domain::{OrgId, Role, UserId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct Jwt {
    sub: String,
    org: String,
    role: String,
    kind: String,
    exp: i64,
    /// Issued-at, and a random token id.
    ///
    /// Without these the payload varies only by `exp`, which has one-second
    /// resolution — so two logins by the same user inside the same second
    /// produced *byte-identical* tokens. That collided on the refresh-token
    /// primary key (surfacing as a nonsensical 409 on login) and, worse, meant
    /// two independent sessions shared one refresh token, so rotating either
    /// silently revoked the other. `jti` makes every issued token unique and
    /// unpredictable regardless of timing.
    iat: i64,
    jti: String,
}

/// HS256 JWT adapter issuing short-lived access and longer refresh tokens.
pub struct JwtTokenService {
    secret: Vec<u8>,
    access_ttl: i64,
    refresh_ttl: i64,
}

impl JwtTokenService {
    pub fn new(secret: &str, access_ttl: i64, refresh_ttl: i64) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            access_ttl,
            refresh_ttl,
        }
    }

    fn issue(
        &self,
        user_id: UserId,
        org_id: OrgId,
        role: Role,
        kind: &str,
        ttl: i64,
    ) -> Result<String, AppError> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let claims = Jwt {
            sub: user_id.to_string(),
            org: org_id.to_string(),
            role: role.as_str().to_string(),
            kind: kind.to_string(),
            exp: now + ttl,
            iat: now,
            jti: Uuid::new_v4().to_string(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|e| AppError::Internal(format!("token encode: {e}")))
    }
}

impl TokenService for JwtTokenService {
    fn issue_pair(
        &self,
        user_id: UserId,
        org_id: OrgId,
        role: Role,
    ) -> Result<TokenPair, AppError> {
        Ok(TokenPair {
            access_token: self.issue(user_id, org_id, role, "access", self.access_ttl)?,
            refresh_token: self.issue(user_id, org_id, role, "refresh", self.refresh_ttl)?,
        })
    }

    fn issue_access(&self, user_id: UserId, org_id: OrgId, role: Role) -> Result<String, AppError> {
        self.issue(user_id, org_id, role, "access", self.access_ttl)
    }

    fn verify(&self, token: &str) -> Result<Claims, AppError> {
        let data = decode::<Jwt>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|_| AppError::Unauthorized)?;
        let c = data.claims;
        let user_id = UserId::new(Uuid::parse_str(&c.sub).map_err(|_| AppError::Unauthorized)?);
        let org_id = OrgId::new(Uuid::parse_str(&c.org).map_err(|_| AppError::Unauthorized)?);
        let role = Role::parse(&c.role).ok_or(AppError::Unauthorized)?;
        let kind = match c.kind.as_str() {
            "access" => TokenKind::Access,
            "refresh" => TokenKind::Refresh,
            _ => return Err(AppError::Unauthorized),
        };
        Ok(Claims {
            user_id,
            org_id,
            role,
            kind,
            jti: c.jti,
            exp: c.exp,
        })
    }
}
