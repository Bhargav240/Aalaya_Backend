use crate::error::ApiError;
use crate::extractors::AuthUser;
use crate::state::AppState;
use crate::support::{require_resident_management_scope, resolve_ctx};
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::Router;
use pg_application::services;
use pg_application::services::ReceiptDownload;
use pg_domain::ReceiptId;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/receipts/{id}/download", get(download))
}

/// Redirects to a short-lived signed URL (Supabase/S3) when available, else
/// streams the PDF through the backend. Always after the authz + ready check.
async fn download(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(receipt_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let deps = state.deps()?;
    let ctx = resolve_ctx(deps, claims.org_id).await?;
    // Branch-scope managers: the receipt's resident must be in their branch.
    let resident = services::receipt_resident(
        deps.payment_repo.as_ref(),
        &ctx,
        claims.role,
        ReceiptId::new(receipt_id),
    )
    .await?;
    require_resident_management_scope(deps, &ctx, &claims, resident).await?;
    let target = services::resolve_receipt_download(
        deps.payment_repo.as_ref(),
        deps.store.as_ref(),
        &ctx,
        claims.role,
        ReceiptId::new(receipt_id),
        deps.receipt_url_ttl_secs,
    )
    .await?;
    Ok(match target {
        ReceiptDownload::Redirect(url) => Redirect::temporary(&url).into_response(),
        ReceiptDownload::Inline(bytes) => {
            ([(header::CONTENT_TYPE, "application/pdf")], bytes).into_response()
        }
    })
}
