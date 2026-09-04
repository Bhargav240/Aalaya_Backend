//! Composition root. The one place that knows every concrete adapter and wires
//! them into the application. Boots even without a database (degraded mode).
use std::sync::Arc;

use pg_api::{build_router, AppState, Deps};
use pg_application::ports::clock::SystemClock;
use pg_application::ports::google::GoogleVerifier;
use pg_application::ports::storage::ObjectStore;
use pg_application::ports::tenancy::TenantResolver;
use pg_application::ports::verification::IdentityVerifier;
use pg_infrastructure::{
    auth::{HttpGoogleVerifier, JwtTokenService, NullGoogleVerifier, RandomOtpCodeGenerator},
    billing::SimpleBillingEngine,
    config::Config,
    notify::{
        FcmNotifier, LogEmailSender, LogInvitationEmailSender, LogNotifier, ResendEmailSender,
        ResendInvitationEmailSender,
    },
    observability, persistence,
    persistence::{
        PgArchiveRepository, PgAuditRepository, PgAwayRepository, PgBedRepository,
        PgBranchRepository, PgChangeRequestRepository, PgComplaintRepository,
        PgDeviceTokenRepository, PgEmailOtpRepository, PgEnquiryRepository, PgFinanceRepository,
        PgFloorRepository, PgInvitationVerificationRepository, PgListingRepository,
        PgNoticeRepository, PgOrganizationRepository, PgPaymentRepository,
        PgReceiptTemplateRepository, PgRefreshTokenRepository, PgResidentRepository,
        PgRoomRepository, PgSettlementRepository, PgTenantActivationRepository,
        PgTenantInvitationRepository, PgTenantProvisioner, PgTrustedDeviceRepository,
        PgUserRepository, PgVacancyRepository, PgVerificationRepository,
    },
    receipt::SimpleReceiptRenderer,
    storage::{LocalFsObjectStore, SupabaseStorage},
    tenancy::{CachedTenantResolver, DbTenantResolver},
    verify::{
        CompositeVerifier, DigiLockerClientConfig, DigiLockerVerifier, HttpDigiLockerClient,
        MockIdentityVerifier,
    },
};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env();
    observability::init(config.log_format);

    info!(bind = %config.bind_addr, "starting pg-server");
    if config.jwt_secret_is_default {
        warn!("JWT_SECRET not set -- using an insecure dev default. Set JWT_SECRET before production.");
    }
    if config.provisioning_key.is_none() {
        warn!("PROVISIONING_KEY not set -- the org-provisioning endpoint is disabled until it is.");
    }

    // Fail fast in profile=production unless the operator explicitly opts in
    // via --degraded / PG_ALLOW_DEGRADED. A silently-degraded deploy accepting
    // traffic with 503s is worse than a crash because load balancers keep it alive.
    if config.database_url.is_none() && std::env::var_os("PG_ALLOW_DEGRADED").is_none() {
        eprintln!("FATAL: DATABASE_URL is unset. Refusing to start the API in degraded mode.");
        eprintln!("To boot anyway (dev only): PG_ALLOW_DEGRADED=1 ./pg-server");
        std::process::exit(1);
    }

    let state = match &config.database_url {
        Some(url) => match persistence::connect_with_limits(
            url,
            config.db_min_connections,
            config.db_max_connections,
            config.db_idle_timeout_secs,
        )
        .await
        {
            Ok(pool) => {
                match run_migrations(config.migration_database_url.as_deref().unwrap_or(url)).await
                {
                    Ok(()) => {
                        let resolver: Arc<dyn TenantResolver> =
                            Arc::new(CachedTenantResolver::new(Arc::new(DbTenantResolver::new(
                                pool.clone(),
                            ))));
                        let store: Arc<dyn ObjectStore> = match config.storage_backend.as_str() {
                            "supabase" => match (
                                &config.supabase_url,
                                &config.supabase_service_key,
                                &config.supabase_bucket,
                            ) {
                                (Some(u), Some(k), Some(b)) => {
                                    info!("using Supabase Storage backend");
                                    Arc::new(SupabaseStorage::new(u.clone(), k.clone(), b.clone()))
                                }
                                _ => {
                                    warn!("STORAGE_BACKEND=supabase but SUPABASE_URL/SERVICE_KEY/BUCKET not all set; using local FS");
                                    Arc::new(LocalFsObjectStore::new(config.storage_dir.clone()))
                                }
                            },
                            _ => Arc::new(LocalFsObjectStore::new(config.storage_dir.clone())),
                        };
                        let deps = Deps {
                            onboarding_layout: config.onboarding_layout.clone(),
                            org_repo: Arc::new(PgOrganizationRepository::new(pool.clone())),
                            tenant_invitation_repo: Arc::new(PgTenantInvitationRepository::new(
                                pool.clone(),
                            )),
                            tenant_activation_repo: Arc::new(PgTenantActivationRepository::new(
                                pool.clone(),
                            )),
                            user_repo: Arc::new(PgUserRepository::new(pool.clone())),
                            branch_repo: Arc::new(PgBranchRepository::new(pool.clone())),
                            room_repo: Arc::new(PgRoomRepository::new(pool.clone())),
                            bed_repo: Arc::new(PgBedRepository::new(pool.clone())),
                            resident_repo: Arc::new(PgResidentRepository::new(pool.clone())),
                            away_repo: Arc::new(PgAwayRepository::new(pool.clone())),
                            settlement_repo: Arc::new(PgSettlementRepository::new(pool.clone())),
                            cr_repo: Arc::new(PgChangeRequestRepository::new(pool.clone())),
                            audit_repo: Arc::new(PgAuditRepository::new(pool.clone())),
                            payment_repo: Arc::new(PgPaymentRepository::new(pool.clone())),
                            store: store.clone(),
                            renderer: Arc::new(SimpleReceiptRenderer),
                            billing: Arc::new(SimpleBillingEngine),
                            receipt_url_ttl_secs: config.receipt_url_ttl_secs,
                            template_repo: Arc::new(PgReceiptTemplateRepository::new(pool.clone())),
                            complaint_repo: Arc::new(PgComplaintRepository::new(pool.clone())),
                            notice_repo: Arc::new(PgNoticeRepository::new(pool.clone())),
                            notifier: match &config.fcm_service_account_json {
                                Some(json) => match FcmNotifier::from_service_account_json(json) {
                                    Ok(fcm) => {
                                        info!("using FCM push notifier");
                                        Arc::new(fcm)
                                            as Arc<dyn pg_application::ports::notifier::Notifier>
                                    }
                                    Err(e) => {
                                        warn!(error = %e, "FCM key invalid; notices will only log");
                                        Arc::new(LogNotifier)
                                            as Arc<dyn pg_application::ports::notifier::Notifier>
                                    }
                                },
                                None => {
                                    warn!(
                                    "FCM_SERVICE_ACCOUNT_JSON not set; notices will only log (dev)"
                                );
                                    Arc::new(LogNotifier)
                                        as Arc<dyn pg_application::ports::notifier::Notifier>
                                }
                            },
                            verifier: {
                                let v: Arc<dyn IdentityVerifier> = Arc::new(
                                    CompositeVerifier::new(config.verification_method.clone())
                                        .register("mock", Arc::new(MockIdentityVerifier))
                                        .register("digilocker", Arc::new(DigiLockerVerifier)),
                                );
                                v
                            },
                            verification_repo: Arc::new(PgVerificationRepository::new(
                                pool.clone(),
                            )),
                            verification_method: config.verification_method.clone(),
                            google_verifier: match &config.google_client_id {
                                Some(id) if !id.is_empty() => {
                                    Arc::new(HttpGoogleVerifier::new(id.clone()))
                                        as Arc<dyn GoogleVerifier>
                                }
                                _ => {
                                    warn!("GOOGLE_CLIENT_ID missing; /auth/google disabled");
                                    Arc::new(NullGoogleVerifier) as Arc<dyn GoogleVerifier>
                                }
                            },
                            archive_repo: Arc::new(PgArchiveRepository::new(pool.clone())),
                            provisioner: Arc::new(PgTenantProvisioner::new(pool.clone())),
                            listing_repo: Arc::new(PgListingRepository::new(pool.clone())),
                            finance_repo: Arc::new(PgFinanceRepository::new(pool.clone())),
                            vacancy_repo: Arc::new(PgVacancyRepository::new(pool.clone())),
                            enquiry_repo: Arc::new(PgEnquiryRepository::new(pool.clone())),
                            device_token_repo: Arc::new(PgDeviceTokenRepository::new(pool.clone())),
                            trusted_device_repo: Arc::new(PgTrustedDeviceRepository::new(
                                pool.clone(),
                            )),
                            floor_repo: Arc::new(PgFloorRepository::new(pool.clone())),
                            tokens: Arc::new(JwtTokenService::new(
                                &config.jwt_secret,
                                config.access_ttl_secs,
                                config.refresh_ttl_secs,
                            )),
                            resolver,
                            clock: Arc::new(SystemClock),
                            provisioning_key: config.provisioning_key.clone(),
                            refresh_repo: Arc::new(PgRefreshTokenRepository::new(pool.clone())),
                            refresh_ttl_secs: config.refresh_ttl_secs,
                            otp_codegen: Arc::new(RandomOtpCodeGenerator),
                            otp_ttl_secs: config.otp_ttl_secs,
                            otp_resend_cooldown_secs: config.otp_resend_cooldown_secs,
                            otp_max_attempts: config.otp_max_attempts,
                            email_otp_repo: Arc::new(PgEmailOtpRepository::new(pool.clone())),
                            email_sender: match config.email_provider.as_str() {
                                "resend" => match (&config.resend_api_key, &config.email_from) {
                                    (Some(api_key), Some(from)) => Arc::new(ResendEmailSender::new(
                                        api_key.clone(),
                                        from.clone(),
                                        config.pg_name.clone(),
                                    ))
                                        as Arc<dyn pg_application::ports::EmailSender>,
                                    _ => {
                                        warn!("EMAIL_PROVIDER=resend but RESEND_API_KEY or EMAIL_FROM missing; email OTPs will only log");
                                        Arc::new(LogEmailSender::new(config.debug_email_otp))
                                            as Arc<dyn pg_application::ports::EmailSender>
                                    }
                                },
                                _ => Arc::new(LogEmailSender::new(config.debug_email_otp))
                                    as Arc<dyn pg_application::ports::EmailSender>,
                            },
                            invitation_email_sender: match config.email_provider.as_str() {
                                "resend" => match (&config.resend_api_key, &config.email_from) {
                                    (Some(api_key), Some(from)) => {
                                        Arc::new(ResendInvitationEmailSender::new(
                                            api_key.clone(),
                                            from.clone(),
                                            config.app_deep_link_base.clone(),
                                        ))
                                            as Arc<dyn pg_application::ports::InvitationEmailSender>
                                    }
                                    _ => Arc::new(LogInvitationEmailSender)
                                        as Arc<dyn pg_application::ports::InvitationEmailSender>,
                                },
                                _ => Arc::new(LogInvitationEmailSender)
                                    as Arc<dyn pg_application::ports::InvitationEmailSender>,
                            },
                            invitation_resend_cooldown_secs: config.invitation_resend_cooldown_secs,
                            digilocker_client: match (
                                &config.digilocker_client_id,
                                &config.digilocker_client_secret,
                                &config.digilocker_redirect_uri,
                            ) {
                                (Some(client_id), Some(client_secret), Some(redirect_uri)) => Some(
                                    Arc::new(HttpDigiLockerClient::new(DigiLockerClientConfig {
                                        client_id: client_id.clone(),
                                        client_secret: client_secret.clone(),
                                        redirect_uri: redirect_uri.clone(),
                                        authorization_endpoint: config
                                            .digilocker_authorization_endpoint
                                            .clone(),
                                        token_endpoint: config.digilocker_token_endpoint.clone(),
                                        userinfo_endpoint: config
                                            .digilocker_userinfo_endpoint
                                            .clone(),
                                        scopes: config.digilocker_scopes.clone(),
                                    }))
                                        as Arc<dyn pg_application::ports::DigiLockerClient>,
                                ),
                                _ => None,
                            },
                            invitation_verification_repo: Arc::new(
                                PgInvitationVerificationRepository::new(pool.clone()),
                            ),
                            digilocker_session_encryption_key: config
                                .digilocker_session_encryption_key
                                .clone(),
                            digilocker_session_ttl_secs: config.digilocker_session_ttl_secs,
                            app_deep_link_base: config.app_deep_link_base.clone(),
                            email_otp_max_sends: config.email_otp_max_sends,
                            email_otp_window_secs: config.email_otp_window_secs,
                            platform_org_id: config.platform_org_id,
                        };
                        info!("database connected and migrated");
                        AppState {
                            platform_logs: std::sync::Arc::new(std::sync::RwLock::new(
                                std::collections::VecDeque::new(),
                            )),
                            deps: Some(Arc::new(deps)),
                            revoked_jtis: Default::default(),
                            active_memberships_cache: Default::default(),
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "migrations failed; continuing in degraded mode");
                        AppState {
                            platform_logs: std::sync::Arc::new(std::sync::RwLock::new(
                                std::collections::VecDeque::new(),
                            )),
                            deps: None,
                            revoked_jtis: Default::default(),
                            active_memberships_cache: Default::default(),
                        }
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "could not connect to database; degraded mode");
                AppState {
                    platform_logs: std::sync::Arc::new(std::sync::RwLock::new(
                        std::collections::VecDeque::new(),
                    )),
                    deps: None,
                    revoked_jtis: Default::default(),
                    active_memberships_cache: Default::default(),
                }
            }
        },
        None => {
            warn!("DATABASE_URL not set; starting in degraded mode (no DB)");
            AppState {
                platform_logs: std::sync::Arc::new(std::sync::RwLock::new(
                    std::collections::VecDeque::new(),
                )),
                deps: None,
                revoked_jtis: Default::default(),
                active_memberships_cache: Default::default(),
            }
        }
    };

    // Background jobs (vacancy projector + archive scheduler) get their own
    // narrow pool so a stuck sweep can never starve HTTP. Both are best-effort:
    // their failure must never crash the API; Ctrl-C cancels them.
    let mut worker_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    if let (Some(deps), Some(db_url)) = (&state.deps, &config.database_url) {
        match pg_infrastructure::persistence::pool::connect_worker(db_url).await {
            Ok(worker_pool) => {
                {
                    let deps = deps.clone();
                    let _pool = worker_pool.clone();
                    let interval = config.vacancy_interval_secs.max(10);
                    worker_handles.push(tokio::spawn(async move {
                        // B22: random start-offset desynchronizes sweeps across
                        // replicas (avoids every instance waking up on the same tick).
                        let offset = {
                            use std::collections::hash_map::RandomState;
                            use std::hash::{BuildHasher, Hasher};
                            let mut h = RandomState::new().build_hasher();
                            h.write_u64(interval as u64);
                            h.finish() % (interval as u64).max(2)
                        };
                        tokio::time::sleep(std::time::Duration::from_secs(offset)).await;
                        let mut tick =
                            tokio::time::interval(std::time::Duration::from_secs(interval as u64));
                        loop {
                            tick.tick().await;
                            if let Err(e) = pg_application::services::refresh_all(
                                deps.vacancy_repo.as_ref(),
                                deps.resolver.as_ref(),
                            )
                            .await
                            {
                                warn!(error = %e, "vacancy sweep failed");
                            }
                        }
                    }));
                }
                {
                    let deps = deps.clone();
                    let interval = config.archive_interval_secs.max(60);
                    let older_than = config.archive_older_than_days;
                    worker_handles.push(tokio::spawn(async move {
                        let mut tick = tokio::time::interval(std::time::Duration::from_secs(
                            interval as u64,
                        ));
                        loop {
                            tick.tick().await;
                            for org_row in match deps.org_repo.list_all_live().await {
                                Ok(v) => v,
                                Err(e) => {
                                    warn!(error = %e, "archive: list orgs failed");
                                    continue;
                                }
                            } {
                                let ctx = match deps.resolver.resolve(org_row).await {
                                    Ok(c) => c,
                                    Err(_) => continue,
                                };
                                if let Err(e) = pg_application::services::archive_deleted(
                                    pg_application::services::ArchiveDeletedCommand {
                                        archive_repo: deps.archive_repo.as_ref(),
                                        store: deps.store.as_ref(),
                                        clock: deps.clock.as_ref(),
                                        ctx: &ctx,
                                        role: pg_domain::Role::Owner,
                                        actor: &pg_application::ports::repositories::AuditActor {
                                        user_id: None,
                                        request_id: None,
                                        },
                                        older_than_days: older_than,
                                    },
                                )
                                .await
                                {
                                    warn!(org = %org_row.as_uuid(), error = %e, "archive sweep failed");
                                }
                            }
                        }
                    }));
                }
                {
                    // B18: sweep append-only control-plane tables (email OTPs, dead
                    // refresh tokens) so they never grow unbounded.
                    let pool = worker_pool.clone();
                    worker_handles.push(tokio::spawn(async move {
                        let mut tick =
                            tokio::time::interval(std::time::Duration::from_secs(86_400));
                        loop {
                            tick.tick().await;
                            let otps = pg_infrastructure::persistence::PgEmailOtpRepository::new(
                                pool.clone(),
                            );
                            match otps.delete_expired().await {
                                Ok(n) if n > 0 => info!(expired_otps = n, "cleanup: email_otps"),
                                Err(e) => warn!(error = %e, "cleanup email_otps failed"),
                                _ => {}
                            }
                            let refresh =
                                pg_infrastructure::persistence::PgRefreshTokenRepository::new(
                                    pool.clone(),
                                );
                            match refresh.delete_expired().await {
                                Ok(n) if n > 0 => {
                                    info!(expired_tokens = n, "cleanup: refresh_tokens")
                                }
                                Err(e) => warn!(error = %e, "cleanup refresh_tokens failed"),
                                _ => {}
                            }
                        }
                    }));
                }
            }
            Err(e) => warn!(error = %e, "worker pool failed to init; background jobs disabled"),
        }
    }

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    info!(addr = %config.bind_addr, "listening");
    // `ConnectInfo<SocketAddr>` is how the per-IP rate limiter on the public
    // marketplace routes learns the caller's address.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    info!("shutting down; aborting background workers");
    for h in worker_handles {
        h.abort();
    }
    Ok(())
}

async fn run_migrations(database_url: &str) -> anyhow::Result<()> {
    let pool = persistence::connect_migrations(database_url).await?;
    let result = persistence::migrations::run(&pool).await;
    pool.close().await;
    result
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
