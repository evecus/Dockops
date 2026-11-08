//! Router, auth middleware and response helpers (gin equivalent).

use crate::compose::Manager;
use crate::config::Config;
use crate::db::Db;
use crate::scheduler::Scheduler;
use crate::{auth, handlers, web, ws};
use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{header, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct AppState {
    pub _cfg: Config,
    pub db: Arc<Db>,
    pub sched: Arc<Scheduler>,
    pub compose: Manager,
    pub version: String,
}

// ===== Response helpers (mirroring ok()/fail() in Go) =====

pub fn ok(data: Value) -> Response {
    Json(json!({ "code": 0, "data": data })).into_response()
}

pub fn fail(status: u16, msg: &str) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(json!({ "code": -1, "error": msg })),
    )
        .into_response()
}

pub fn fail500<E: ToString>(err: E) -> Response {
    fail(500, &err.to_string())
}

/// gin's `binding:"required"` error shape.
pub fn required(field: &str) -> Response {
    fail(
        400,
        &format!(
            "Key: '{}' Error:Field validation for '{}' failed on the 'required' tag",
            field, field
        ),
    )
}

pub fn parse_json<T: DeserializeOwned>(body: &Bytes) -> std::result::Result<T, Response> {
    serde_json::from_slice(body).map_err(|e| fail(400, &e.to_string()))
}

pub fn sse_response(body: Body, accel_buffering: bool, keep_alive: bool) -> Response {
    let mut b = Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache");
    if keep_alive {
        b = b.header(header::CONNECTION, "keep-alive");
    }
    if accel_buffering {
        b = b.header("X-Accel-Buffering", "no");
    }
    b.body(body).unwrap()
}

// ===== Auth middleware =====

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(a), Some(b)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(a * 16 + b);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn query_param(uri: &Uri, key: &str) -> Option<String> {
    let q = uri.query()?;
    for pair in q.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k == key {
            return Some(percent_decode(v));
        }
    }
    None
}

/// Equivalent of middleware.Auth(): Authorization header first, then ?token=,
/// optional "Bearer " prefix, then JWT validation.
async fn auth_middleware(
    State(_state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let mut token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if token.is_empty() {
        token = query_param(req.uri(), "token").unwrap_or_default();
    }
    if let Some(rest) = token.strip_prefix("Bearer ") {
        token = rest.to_string();
    }
    if token.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
            .into_response();
    }
    let claims = match auth::validate_token(&token) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "invalid token" })),
            )
                .into_response()
        }
    };
    req.extensions_mut().insert(claims.username);
    next.run(req).await
}

/// WebSocket endpoints validate the ?token= query *before* the upgrade,
/// exactly like the Go handlers (so a plain GET without a token gets 401,
/// not axum's 400 "missing Upgrade header").
async fn ws_auth_middleware(req: Request, next: Next) -> Response {
    let token = query_param(req.uri(), "token").unwrap_or_default();
    if auth::validate_token(&token).is_err() {
        return (StatusCode::UNAUTHORIZED, "unauthorized\n").into_response();
    }
    next.run(req).await
}

// ===== Router =====

pub fn router(state: Arc<AppState>) -> Router {
    let public = Router::new()
        .route("/api/system/status", get(handlers::system_status))
        .route("/api/auth/setup", post(handlers::setup))
        .route("/api/auth/login", post(handlers::login));

    let protected = Router::new()
        .route("/api/dashboard/info", get(handlers::dashboard_info))
        .route("/api/dashboard/stats", get(handlers::dashboard_stats))
        .route("/api/dashboard/refresh", post(handlers::dashboard_refresh))
        // Containers — :name is the docker container name
        .route("/api/containers", get(handlers::list_containers).post(handlers::create_container))
        .route("/api/containers/stream", post(handlers::create_container_stream))
        .route("/api/containers/parse-run", post(handlers::parse_docker_run))
        .route("/api/containers/:name", get(handlers::get_container).put(handlers::update_container).delete(handlers::delete_container))
        .route("/api/containers/:name/form-data", get(handlers::get_container_form_data))
        .route("/api/containers/:name/start", post(handlers::start_container))
        .route("/api/containers/:name/stop", post(handlers::stop_container))
        .route("/api/containers/:name/restart", post(handlers::restart_container))
        .route("/api/containers/:name/stats", get(handlers::container_stats))
        .route("/api/containers/:name/logs", get(handlers::container_logs))
        .route("/api/containers/:name/files", get(handlers::list_files).delete(handlers::delete_file))
        .route("/api/containers/:name/files/download", get(handlers::download_file))
        .route("/api/containers/:name/files/upload", post(handlers::upload_file))
        .route("/api/containers/:name/update", post(handlers::update_container_image))
        // Images
        .route("/api/images", get(handlers::list_images))
        .route("/api/images/check-update", get(handlers::check_image_update))
        .route("/api/images/update-status", get(handlers::get_image_update_status))
        .route("/api/images/pull", post(handlers::pull_image))
        .route("/api/images/load", post(handlers::load_image))
        .route("/api/images/*id", delete(handlers::delete_image))
        // Networks
        .route("/api/networks", get(handlers::list_networks).post(handlers::create_network))
        .route("/api/networks/:id", delete(handlers::delete_network))
        .route("/api/networks/prune", post(handlers::prune_networks))
        // Volumes
        .route("/api/volumes", get(handlers::list_volumes).post(handlers::create_volume))
        .route("/api/volumes/:name", delete(handlers::delete_volume))
        .route("/api/volumes/prune", post(handlers::prune_volumes))
        // Settings
        .route("/api/settings", get(handlers::get_settings).put(handlers::update_settings))
        .route("/api/settings/install-docker", post(handlers::install_docker))
        .route("/api/settings/admin", put(handlers::update_admin))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let ws_routes = Router::new()
        .route("/ws/containers/:name/terminal", get(ws::terminal))
        .route("/ws/containers/:name/logs", get(ws::logs))
        .route_layer(middleware::from_fn(ws_auth_middleware));

    Router::new()
        .merge(public)
        .merge(protected)
        .merge(ws_routes)
        .fallback(fallback)
        .with_state(state)
}

async fn fallback(State(_state): State<Arc<AppState>>, req: Request) -> Response {
    let path = req.uri().path().to_string();
    if web::is_api_or_ws(&path) {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "not found" })),
        )
            .into_response();
    }
    web::serve(&path)
}
