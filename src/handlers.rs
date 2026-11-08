//! HTTP handlers: auth, dashboard, settings, plus shared request types.
//! Container/image/network/volume handlers live in handlers_containers.rs.

pub use crate::handlers_containers::*;
pub use crate::requests::*;

use crate::auth;
use crate::docker::DockerClient;
use crate::handler::{fail, fail500, ok, parse_json, required, sse_response, AppState};
use crate::sse;
use axum::body::{Body, Bytes};
use axum::extract::State;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

// ===== AUTH =====

pub async fn system_status(State(state): State<Arc<AppState>>) -> axum::response::Response {
    ok(json!({ "setup": state.db.is_setup(), "version": state.version }))
}

pub async fn setup(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> axum::response::Response {
    if state.db.is_setup() {
        return fail(400, "already setup");
    }
    let req: Credentials = match parse_json(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if req.username.is_empty() {
        return required("Username");
    }
    if req.password.is_empty() {
        return required("Password");
    }
    if let Err(e) = auth::create_admin(&state.db, &req.username, &req.password) {
        return fail500(e);
    }
    ok(json!({ "message": "admin created" }))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> axum::response::Response {
    let req: Credentials = match parse_json(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if req.username.is_empty() {
        return required("Username");
    }
    if req.password.is_empty() {
        return required("Password");
    }
    match auth::login(&state.db, &req.username, &req.password) {
        Ok(token) => ok(json!({ "token": token })),
        Err(e) => fail(401, &e),
    }
}

// ===== DASHBOARD =====

pub async fn dashboard_info(
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    let (info, _) = state.sched.cache_get();
    if let Some(info) = info {
        return ok(info);
    }
    let client = DockerClient::new();
    match client.get_system_info().await {
        Ok(info) => ok(info),
        Err(e) => fail500(e),
    }
}

pub async fn dashboard_stats(
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    let (_, stats) = state.sched.cache_get();
    match stats {
        Some(s) => ok(s),
        None => ok(json!({})),
    }
}

pub async fn dashboard_refresh(
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    // Collect synchronously so the response contains fresh data.
    state.sched.collect_now();
    let client = DockerClient::new();
    let info = match client.get_system_info().await {
        Ok(v) => v,
        Err(e) => return fail500(e),
    };
    let containers = client.list_containers().await.unwrap_or_default();
    let mut total_cpu = 0.0f64;
    let mut total_mem = 0u64;
    let mut total_mem_limit = 0u64;
    for ct in &containers {
        if ct.get("state").and_then(|v| v.as_str()) == Some("running") {
            let id = ct.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if let Ok(st) = client.container_stats(id).await {
                total_cpu += st.get("cpu_percent").and_then(|v| v.as_f64()).unwrap_or(0.0);
                total_mem += st.get("memory_usage").and_then(|v| v.as_u64()).unwrap_or(0);
                total_mem_limit += st
                    .get("memory_limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }
        }
    }
    ok(json!({
        "info": info,
        "stats": {
            "total_cpu_percent": total_cpu,
            "total_mem_usage": total_mem,
            "total_mem_limit": total_mem_limit,
            "containers": containers.len(),
        }
    }))
}

// ===== SETTINGS =====

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    let mut settings = match state.db.get_all_settings() {
        Ok(m) => m,
        Err(e) => return fail500(e),
    };
    let admin = auth::get_admin(&state.db);
    settings.insert("admin_username".to_string(), admin.username);
    let mut map = serde_json::Map::new();
    for (k, v) in settings {
        map.insert(k, json!(v));
    }
    ok(Value::Object(map))
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> axum::response::Response {
    let req: HashMap<String, String> = match parse_json(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    for (k, v) in req {
        let allowed = matches!(
            k.as_str(),
            "docker_proxy" | "collect_interval" | "update_check_interval"
        );
        if !allowed {
            continue;
        }
        if let Err(e) = state.db.set_setting(&k, &v) {
            return fail500(e);
        }
        if k == "collect_interval" {
            let _ = state.sched.update_collect_interval(&v);
        }
        if k == "update_check_interval" {
            let _ = state.sched.update_image_check_interval(&v);
        }
    }
    ok(json!({ "message": "settings updated" }))
}

pub async fn update_admin(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> axum::response::Response {
    let req: Credentials = match parse_json(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(e) = auth::update_admin(&state.db, &req.username, &req.password) {
        return fail500(e);
    }
    ok(json!({ "message": "admin updated" }))
}

pub async fn install_docker(
    State(_state): State<Arc<AppState>>,
) -> axum::response::Response {
    let stream = async_stream::stream! {
        yield Ok::<_, std::io::Error>(sse::event("info", "Starting Docker installation..."));
        yield Ok::<_, std::io::Error>(sse::event("info", "Run: curl -fsSL https://get.docker.com | sh"));
        yield Ok::<_, std::io::Error>(sse::event("done", ""));
    };
    sse_response(Body::from_stream(stream), false, false)
}
