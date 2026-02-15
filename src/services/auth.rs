use crate::models::auth::{LoginRequest, LoginResponse, PasswordChangeRequest, User};
use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

const JWT_SECRET: &[u8] = b"shaydz-secret-key-change-in-production";
const SESSION_DURATION_HOURS: i64 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
}

pub struct AuthService {
    users: Arc<RwLock<HashMap<String, User>>>,
    sessions: Arc<RwLock<HashMap<String, String>>>, // token -> username
}

impl AuthService {
    pub fn new() -> Self {
        let mut users = HashMap::new();
        
        // Create default admin user
        let default_user = User {
            username: "admin".to_string(),
            password_hash: "$argon2i$v=19$m=4096,t=3,p=1$SHhhZFpNdWx0aU1vbml0b3I$V2VsY29tZVRvU2hheWRa".to_string(),
            created_at: Utc::now(),
            last_login: None,
        };
        users.insert("admin".to_string(), default_user);
        
        Self {
            users: Arc::new(RwLock::new(users)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn login(&self, req: LoginRequest) -> Result<LoginResponse> {
        // Validate input length
        if req.username.len() > 64 || req.password.len() > 128 {
            return Ok(LoginResponse {
                success: false,
                message: "Invalid credentials".to_string(),
                token: None,
            });
        }
        
        let users = self.users.read().await;
        
        if let Some(user) = users.get(&req.username) {
            // Verify password
            let argon2 = Argon2::default();
            if let Ok(parsed_hash) = PasswordHash::new(&user.password_hash) {
                if argon2.verify_password(req.password.as_bytes(), &parsed_hash).is_ok() {
                    // Generate JWT token
                    let now = Utc::now();
                    let exp = now + Duration::hours(SESSION_DURATION_HOURS);
                    
                    let claims = Claims {
                        sub: req.username.clone(),
                        exp: exp.timestamp() as usize,
                        iat: now.timestamp() as usize,
                    };
                    
                    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET))?;
                    
                    // Store session
                    drop(users);
                    let mut sessions = self.sessions.write().await;
                    sessions.insert(token.clone(), req.username.clone());
                    
                    return Ok(LoginResponse {
                        success: true,
                        message: "Login successful".to_string(),
                        token: Some(token),
                    });
                }
            }
        }
        
        Ok(LoginResponse {
            success: false,
            message: "Invalid credentials".to_string(),
            token: None,
        })
    }
    
    pub async fn verify_token(&self, token: &str) -> Result<String> {
        let validation = Validation::default();
        let token_data = decode::<Claims>(token, &DecodingKey::from_secret(JWT_SECRET), &validation)?;
        
        // Check if session exists
        let sessions = self.sessions.read().await;
        if sessions.get(token).is_some() {
            Ok(token_data.claims.sub)
        } else {
            Err(anyhow!("Session not found"))
        }
    }
    
    pub async fn logout(&self, token: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(token);
        Ok(())
    }
    
    pub async fn change_password(&self, username: &str, req: PasswordChangeRequest) -> Result<()> {
        // Validate input
        if req.current_password.len() > 128 || req.new_password.len() > 128 {
            return Err(anyhow!("Password too long"));
        }
        
        if req.new_password.len() < 1 {
            return Err(anyhow!("New password cannot be empty"));
        }
        
        if req.new_password != req.confirm_password {
            return Err(anyhow!("Passwords do not match"));
        }
        
        let mut users = self.users.write().await;
        
        if let Some(user) = users.get_mut(username) {
            // Verify current password
            let argon2 = Argon2::default();
            if let Ok(parsed_hash) = PasswordHash::new(&user.password_hash) {
                if argon2.verify_password(req.current_password.as_bytes(), &parsed_hash).is_ok() {
                    // Hash new password
                    let salt = SaltString::generate(&mut OsRng);
                    let new_hash = argon2.hash_password(req.new_password.as_bytes(), &salt)
                        .map_err(|e| anyhow!("Password hashing failed: {:?}", e))?;
                    
                    user.password_hash = new_hash.to_string();
                    return Ok(());
                }
            }
            return Err(anyhow!("Current password incorrect"));
        }
        
        Err(anyhow!("User not found"))
    }
}
