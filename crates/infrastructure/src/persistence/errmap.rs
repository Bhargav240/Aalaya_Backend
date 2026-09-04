use pg_application::AppError;

/// Translate sqlx errors into the application's vocabulary. Unique violations
/// carry the constraint name (where available) so the API can route the right
/// user-facing conflict title; FK violations mean the caller referenced an
/// entity that doesn't exist (mapped to NotFound); RLS denials to Forbidden.
pub fn map_sqlx(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            let constraint = db.constraint().unwrap_or("duplicate");
            return AppError::Conflict(constraint.to_string());
        }
        if db.is_foreign_key_violation() {
            return AppError::NotFound;
        }
        // Insufficient privilege (RLS denial) — should never reach the app, but
        // surface as Forbidden rather than a bare 500 so misconfigurations are visible.
        if db.code().map(|c| c == "42501").unwrap_or(false) {
            return AppError::Forbidden;
        }
    }
    AppError::Persistence(e.to_string())
}
