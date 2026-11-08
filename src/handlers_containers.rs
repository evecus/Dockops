//! Container / image / network / volume handlers, ported from
//! internal/handler/server.go.

use crate::compose;
use crate::docker::{strip_docker_headers, DockerClient};
use crate::filebrowser;
use crate::handler::{fail, fail500, ok, parse_json, required, sse_response, AppState};
use crate::parser;
use crate::requests::{CommandRequest, ComposeRequest, ImageRequest, NetworkRequest, VolumeRequest};
use crate::sse;
use axum::body::{Body, Bytes};
use axum::extract::{Multipart, Path, Query, State};
use axum::response::Response;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use tokio::sync::mpsc;

fn field(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn items(v: &Value) -> &[Value] {
    v.as_array().map(|a| a.as_slice()).unwrap_or_default()
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn list_or_null(items: Vec<String>) -> Value {
    if items.is_empty() {
        Value::Null
    } else {
        Value::Array(items.into_iter().map(|s| json!(s)).collect())
    }
}

fn sse_error(msg: &str, accel: bool) -> Response {
    let msg = msg.to_string();
    let stream = async_stream::stream! {
        yield Ok::<_, io::Error>(sse::event("error", &msg));
    };
    sse_response(Body::from_stream(stream), accel, true)
}

// ===== CONTAINERS =====

pub async fn list_containers(State(state): State<Arc<AppState>>) -> Response {
    let client = DockerClient::new();
    let containers = match client.list_containers().await {
        Ok(v) => v,
        Err(e) => return fail500(e),
    };
    let update_status = state.sched.get_image_update_status();
    let mut result = Vec::new();
    for ct in containers {
        let name = field(&ct, "name");
        let image = field(&ct, "image");
        let has_compose = state.compose.has_compose_file(&name);
        let mut info = json!({
            "name": name,
            "image": image,
            "state": field(&ct, "state"),
            "status": field(&ct, "status"),
            "ports": ct.get("ports").cloned().unwrap_or(Value::Null),
            "has_compose": has_compose,
            "image_update_available": false,
        });
        if has_compose {
            info.as_object_mut().unwrap().insert(
                "compose_dir".to_string(),
                json!(state.compose.get_compose_dir(&name)),
            );
        }
        if let Some(us) = update_status.get(&image) {
            info.as_object_mut().unwrap().insert(
                "image_update_available".to_string(),
                json!(us.get("has_update").and_then(|v| v.as_bool()).unwrap_or(false)),
            );
        }
        result.push(info);
    }
    ok(Value::Array(result))
}

pub async fn create_container_stream(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Response {
    let req: ComposeRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return sse_error(&e.to_string(), true),
    };
    if req.compose_content.is_empty() {
        return sse_error(
            "Key: 'ComposeContent' Error:Field validation for 'ComposeContent' failed on the 'required' tag",
            true,
        );
    }
    let mut name = req.name.trim().to_string();
    if name.is_empty() {
        name = compose::extract_name_from_compose(&req.compose_content);
    }
    if name.is_empty() {
        return sse_error("cannot determine container name from compose content", true);
    }

    let s = state.clone();
    let content = req.compose_content.clone();
    let stream = async_stream::stream! {
        yield Ok::<_, io::Error>(sse::event(
            "info",
            &format!("正在写入 Compose 配置: data/compose/{}/", name),
        ));
        if let Err(e) = s.compose.write_compose(&name, &content) {
            yield Ok(sse::event("error", &format!("写入配置失败: {}", e)));
            return;
        }
        yield Ok(sse::event("info", "正在拉取镜像并启动容器..."));

        let (tx, mut rx) = mpsc::channel::<String>(64);
        let name2 = name.clone();
        let s2 = s.clone();
        let handle = tokio::spawn(async move { s2.compose.up_stream(&name2, tx).await });

        while let Some(line) = rx.recv().await {
            yield Ok(sse::event("log", &line));
        }
        let up_err = match handle.await {
            Ok(r) => r,
            Err(e) => Err(e.to_string()),
        };
        if let Err(e) = up_err {
            s.compose.remove_compose_dir(&name);
            yield Ok(sse::event("error", &format!("启动失败: {}", e)));
            return;
        }
        yield Ok(sse::event("done", &name));
    };
    sse_response(Body::from_stream(stream), true, true)
}

pub async fn create_container(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Response {
    let req: ComposeRequest = match parse_json(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if req.compose_content.is_empty() {
        return required("ComposeContent");
    }
    let mut name = req.name.trim().to_string();
    if name.is_empty() {
        name = compose::extract_name_from_compose(&req.compose_content);
    }
    if name.is_empty() {
        return fail(400, "cannot determine container name from compose content");
    }
    if let Err(e) = state.compose.write_compose(&name, &req.compose_content) {
        return fail500(e);
    }
    if let Err(e) = state.compose.up(&name).await {
        state.compose.remove_compose_dir(&name);
        return fail(500, &format!("Container created but failed to start: {}", e));
    }
    ok(json!({ "message": "created", "name": name }))
}

pub async fn parse_docker_run(
    State(_state): State<Arc<AppState>>,
    body: Bytes,
) -> Response {
    let req: CommandRequest = match parse_json(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match parser::parse_docker_run(&req.command) {
        Ok(svc) => ok(json!({ "yaml": svc.to_yaml(), "service": svc.to_json() })),
        Err(e) => fail(400, &e),
    }
}

pub async fn get_container(State(_state): State<Arc<AppState>>, Path(name): Path<String>) -> Response {
    let client = DockerClient::new();
    match client.inspect_container(&name).await {
        Ok(detail) => ok(detail),
        Err(_) => fail(404, "container not found"),
    }
}

pub async fn get_container_form_data(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Response {
    let client = DockerClient::new();
    let detail = match client.inspect_container(&name).await {
        Ok(v) => v,
        Err(e) => return fail(404, &format!("container not found: {}", e)),
    };

    let mut ports = Vec::new();
    for p in items(&detail["ports"]) {
        let hp = field(p, "host_port");
        if !hp.is_empty() && hp != "0" {
            ports.push(format!(
                "{}:{}/{}",
                hp,
                field(p, "container_port"),
                field(p, "protocol")
            ));
        }
    }
    let mut volumes = Vec::new();
    for m in items(&detail["mounts"]) {
        let t = field(m, "type");
        if t == "bind" || t == "volume" {
            volumes.push(format!("{}:{}", field(m, "source"), field(m, "destination")));
        }
    }
    let mut env = Vec::new();
    for e in strs(&detail["env"]) {
        if e.contains('=') {
            env.push(e);
        }
    }
    let mut network_mode = String::new();
    if let Some(nets) = detail["networks"].as_object() {
        for net in nets.keys() {
            if net == "bridge" || net == "host" || net == "none" {
                network_mode = net.clone();
            }
            break; // Go only looks at the first network entry
        }
    }

    let fields = json!({
        "image": detail["image"],
        "restart": detail["restart_policy"],
        "hostname": detail["hostname"],
        "privileged": false,
        "ports": list_or_null(ports),
        "volumes": list_or_null(volumes),
        "env": list_or_null(env),
        "command": "",
        "entrypoint": "",
        "user": "",
        "network_mode": network_mode,
    });
    ok(json!({ "name": name, "fields": fields }))
}

pub async fn update_container(
    State(state): State<Arc<AppState>>,
    Path(old_name): Path<String>,
    body: Bytes,
) -> Response {
    let req: ComposeRequest = match parse_json(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if req.name.is_empty() {
        return required("Name");
    }
    if req.compose_content.is_empty() {
        return required("ComposeContent");
    }
    let new_name = req.name.clone();

    if new_name != old_name && compose::container_exists(&new_name).await {
        return fail(400, &format!("container '{}' already exists", new_name));
    }

    compose::stop_and_remove(&old_name).await;
    if new_name != old_name {
        state.compose.remove_compose_dir(&old_name);
    }
    if let Err(e) = state.compose.write_compose(&new_name, &req.compose_content) {
        return fail500(e);
    }
    if let Err(e) = state.compose.up(&new_name).await {
        return fail(500, &format!("Updated but failed to start: {}", e));
    }
    ok(json!({ "message": "updated", "name": new_name }))
}

pub async fn delete_container(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let remove_compose = params.get("remove_compose").map(|v| v.as_str()) == Some("true");
    if state.compose.has_compose_file(&name) {
        if let Err(e) = state.compose.down(&name).await {
            return fail500(e);
        }
        if remove_compose {
            state.compose.remove_compose_dir(&name);
        }
    } else {
        let client = DockerClient::new();
        let _ = client.stop_container(&name).await;
        if let Err(e) = client.remove_container(&name, true).await {
            return fail500(e);
        }
    }
    ok(json!({ "message": "deleted" }))
}

pub async fn start_container(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Response {
    if state.compose.has_compose_file(&name) {
        if let Err(e) = state.compose.up(&name).await {
            return fail500(e);
        }
    } else {
        let client = DockerClient::new();
        if let Err(e) = client.start_container(&name).await {
            return fail500(e);
        }
    }
    ok(json!({ "message": "started" }))
}

pub async fn stop_container(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Response {
    let client = DockerClient::new();
    match client.stop_container(&name).await {
        Ok(_) => ok(json!({ "message": "stopped" })),
        Err(e) => fail500(e),
    }
}

pub async fn restart_container(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Response {
    let client = DockerClient::new();
    match client.restart_container(&name).await {
        Ok(_) => ok(json!({ "message": "restarted" })),
        Err(e) => fail500(e),
    }
}

pub async fn container_stats(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Response {
    let client = DockerClient::new();
    match client.container_stats(&name).await {
        Ok(v) => ok(v),
        Err(e) => fail500(e),
    }
}

pub async fn container_logs(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let tail = params
        .get("tail")
        .cloned()
        .unwrap_or_else(|| "500".to_string());
    let client = DockerClient::new();
    let data = match client.container_logs(&name, &tail).await {
        Ok(v) => v,
        Err(e) => return fail500(e),
    };
    let stripped = strip_docker_headers(&data);
    ok(json!({ "logs": String::from_utf8_lossy(&stripped).to_string() }))
}

pub async fn list_files(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let path = params.get("path").cloned().unwrap_or_else(|| "/".to_string());
    let client = DockerClient::new();
    match filebrowser::list_container_files(&client, &name, &path).await {
        Ok(entries) => {
            if entries.is_empty() {
                ok(Value::Null)
            } else {
                ok(Value::Array(entries))
            }
        }
        Err(e) => fail500(e),
    }
}

pub async fn download_file(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let path = params.get("path").cloned().unwrap_or_default();
    let client = DockerClient::new();
    let mut reader = match client.download_container_file(&name, &path).await {
        Ok(v) => v,
        Err(e) => return fail500(e),
    };
    let stream = async_stream::stream! {
        while let Ok(Some(chunk)) = reader.next_chunk().await {
            yield Ok::<_, io::Error>(Bytes::from(chunk));
        }
    };
    Response::builder()
        .header("Content-Disposition", "attachment; filename=download.tar")
        .header("Content-Type", "application/x-tar")
        .body(Body::from_stream(stream))
        .unwrap()
}

pub async fn upload_file(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    mut multipart: Multipart,
) -> Response {
    let dst_path = params.get("path").cloned().unwrap_or_else(|| "/tmp".to_string());
    let mut data: Option<Vec<u8>> = None;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                if field.name() == Some("file") {
                    match field.bytes().await {
                        Ok(b) => {
                            data = Some(b.to_vec());
                            break;
                        }
                        Err(e) => return fail(400, &e.to_string()),
                    }
                }
            }
            Ok(None) => break,
            Err(e) => return fail(400, &e.to_string()),
        }
    }
    let data = match data {
        Some(d) => d,
        None => return fail(400, "http: no such file"),
    };
    let client = DockerClient::new();
    match client.upload_to_container(&name, &dst_path, data).await {
        Ok(_) => ok(json!({ "message": "uploaded" })),
        Err(e) => fail500(e),
    }
}

pub async fn delete_file(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let path = params.get("path").cloned().unwrap_or_default();
    let client = DockerClient::new();
    match client.delete_container_file(&name, &path).await {
        Ok(_) => ok(json!({ "message": "deleted" })),
        Err(e) => fail500(e),
    }
}

pub async fn update_container_image(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Response {
    let client = DockerClient::new();
    if state.compose.has_compose_file(&name) {
        if let Err(e) = state.compose.pull(&name).await {
            return fail500(e);
        }
        let _ = state.compose.down(&name).await;
        if let Err(e) = state.compose.up(&name).await {
            return fail500(e);
        }
    } else {
        if let Err(e) = client.rebuild_container(&name).await {
            return fail500(e);
        }
    }
    if let Ok(images) = client.list_images().await {
        for img in images {
            for tag in strs(&img["repo_tags"]) {
                state.sched.mark_image_updated(&tag);
            }
        }
    }
    ok(json!({ "message": "updated" }))
}

// ===== IMAGES =====

pub async fn check_image_update(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let tag = params.get("tag").cloned().unwrap_or_default();
    if tag.is_empty() {
        return sse_error("missing tag parameter", true);
    }
    let client = DockerClient::new();
    let sched = state.sched.clone();
    let stream = async_stream::stream! {
        yield Ok::<_, io::Error>(sse::event("checking", "正在获取当前版本信息..."));
        let old_id = client.get_image_id(&tag).await.unwrap_or_default();
        yield Ok(sse::event("pulling", "正在拉取最新版本..."));
        let mut reader = match client.stream_pull_image(&tag).await {
            Ok(r) => r,
            Err(e) => {
                yield Ok(sse::event("error", &format!("拉取失败: {}", e)));
                return;
            }
        };
        while let Ok(Some(chunk)) = reader.next_chunk().await {
            yield Ok(sse::event("progress", &String::from_utf8_lossy(&chunk)));
        }
        let new_id = match client.get_image_id(&tag).await {
            Ok(id) => id,
            Err(e) => {
                yield Ok(sse::event("error", &format!("获取新版本信息失败: {}", e)));
                return;
            }
        };
        if !old_id.is_empty() && old_id == new_id {
            yield Ok(sse::event("up-to-date", "当前版本已是最新版，无需更新"));
        } else {
            sched.mark_image_updated(&tag);
            yield Ok(sse::event("updated", "更新完成！镜像已更新到最新版本"));
        }
    };
    sse_response(Body::from_stream(stream), true, true)
}

pub async fn get_image_update_status(
    State(state): State<Arc<AppState>>,
) -> Response {
    ok(state.sched.get_image_update_status())
}

pub async fn list_images(State(_state): State<Arc<AppState>>) -> Response {
    let client = DockerClient::new();
    match client.list_images().await {
        Ok(v) => ok(Value::Array(v)),
        Err(e) => fail500(e),
    }
}

pub async fn pull_image(
    State(_state): State<Arc<AppState>>,
    body: Bytes,
) -> Response {
    let req: ImageRequest = match parse_json(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let client = DockerClient::new();
    let stream = async_stream::stream! {
        let mut reader = match client.stream_pull_image(&req.image).await {
            Ok(r) => r,
            Err(e) => {
                yield Ok::<_, io::Error>(sse::event("error", &e));
                return;
            }
        };
        while let Ok(Some(chunk)) = reader.next_chunk().await {
            yield Ok(sse::event("progress", &String::from_utf8_lossy(&chunk)));
        }
        yield Ok(sse::event("done", "pull complete"));
    };
    sse_response(Body::from_stream(stream), false, true)
}

pub async fn load_image(
    State(_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Response {
    let mut data: Option<Vec<u8>> = None;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                if field.name() == Some("file") {
                    match field.bytes().await {
                        Ok(b) => {
                            data = Some(b.to_vec());
                            break;
                        }
                        Err(e) => return fail(400, &e.to_string()),
                    }
                }
            }
            Ok(None) => break,
            Err(e) => return fail(400, &e.to_string()),
        }
    }
    let data = match data {
        Some(d) => d,
        None => return fail(400, "http: no such file"),
    };
    let client = DockerClient::new();
    match client.load_image(data).await {
        Ok(_) => ok(json!({ "message": "image loaded" })),
        Err(e) => fail500(e),
    }
}

pub async fn delete_image(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    // Go's *id wildcard includes a leading slash.
    let id = id.trim_start_matches('/').to_string();
    let force = params.get("force").map(|v| v.as_str()) == Some("true");
    let client = DockerClient::new();
    match client.remove_image(&id, force).await {
        Ok(_) => ok(json!({ "message": "deleted" })),
        Err(e) => fail500(e),
    }
}

// ===== NETWORKS =====

pub async fn list_networks(State(_state): State<Arc<AppState>>) -> Response {
    let client = DockerClient::new();
    match client.list_networks().await {
        Ok(v) => ok(Value::Array(v)),
        Err(e) => fail500(e),
    }
}

pub async fn create_network(
    State(_state): State<Arc<AppState>>,
    body: Bytes,
) -> Response {
    let mut req: NetworkRequest = match parse_json(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if req.driver.is_empty() {
        req.driver = "bridge".to_string();
    }
    let client = DockerClient::new();
    match client.create_network(&req.name, &req.driver).await {
        Ok(_) => ok(json!({ "message": "created" })),
        Err(e) => fail500(e),
    }
}

pub async fn delete_network(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    let client = DockerClient::new();
    match client.remove_network(&id).await {
        Ok(_) => ok(json!({ "message": "deleted" })),
        Err(e) => fail500(e),
    }
}

pub async fn prune_networks(State(_state): State<Arc<AppState>>) -> Response {
    let client = DockerClient::new();
    let _ = client.prune_networks().await; // Go ignores the error
    ok(json!({ "message": "pruned" }))
}

// ===== VOLUMES =====

pub async fn list_volumes(State(_state): State<Arc<AppState>>) -> Response {
    let client = DockerClient::new();
    match client.list_volumes().await {
        Ok(v) => ok(Value::Array(v)),
        Err(e) => fail500(e),
    }
}

pub async fn create_volume(
    State(_state): State<Arc<AppState>>,
    body: Bytes,
) -> Response {
    let req: VolumeRequest = match parse_json(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let client = DockerClient::new();
    match client.create_volume(&req.name).await {
        Ok(_) => ok(json!({ "message": "created" })),
        Err(e) => fail500(e),
    }
}

pub async fn delete_volume(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let force = params.get("force").map(|v| v.as_str()) == Some("true");
    let client = DockerClient::new();
    match client.remove_volume(&name, force).await {
        Ok(_) => ok(json!({ "message": "deleted" })),
        Err(e) => fail500(e),
    }
}

pub async fn prune_volumes(State(_state): State<Arc<AppState>>) -> Response {
    let client = DockerClient::new();
    let _ = client.prune_volumes().await; // Go ignores the error
    ok(json!({ "message": "pruned" }))
}
