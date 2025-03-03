mod jwt_structure;

use std::env;
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use chrono::{Utc, Duration};

pub fn create_access_jwt(user_id: &str, time: Duration) -> Result<String, jsonwebtoken::errors::Error> {
    let secret_key = match env::var("SECRET_KEY")  {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Use in create 'default' as key SECRET_KEY not found in env");
            "default".to_string()
        },
    }; 
    let expiration = Utc::now()
        .checked_add_signed(time) 
        .expect("Invalid timestamp")
        .timestamp();

    let claims = jwt_structure::AccessToken::new(user_id, expiration as usize);

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret_key.as_bytes()),
    )
}

pub fn create_refresh_jwt(user_id: &str, browser: &str, device:&str, os:&str, time: Duration) -> Result<String, jsonwebtoken::errors::Error> {
    let secret_key = match env::var("SECRET_KEY")  {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Use in create 'default' as key SECRET_KEY not found in env");
            "default".to_string()
        },
    }; 
    let expiration = Utc::now()
        .checked_add_signed(time) 
        .expect("Invalid timestamp")
        .timestamp();

    let claims = jwt_structure::RefreshToken::new(user_id, browser, device, os, expiration as usize);

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret_key.as_bytes()),
    )
}

pub fn validate_access_jwt(token: &str) -> Result<jwt_structure::AccessTokenPayload, jsonwebtoken::errors::Error> {
    let secret_key = match env::var("SECRET_KEY")  {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Use in validate 'default' as key SECRET_KEY not found in env");
            "default".to_string()
        },
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

pub fn validate_refresh_jwt(token: &str) -> Result<jwt_structure::RefreshTokenPayload, jsonwebtoken::errors::Error> {
    let secret_key = match env::var("SECRET_KEY")  {
        Ok(s) => s,
        Err(_) => {
            log::warn!("Use in validate 'default' as key SECRET_KEY not found in env");
            "default".to_string()
        },
    }; 
    let token_data = decode::<jwt_structure::RefreshTokenPayload>(
        token,
        &DecodingKey::from_secret(secret_key.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;

    Ok(token_data.claims)
}