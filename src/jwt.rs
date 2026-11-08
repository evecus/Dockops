//! HS256 JWT implementation, byte-compatible with github.com/golang-jwt/jwt/v5
//! used by the Go backend (header {"alg":"HS256","typ":"JWT"}, claims
//! {username, exp, iat}, base64url without padding, HMAC-SHA256).
//! Implemented locally to keep the musl build free of any C/crypto toolchain.

use crate::error::Result;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;

pub const JWT_SECRET: &[u8] = b"dockops-secret-change-in-prod";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    pub username: String,
    pub exp: i64,
    pub iat: i64,
}

fn b64(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

fn sign(input: &str) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(JWT_SECRET).expect("hmac key");
    mac.update(input.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn encode(username: &str) -> String {
    let now = Utc::now().timestamp();
    let claims = Claims {
        username: username.to_string(),
        exp: now + 24 * 3600,
        iat: now,
    };
    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let payload = serde_json::to_string(&claims).unwrap();
    let input = format!("{}.{}", b64(header.as_bytes()), b64(payload.as_bytes()));
    let sig = sign(&input);
    format!("{}.{}", input, b64(&sig))
}

pub fn validate(token: &str) -> Result<Claims> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("invalid token".to_string());
    }
    let header_bytes = URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|_| "invalid token".to_string())?;
    let header: Value = serde_json::from_slice(&header_bytes)
        .map_err(|_| "invalid token".to_string())?;
    if header.get("alg").and_then(|v| v.as_str()) != Some("HS256") {
        return Err("invalid token".to_string());
    }

    let input = format!("{}.{}", parts[0], parts[1]);
    let expected = sign(&input);
    let actual = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| "invalid token".to_string())?;
    if !ct_eq(&expected, &actual) {
        return Err("invalid token".to_string());
    }

    let payload = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| "invalid token".to_string())?;
    let claims: Claims = serde_json::from_slice(&payload)
        .map_err(|_| "invalid token".to_string())?;
    // jwt/v5 validates exp by default; iat is not validated.
    if claims.exp <= Utc::now().timestamp() {
        return Err("token is expired".to_string());
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let t = encode("admin");
        let c = validate(&t).unwrap();
        assert_eq!(c.username, "admin");
        assert_eq!(c.exp - c.iat, 24 * 3600);
    }

    #[test]
    fn tampered_token_rejected() {
        let t = encode("admin");
        let mut chars: Vec<char> = t.chars().collect();
        let i = chars.len() - 3;
        chars[i] = if chars[i] == 'A' { 'B' } else { 'A' };
        let bad: String = chars.into_iter().collect();
        assert!(validate(&bad).is_err());
    }

    #[test]
    fn expired_token_rejected() {
        let now = Utc::now().timestamp();
        let claims = Claims {
            username: "admin".into(),
            exp: now - 10,
            iat: now - 20,
        };
        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let input = format!(
            "{}.{}",
            b64(header.as_bytes()),
            b64(serde_json::to_string(&claims).unwrap().as_bytes())
        );
        let token = format!("{}.{}", input, b64(&sign(&input)));
        assert!(validate(&token).is_err());
    }
}
