mod jwt_structure;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use jwt_structure::RefreshTokenPayload;
use std::env;

pub fn create_access_jwt(
    id: &uuid::Uuid,
    user_id: &str,
    time: Duration,
) -> Result<String, jsonwebtoken::errors::Error> {
    let secret_key = match env::var("SECRET_KEY") {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Use in create 'default' as key SECRET_KEY not found in env");
            "default".to_string()
        }
    };
    let expiration = Utc::now()
        .checked_add_signed(time)
        .expect("Invalid timestamp")
        .timestamp();

    let claims = jwt_structure::AccessToken::new(id, user_id, expiration as usize);

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret_key.as_bytes()),
    )
}

pub fn create_refresh_jwt(
    id: &uuid::Uuid,
    browser: &str,
    device: &str,
    os: &str,
    time: Duration,
) -> Result<String, jsonwebtoken::errors::Error> {
    let secret_key = match env::var("SECRET_KEY") {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Use in create 'default' as key SECRET_KEY not found in env");
            "default".to_string()
        }
    };
    let expiration = Utc::now()
        .checked_add_signed(time)
        .expect("Invalid timestamp")
        .timestamp();

    let claims = jwt_structure::RefreshToken::new(id, browser, device, os, expiration as usize);

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret_key.as_bytes()),
    )
}

pub fn validate_access_jwt(
    token: &str,
) -> Result<jwt_structure::AccessTokenPayload, jsonwebtoken::errors::Error> {
    let secret_key = match env::var("SECRET_KEY") {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Use in validate 'default' as key SECRET_KEY not found in env");
            "default".to_string()
        }
    };
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.leeway = 0;

    let token_data = decode::<jwt_structure::AccessTokenPayload>(
        token,
        &DecodingKey::from_secret(secret_key.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

pub fn validate_refresh_jwt(
    token: &str,
) -> Result<jwt_structure::RefreshTokenPayload, jsonwebtoken::errors::Error> {
    let secret_key = match env::var("SECRET_KEY") {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Use in validate 'default' as key SECRET_KEY not found in env");
            "default".to_string()
        }
    };
    let token_data = decode::<jwt_structure::RefreshTokenPayload>(
        token,
        &DecodingKey::from_secret(secret_key.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;

    Ok(token_data.claims)
}

pub fn validate_data_token_refresh(
    refresh_token: &RefreshTokenPayload,
    browser: &str,
    os: &str,
    device: &str,
) -> bool {
    // println!("Token browser: {}", refresh_token.browser);
    // println!("Input browser: {}", browser);
    // println!("Browser match: {}", refresh_token.browser == browser);

    // println!("Token OS: {}", refresh_token.os);
    // println!("Input OS: {}", os);
    // println!("OS match: {}", refresh_token.os == os);

    // println!("Token device: {}", refresh_token.device);
    // println!("Input device: {}", device);
    // println!("Device match: {}", refresh_token.device == device);

    refresh_token.browser == browser && refresh_token.os == os && refresh_token.device == device
}
