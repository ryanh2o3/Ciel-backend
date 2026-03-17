use crate::http::AppError;

pub fn internal_err(context: &'static str, err: anyhow::Error) -> AppError {
    tracing::error!(context, error = ?err, "request failed");
    AppError::internal("internal server error")
}

pub fn internal_err_user(
    context: &'static str,
    user_id: impl std::fmt::Display,
    err: anyhow::Error,
) -> AppError {
    tracing::error!(context, user_id = %user_id, error = ?err, "request failed");
    AppError::internal("internal server error")
}

