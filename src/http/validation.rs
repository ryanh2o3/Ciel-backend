use crate::http::AppError;

pub const MAX_HANDLE_LEN: usize = 30;
pub const MIN_HANDLE_LEN: usize = 3;
pub const MAX_DISPLAY_NAME_LEN: usize = 50;
pub const MAX_BIO_LEN: usize = 500;
pub const MAX_CAPTION_LEN: usize = 2200;
pub const MAX_PASSWORD_LEN: usize = 128;

pub fn required_trimmed(field: &'static str, value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request(format!("{field} is required")));
    }
    Ok(trimmed.to_string())
}

pub fn validate_max_len(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), AppError> {
    if value.len() > max_len {
        return Err(AppError::bad_request(format!(
            "{field} must be at most {max_len} characters"
        )));
    }
    Ok(())
}

pub fn validate_handle(handle: &str) -> Result<(), AppError> {
    let handle = handle.trim();
    if handle.len() < MIN_HANDLE_LEN {
        return Err(AppError::bad_request("handle must be at least 3 characters"));
    }
    if handle.len() > MAX_HANDLE_LEN {
        return Err(AppError::bad_request("handle must be at most 30 characters"));
    }
    if !handle.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::bad_request(
            "handle can only contain letters, numbers, and underscores",
        ));
    }
    Ok(())
}

