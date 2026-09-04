use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub onboarding_layout: Option<pg_application::services::OnboardingLayout>,
    /// None => degraded mode (health works, DB routes 503).
    pub database_url: Option<String>,
    /// Direct or session-pooler URL for startup migrations. Falls back to
    /// DATABASE_URL for existing deployments, but Supabase should set this
    /// separately from its transaction-pooler application URL.
    pub migration_database_url: Option<String>,
    pub bind_addr: String,
    pub log_format: LogFormat,
    pub jwt_secret: String,
    /// "production" | "development". Anything else is treated as development.
    /// Production profile FAILS FAST at startup for dangerous defaults (B15).
    pub profile: String,
    /// Guards the org-provisioning endpoint (no auth exists before the first owner).
    pub provisioning_key: Option<String>,
    pub access_ttl_secs: i64,
    pub refresh_ttl_secs: i64,
    pub platform_org_id: pg_domain::OrgId,
    pub storage_dir: String,
    pub verification_method: String,
    pub storage_backend: String,
    pub supabase_url: Option<String>,
    pub supabase_service_key: Option<String>,
    pub supabase_bucket: Option<String>,
    pub receipt_url_ttl_secs: i64,
    /// True when jwt_secret fell back to the built-in dev default.
    pub jwt_secret_is_default: bool,
    pub otp_ttl_secs: i64,
    pub otp_resend_cooldown_secs: i64,
    pub otp_max_attempts: i32,
    pub email_provider: String,
    pub resend_api_key: Option<String>,
    pub email_from: Option<String>,
    pub pg_name: String,
    pub app_deep_link_base: String,
    pub invitation_resend_cooldown_secs: i64,
    pub digilocker_client_id: Option<String>,
    pub digilocker_client_secret: Option<String>,
    pub digilocker_redirect_uri: Option<String>,
    pub digilocker_authorization_endpoint: String,
    pub digilocker_token_endpoint: String,
    pub digilocker_userinfo_endpoint: String,
    pub digilocker_scopes: String,
    pub digilocker_session_encryption_key: Option<String>,
    pub digilocker_session_ttl_secs: i64,
    pub email_otp_max_sends: i64,
    pub email_otp_window_secs: i64,
    /// Google OAuth 2.0 client ID accepted as `aud` on identity tokens
    /// (Android + any web client). None disables the Google route in degraded
    /// mode (401).
    pub google_client_id: Option<String>,
    /// Local OTP codes are visible only when this flag is true in a debug build.
    pub debug_email_otp: bool,
    /// Firebase service-account JSON for FCM push. None => LogNotifier (dev).
    pub fcm_service_account_json: Option<String>,
    /// Vacancy projector sweep interval, seconds.
    pub vacancy_interval_secs: i64,
    /// Archive scheduler interval, seconds (default 24h).
    pub archive_interval_secs: i64,
    /// Soft-deleted rows older than this many days get archived.
    pub archive_older_than_days: i64,
    /// Minimum warmed DB connections maintained in the pool (default 5).
    pub db_min_connections: u32,
    /// Maximum DB connections in the pool (default 20).
    pub db_max_connections: u32,
    /// Idle connection timeout in seconds before retiring (default 120s).
    pub db_idle_timeout_secs: u64,
}

const DEV_SECRET: &str = "dev-insecure-secret-change-me";

/// Production startup refuses to boot with this config. Loud and immediate —
/// better than a warn-log people scroll past.
fn fatal_config(var: &str, reason: &str) -> ! {
    eprintln!("FATAL: unsafe production configuration");
    eprintln!("  {var}: {reason}");
    eprintln!("Fix the env and restart; do not deploy this to production.");
    std::process::exit(1);
}

impl Config {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        let database_url = env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
        let migration_database_url = env::var("MIGRATION_DATABASE_URL")
            .ok()
            .filter(|s| !s.is_empty());
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let log_format = match env::var("LOG_FORMAT").as_deref() {
            Ok("json") => LogFormat::Json,
            _ => LogFormat::Pretty,
        };
        let (jwt_secret, jwt_secret_is_default) = match env::var("JWT_SECRET") {
            Ok(s) if !s.is_empty() => (s, false),
            _ => (DEV_SECRET.to_string(), true),
        };
        let provisioning_key = env::var("PROVISIONING_KEY").ok().filter(|s| !s.is_empty());
        let profile = env::var("PROFILE").unwrap_or_else(|_| "development".into());
        let database_url_final = database_url.clone();
        let is_production = profile.eq_ignore_ascii_case("production");

        // B15 fail-fast guards: refuse to start in production with unsafe config.
        if is_production {
            if jwt_secret_is_default {
                fatal_config(
                    "JWT_SECRET",
                    "must be set in production (dev default leaks)",
                );
            }
            if database_url_final.is_none() {
                fatal_config("DATABASE_URL", "must be set in production");
            }
        }
        let verification_method =
            env::var("VERIFICATION_METHOD").unwrap_or_else(|_| "mock".to_string());
        if is_production && verification_method == "mock" {
            fatal_config(
                "VERIFICATION_METHOD",
                "must not be 'mock' in production — every identity would auto-verify",
            );
        }
        let email_provider = env::var("EMAIL_PROVIDER").unwrap_or_else(|_| "log".to_string());
        let resend_api_key = env::var("RESEND_API_KEY").ok().filter(|s| !s.is_empty());
        let email_from = env::var("EMAIL_FROM").ok().filter(|s| !s.is_empty());
        if is_production && email_provider != "resend" {
            fatal_config(
                "EMAIL_PROVIDER",
                "must be 'resend' in production so login and invitation emails are deliverable",
            );
        }
        if is_production && (resend_api_key.is_none() || email_from.is_none()) {
            fatal_config(
                "RESEND_API_KEY/EMAIL_FROM",
                "both values are required for production email delivery",
            );
        }
        let digilocker_client_id = optional_env("DIGILOCKER_CLIENT_ID");
        let digilocker_client_secret = optional_env("DIGILOCKER_CLIENT_SECRET");
        let digilocker_redirect_uri = optional_env("DIGILOCKER_REDIRECT_URI");
        let digilocker_session_encryption_key = optional_env("DIGILOCKER_SESSION_ENCRYPTION_KEY");
        if is_production
            && (digilocker_client_id.is_none()
                || digilocker_client_secret.is_none()
                || digilocker_redirect_uri.is_none()
                || digilocker_session_encryption_key.is_none())
        {
            fatal_config(
                "DIGILOCKER_*",
                "client credentials, redirect URI, and session encryption key are required",
            );
        }

