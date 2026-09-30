use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const JWT_SECRET_DEFAULT: &str = "splitter_default_secret_key_change_in_production_12345";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user_id
    pub email: String,
    pub display_name: String,
    pub is_admin: bool,
    pub exp: usize,
    pub iat: usize,
}

pub fn get_jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| JWT_SECRET_DEFAULT.to_string())
}

pub fn issue_token(
    user_id: &str,
    email: &str,
    display_name: &str,
    is_admin: bool,
) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let exp = now + 30 * 24 * 3600; // 30 days
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        display_name: display_name.to_string(),
        is_admin,
        exp,
        iat: now,
    };

    let secret = get_jwt_secret();
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Token generation error: {}", e)))
}

pub fn verify_token(token: &str) -> Result<Claims, AppError> {
    let secret = get_jwt_secret();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| AppError::Unauthorized(format!("Invalid or expired token: {}", e)))?;

    Ok(token_data.claims)
}

#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|val| val.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized(
                "Invalid Authorization header format. Expected 'Bearer <token>'".to_string(),
            ));
        }

        let token = &auth_header["Bearer ".len()..];
        let claims = verify_token(token)?;
        Ok(AuthUser(claims))
    }
}

#[derive(Debug, Clone)]
pub struct AdminUser(pub Claims);

#[async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthUser(claims) = AuthUser::from_request_parts(parts, state).await?;
        if !claims.is_admin {
            return Err(AppError::Forbidden(
                "Administrator privileges required".to_string(),
            ));
        }
        Ok(AdminUser(claims))
    }
}
