use crate::db::{Admin, Db};
use crate::error::Result;
use crate::jwt::{self, Claims};
use std::sync::Arc;

/// golang.org/x/crypto/bcrypt uses bcrypt.DefaultCost (10) and emits a `$2a$`
/// prefix. The Rust bcrypt crate emits `$2b$`; both are algorithmically
/// identical for ASCII passwords, but we normalise the prefix so the stored
/// hash is byte-for-byte the same shape as the Go version's.
fn hash_password(password: &str) -> Result<String> {
    let hash = bcrypt::hash(password, 10).map_err(|e| e.to_string())?;
    let normalized = if hash.starts_with("$2b$") {
        format!("$2a${}", &hash[4..])
    } else {
        hash
    };
    Ok(normalized)
}

pub fn create_admin(db: &Arc<Db>, username: &str, password: &str) -> Result<()> {
    let hash = hash_password(password)?;
    db.insert_admin(username, &hash)
}

pub fn update_admin(db: &Arc<Db>, username: &str, password: &str) -> Result<()> {
    let hash = hash_password(password)?;
    db.update_admin(username, &hash)
}

pub fn login(db: &Arc<Db>, username: &str, password: &str) -> Result<String> {
    let admin: Admin = db
        .find_admin_by_username(username)
        .ok_or_else(|| "invalid credentials".to_string())?;
    if bcrypt::verify(password, &admin.password_hash).unwrap_or(false) {
        Ok(jwt::encode(&admin.username))
    } else {
        Err("invalid credentials".to_string())
    }
}

pub fn validate_token(token: &str) -> Result<Claims> {
    jwt::validate(token)
}

pub fn get_admin(db: &Arc<Db>) -> Admin {
    db.get_admin()
}
