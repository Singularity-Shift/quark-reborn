use crate::helpers::dto::GroupClaims;

use super::dto::Claims;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use std::env;
use teloxide::types::{ChatId, UserId};

#[derive(Clone)]
pub struct JwtManager {
    secret: String,
}

impl JwtManager {
    pub fn new() -> Self {
        let secret = env::var("SECRET").expect("SECRET environment variable not found");
        JwtManager { secret }
    }

    pub fn generate_token(
        &self,
        telegram_id: UserId,
        account_address: String,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = Utc::now();
        let expiration = now + Duration::days(7);

        let claims = Claims {
            telegram_id,
            exp: expiration.timestamp(),
            iat: now.timestamp(),
            account_address,
            group_id: None,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let token_data: TokenData<Claims> = decode(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    pub fn is_token_valid(&self, token: &str) -> bool {
        match self.validate_token(token) {
            Ok(claims) => {
                let now = Utc::now().timestamp();
                claims.exp > now
            }
            Err(_) => false,
        }
    }

    pub fn get_or_generate_token(
        &self,
        existing_token: Option<&str>,
        telegram_id: UserId,
        account_address: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(token) = existing_token {
            if self.is_token_valid(token) {
                return Ok(token.to_string());
            }
        }

        // Generate new token if none exists or current one is invalid
        self.generate_token(telegram_id, account_address)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub fn validate_and_update_jwt(
        &self,
        mut jwt: String,
        telegram_id: UserId,
        account_address: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let existing_token = if jwt.is_empty() {
            None
        } else {
            Some(jwt.as_str())
        };

        let token = self.get_or_generate_token(existing_token, telegram_id, account_address)?;
        jwt = token;

        Ok(jwt)
    }

    pub fn generate_group_token(
        &self,
        group_id: ChatId,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = Utc::now();
        let expiration = now + Duration::days(7);

        let claims = GroupClaims {
            group_id,
            exp: expiration.timestamp(),
            iat: now.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
    }

    pub fn validate_group_token(
        &self,
        token: &str,
    ) -> Result<GroupClaims, jsonwebtoken::errors::Error> {
        let token_data: TokenData<GroupClaims> = decode(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    pub fn is_group_token_valid(&self, token: &str) -> bool {
        match self.validate_group_token(token) {
            Ok(claims) => {
                let now = Utc::now().timestamp();
                claims.exp > now
            }
            Err(_) => false,
        }
    }

    pub fn get_or_generate_group_token(
        &self,
        existing_token: Option<&str>,
        group_id: ChatId,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(token) = existing_token {
            if self.is_token_valid(token) {
                return Ok(token.to_string());
            }
        }

        // Generate new token if none exists or current one is invalid
        self.generate_group_token(group_id)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub fn validate_and_update_group_jwt(
        &self,
        mut jwt: String,
        group_id: ChatId,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let existing_token = if jwt.is_empty() {
            None
        } else {
            Some(jwt.as_str())
        };

        let token = self.get_or_generate_group_token(existing_token, group_id)?;
        jwt = token;

        Ok(jwt)
    }
}
