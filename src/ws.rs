//! WebSocket endpoints (terminal + live logs), ported from internal/ws/ws.go.

use crate::auth;
use crate::docker::DockerClient;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

#[derive(Deserialize)]
struct TerminalMessage {
    #[serde(rename = "type", default)]
    typ: String,
    #[serde(default)]
    data: String,
    #[serde(default)]
    rows: u32,
    #[serde(default)]
    cols: u32,
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, "unauthorized\n").into_response()
}

fn token_from(params: &HashMap<String, String>) -> String {
    params.get("token").cloned().unwrap_or_default()
}

pub async fn terminal(
    ws: WebSocketUpgrade,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if auth::validate_token(&token_from(&params)).is_err() {
        return unauthorized();
    }
    ws.on_upgrade(move |socket| handle_terminal(socket, name))
}

pub async fn logs(
    ws: WebSocketUpgrade,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if auth::validate_token(&token_from(&params)).is_err() {
        return unauthorized();
    }
    ws.on_upgrade(move |socket| handle_logs(socket, name))
}

async fn handle_terminal(mut socket: WebSocket, name: String) {
    let client = DockerClient::new();

    let cmd = vec![
        "/bin/sh".to_string(),
        "-c".to_string(),
        "if command -v bash > /dev/null 2>&1; then bash; else sh; fi".to_string(),
    ];
    let exec_id = match client.exec_create(&name, &cmd, true, true, true, true).await {
        Ok(id) => id,
        Err(e) => {
            let _ = socket
                .send(Message::Text(format!("Failed to create exec: {}", e)))
                .await;
            return;
        }
    };

    let (io, leftover) = match client.exec_attach(&exec_id, true).await {
        Ok(v) => v,
        Err(e) => {
            let _ = socket
                .send(Message::Text(format!("Failed to attach exec: {}", e)))
                .await;
            return;
        }
    };

    let (mut docker_read, mut docker_write) = tokio::io::split(io);
    if !leftover.is_empty() {
        let _ = socket.send(Message::Binary(leftover)).await;
    }

    // Docker output -> WebSocket
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let forward = tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match docker_read.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    loop {
        tokio::select! {
            Some(data) = rx.recv() => {
                if socket.send(Message::Binary(data)).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(t))) => {
                        if let Ok(m) = serde_json::from_str::<TerminalMessage>(&t) {
                            apply_terminal_message(&client, &exec_id, &mut docker_write, &m).await;
                        } else {
                            let _ = docker_write.write_all(t.as_bytes()).await;
                        }
                    }
                    Some(Ok(Message::Binary(b))) => {
                        if let Ok(m) = serde_json::from_slice::<TerminalMessage>(&b) {
                            apply_terminal_message(&client, &exec_id, &mut docker_write, &m).await;
                        } else {
                            let _ = docker_write.write_all(&b).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {} // Ping / Pong are handled internally
                }
            }
        }
    }
    forward.abort();
}

async fn apply_terminal_message<W>(
    client: &DockerClient,
    exec_id: &str,
    writer: &mut W,
    m: &TerminalMessage,
) where
    W: tokio::io::AsyncWrite + Unpin,
{
    match m.typ.as_str() {
        "input" => {
            let _ = writer.write_all(m.data.as_bytes()).await;
        }
        "resize" => {
            let _ = client.resize_tty(exec_id, m.rows, m.cols).await;
        }
        _ => {}
    }
}

async fn handle_logs(mut socket: WebSocket, name: String) {
    let client = DockerClient::new();
    let mut reader = match client.stream_logs(&name).await {
        Ok(r) => r,
        Err(e) => {
            let _ = socket.send(Message::Text(format!("Error: {}", e))).await;
            return;
        }
    };

    loop {
        tokio::select! {
            chunk = reader.next_chunk() => {
                match chunk {
                    Ok(Some(data)) => {
                        // Go strips the first 8 bytes of every read chunk
                        // (docker multiplexed stream header).
                        let payload: &[u8] = if data.len() > 8 { &data[8..] } else { &data[..] };
                        let text = String::from_utf8_lossy(payload).to_string();
                        if socket.send(Message::Text(text)).await.is_err() {
                            return;
                        }
                    }
                    _ => return,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                    _ => {}
                }
            }
        }
    }
}