        Self {
            onboarding_layout: env::var("ONBOARDING_LAYOUT_PATH")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|path| {
                    let yaml = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                        panic!("failed to read ONBOARDING_LAYOUT_PATH {path}: {e}")
                    });
                    let layout: pg_application::services::OnboardingLayout =
                        serde_yaml::from_str(&yaml).unwrap_or_else(|e| {
                            panic!("invalid onboarding layout YAML {path}: {e}")
                        });
                    layout
                        .validate()
                        .unwrap_or_else(|e| panic!("invalid onboarding layout {path}: {e}"));
                    layout
                }),
            database_url,
            migration_database_url,
            bind_addr,
            log_format,
            jwt_secret,
            profile,
            provisioning_key,
            access_ttl_secs: env_i64("ACCESS_TTL_SECS", 900),
            refresh_ttl_secs: env_i64("REFRESH_TTL_SECS", 1_209_600),
            platform_org_id: pg_domain::OrgId::new(
                uuid::Uuid::parse_str(
                    &std::env::var("PLATFORM_ORG_ID")
                        .unwrap_or_else(|_| "00000000-0000-0000-0000-000000000001".into()),
                )
                .unwrap_or_else(|_| uuid::Uuid::nil()),
            ),
            storage_dir: env::var("STORAGE_DIR")
                .unwrap_or_else(|_| "./data/objectstore".to_string()),
            verification_method,
            storage_backend: env::var("STORAGE_BACKEND").unwrap_or_else(|_| "local".to_string()),
            supabase_url: env::var("SUPABASE_URL").ok().filter(|s| !s.is_empty()),
            supabase_service_key: env::var("SUPABASE_SERVICE_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
            supabase_bucket: env::var("SUPABASE_BUCKET").ok().filter(|s| !s.is_empty()),
            receipt_url_ttl_secs: env_i64("RECEIPT_URL_TTL_SECS", 300),
            jwt_secret_is_default,
            otp_ttl_secs: env_i64("OTP_TTL_SECS", 600),
            otp_resend_cooldown_secs: env_i64("OTP_RESEND_COOLDOWN_SECS", 10),
            otp_max_attempts: env_i64("OTP_MAX_ATTEMPTS", 5) as i32,
            email_provider,
            resend_api_key,
            email_from,
            pg_name: env::var("PG_NAME").unwrap_or_else(|_| "Hamsa Vilas".to_string()),
            app_deep_link_base: env::var("APP_DEEP_LINK_BASE")
                .unwrap_or_else(|_| "hamsavilas://onboarding".to_string()),
            invitation_resend_cooldown_secs: env_i64("INVITATION_RESEND_COOLDOWN_SECS", 60),
            digilocker_client_id,
            digilocker_client_secret,
            digilocker_redirect_uri,
            digilocker_authorization_endpoint: env::var("DIGILOCKER_AUTHORIZATION_ENDPOINT")
                .unwrap_or_else(|_| {
                    "https://digilocker.meripehchaan.gov.in/public/oauth2/1/authorize".into()
                }),
            digilocker_token_endpoint: env::var("DIGILOCKER_TOKEN_ENDPOINT").unwrap_or_else(|_| {
                "https://digilocker.meripehchaan.gov.in/public/oauth2/1/token".into()
            }),
            digilocker_userinfo_endpoint: env::var("DIGILOCKER_USERINFO_ENDPOINT").unwrap_or_else(
                |_| "https://digilocker.meripehchaan.gov.in/public/oauth2/1/user".into(),
            ),
            digilocker_scopes: env::var("DIGILOCKER_SCOPES").unwrap_or_else(|_| "openid".into()),
            digilocker_session_encryption_key,
            digilocker_session_ttl_secs: env_i64("DIGILOCKER_SESSION_TTL_SECS", 600),
            email_otp_max_sends: env_i64("EMAIL_OTP_MAX_SENDS", 10),
            email_otp_window_secs: env_i64("EMAIL_OTP_WINDOW_SECS", 900),
            google_client_id: env::var("GOOGLE_CLIENT_ID").ok().filter(|s| !s.is_empty()),
            debug_email_otp: cfg!(debug_assertions)
                && env::var("DEBUG_EMAIL_OTP").is_ok_and(|value| value == "true"),
            fcm_service_account_json: env::var("FCM_SERVICE_ACCOUNT_JSON")
                .ok()
                .filter(|s| !s.is_empty()),
            vacancy_interval_secs: env_i64("VACANCY_INTERVAL_SECS", 120),
            archive_interval_secs: env_i64("ARCHIVE_INTERVAL_SECS", 86_400),
            archive_older_than_days: env_i64("ARCHIVE_OLDER_THAN_DAYS", 30),
            db_min_connections: env::var("DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            db_max_connections: env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            db_idle_timeout_secs: env::var("DB_IDLE_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(120),
        }
    }
}

fn env_i64(key: &str, default: i64) -> i64 {
    env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.is_empty())
}
