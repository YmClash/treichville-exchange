use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::auth::{
        AuthService, RegisterDto, LoginDto, RefreshTokenDto,
        ChangePasswordDto, UpdateProfileDto, Enable2FADto, Verify2FADto,
        middleware::CurrentUser,
    },
    domain::user::{User, UserRole},
    shared::{
        errors::{AppError, AppResult},
        utils::PaginationParams,
        state::AppState,
    },
};

// POST /api/v1/auth/register
pub async fn register(
    State(state): State<std::sync::Arc<AppState>>,
    Json(dto): Json<RegisterDto>,
) -> AppResult<impl IntoResponse> {
    let response = state.auth_service.register(dto).await?;
    
    Ok((StatusCode::CREATED, Json(response)))
}

// POST /api/v1/auth/login
pub async fn login(
    State(state): State<std::sync::Arc<AppState>>,
    Json(dto): Json<LoginDto>,
) -> AppResult<impl IntoResponse> {
    let response = state.auth_service.login(dto).await?;
    
    Ok((StatusCode::OK, Json(response)))
}

// POST /api/v1/auth/refresh
pub async fn refresh_token(
    State(state): State<std::sync::Arc<AppState>>,
    Json(dto): Json<RefreshTokenDto>,
) -> AppResult<impl IntoResponse> {
    let response = state.auth_service.refresh_token(&dto.refresh_token).await?;
    
    Ok((StatusCode::OK, Json(response)))
}

// POST /api/v1/auth/logout
pub async fn logout(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    state.auth_service.logout(user.id, "").await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Logged out successfully"
    }))))
}

// GET /api/v1/auth/me
pub async fn get_current_user(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let user_details = state.auth_service.get_user_details(user.id).await?;
    
    Ok((StatusCode::OK, Json(user_details)))
}

// PUT /api/v1/auth/update-profile
pub async fn update_profile(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(dto): Json<UpdateProfileDto>,
) -> AppResult<impl IntoResponse> {
    let updated_user = state.auth_service.update_profile(user.id, dto).await?;
    
    Ok((StatusCode::OK, Json(updated_user)))
}

// POST /api/v1/auth/change-password
pub async fn change_password(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(dto): Json<ChangePasswordDto>,
) -> AppResult<impl IntoResponse> {
    state.auth_service.change_password(user.id, dto).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Password changed successfully"
    }))))
}

// GET /api/v1/auth/verify-email/:token
pub async fn verify_email(
    Path(token): Path<String>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    state.auth_service.verify_email(&token).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Email verified successfully"
    }))))
}

// POST /api/v1/auth/forgot-password
pub async fn forgot_password(
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<ForgotPasswordRequest>,
) -> AppResult<impl IntoResponse> {
    state.auth_service.initiate_password_reset(&request.email).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Password reset instructions sent to your email"
    }))))
}

// POST /api/v1/auth/reset-password
pub async fn reset_password(
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<ResetPasswordRequest>,
) -> AppResult<impl IntoResponse> {
    state.auth_service.reset_password(request.token, request.new_password).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Password reset successfully"
    }))))
}

// POST /api/v1/auth/enable-2fa
pub async fn enable_2fa(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(dto): Json<Enable2FADto>,
) -> AppResult<impl IntoResponse> {
    let response = state.auth_service.enable_2fa(user.id, dto).await?;
    
    Ok((StatusCode::OK, Json(response)))
}

// POST /api/v1/auth/verify-2fa
pub async fn verify_2fa(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(dto): Json<Verify2FADto>,
) -> AppResult<impl IntoResponse> {
    state.auth_service.verify_2fa(user.id, dto).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "2FA verified successfully"
    }))))
}

// POST /api/v1/auth/disable-2fa
pub async fn disable_2fa(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<Disable2FARequest>,
) -> AppResult<impl IntoResponse> {
    state.auth_service.disable_2fa(user.id, request.password).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "2FA disabled successfully"
    }))))
}

// Admin endpoints
// GET /api/v1/admin/users
pub async fn admin_get_users(
    Query(params): Query<GetUsersQuery>,
    Extension(admin): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    if !admin.is_admin() {
        return Err(AppError::forbidden("Admin access required"));
    }

    let users = state.auth_service
        .get_users(
            Some(params.page.unwrap_or(1) as u32),
            Some(params.per_page.unwrap_or(20) as u32),
            params.role.clone(),
            params.status.as_deref().map(|s| s == "active"),
        )
        .await?;
    
    Ok((StatusCode::OK, Json(users)))
}

// POST /api/v1/admin/users/:id/activate
pub async fn admin_activate_user(
    Path(user_id): Path<Uuid>,
    Extension(admin): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    if !admin.is_admin() {
        return Err(AppError::forbidden("Admin access required"));
    }

    state.auth_service.activate_user(user_id).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "User activated successfully"
    }))))
}

// POST /api/v1/admin/users/:id/deactivate
pub async fn admin_deactivate_user(
    Path(user_id): Path<Uuid>,
    Extension(admin): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<DeactivateUserRequest>,
) -> AppResult<impl IntoResponse> {
    if !admin.is_admin() {
        return Err(AppError::forbidden("Admin access required"));
    }

    state.auth_service.deactivate_user(user_id, request.reason).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "User deactivated successfully"
    }))))
}

// POST /api/v1/admin/changeurs/verify
pub async fn admin_verify_changeur(
    Extension(admin): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<VerifyChangeurRequest>,
) -> AppResult<impl IntoResponse> {
    if !admin.is_admin() {
        return Err(AppError::forbidden("Admin access required"));
    }

    state.auth_service
        .verify_changeur(
            request.changeur_id,
            "manual".to_string(),
            vec![request.notes.unwrap_or_default()],
        )
        .await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": if request.verified { 
            "Changeur verified successfully" 
        } else { 
            "Changeur verification revoked" 
        }
    }))))
}

// Request DTOs
#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct Disable2FARequest {
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct GetUsersQuery {
    pub role: Option<String>,
    pub status: Option<String>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct DeactivateUserRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyChangeurRequest {
    pub changeur_id: Uuid,
    pub verified: bool,
    pub notes: Option<String>,
}

// Session management endpoints
// GET /api/v1/auth/sessions
pub async fn get_active_sessions(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    let sessions = state.auth_service.get_active_sessions(user.id).await?;
    
    Ok((StatusCode::OK, Json(sessions)))
}

// POST /api/v1/auth/sessions/:id/revoke
pub async fn revoke_session(
    Path(session_id): Path<Uuid>,
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    state.auth_service.revoke_session(user.id, session_id.to_string()).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "Session revoked successfully"
    }))))
}

// POST /api/v1/auth/sessions/revoke-all
pub async fn revoke_all_sessions(
    Extension(user): Extension<CurrentUser>,
    State(state): State<std::sync::Arc<AppState>>,
) -> AppResult<impl IntoResponse> {
    state.auth_service.revoke_all_sessions(user.id).await?;
    
    Ok((StatusCode::OK, Json(serde_json::json!({
        "message": "All sessions revoked successfully"
    }))))
}