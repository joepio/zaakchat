use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::env;

/// Claims structure for JWT
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (User ID / Email)
    pub sub: String,
    /// Expiration time (as UTC timestamp)
    pub exp: usize,
    /// Issued at (as UTC timestamp)
    pub iat: usize,
}

/// Authenticated User Extractor
///
/// This struct implements `FromRequestParts` to automatically extract and validate
/// the JWT from the `Authorization` header.
#[derive(Debug)]
pub struct AuthUser {
    pub user_id: String,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Get Authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // 2. Check for "Bearer " prefix
        if !auth_header.starts_with("Bearer ") {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let token = &auth_header[7..];

        // 3. Decode and validate token
        let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let validation = Validation::default();

        match decode::<Claims>(token, &decoding_key, &validation) {
            Ok(token_data) => Ok(AuthUser {
                user_id: token_data.claims.sub,
            }),
            Err(_) => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

/// Helper to create a JWT for a user with default 24h expiration
pub fn create_jwt(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    create_jwt_with_expiry(user_id, chrono::Duration::hours(24))
}

/// Helper to create a JWT for a user with custom expiration
pub fn create_jwt_with_expiry(user_id: &str, duration: chrono::Duration) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let expiration = chrono::Utc::now()
        .checked_add_signed(duration)
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_owned(),
        iat: chrono::Utc::now().timestamp() as usize,
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Helper to verify a JWT token
pub fn verify_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::default();

    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_creation_and_validation() {
        let user_id = "alice@example.com";
        let token = create_jwt(user_id).expect("failed to create token");

        let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let validation = Validation::default();

        let token_data = decode::<Claims>(&token, &decoding_key, &validation)
            .expect("failed to decode token");

        assert_eq!(token_data.claims.sub, user_id);
    }
}
