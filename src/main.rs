mod auth;
mod compose;
mod config;
mod db;
mod docker;
mod docker_api;
mod error;
mod filebrowser;
mod handler;
mod handlers;
mod handlers_containers;
mod jwt;
mod parser;
mod requests;
mod scheduler;
mod sse;
mod web;
mod ws;

use crate::compose::Manager;
use crate::config::Config;
use crate::db::Db;
use crate::handler::AppState;
use crate::scheduler::Scheduler;
use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Mirrors Go's `flag` package behaviour used by the original `main.go`:
/// `-flag value`, `-flag=value`, `--flag value`; parsing stops at the first
/// non-flag argument or at `--`; `-h`/`-help` prints usage and exits 0;
/// unknown flags, missing values and bad integers print usage and exit 2.
fn usage() -> String {
    let prog = std::env::args().next().unwrap_or_else(|| "dockops".to_string());
    format!(
        "Usage of {}:\n  -dir string\n    \tdata directory (default: ./data next to binary)\n  -http int\n    \tHTTP port (default 9080)\n  -https int\n    \tHTTPS port (default 9443, only if certs exist)\n",
        prog
    )
}

fn flag_error(msg: &str) -> ! {
    eprintln!("{}", msg);
    eprint!("{}", usage());
    std::process::exit(2);
}

fn parse_args() -> (i64, i64, String) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut http = 0i64;
    let mut https = 0i64;
    let mut dir = String::new();
    let mut i = 0usize;
    while i < args.len() {
        let raw = &args[i];
        // Go's flag package stops at the first non-flag argument and at "--".
        if raw == "--" || raw == "-" || !raw.starts_with('-') {
            break;
        }
        let body = raw.trim_start_matches('-');
        let (key, inline) = match body.split_once('=') {
            Some((k, v)) => (k.to_string(), Some(v.to_string())),
            None => (body.to_string(), None),
        };
        match key.as_str() {
            "h" | "help" => {
                eprint!("{}", usage());
                std::process::exit(0);
            }
            "http" | "https" | "dir" => {
                let value = match inline {
                    Some(v) => v,
                    None => {
                        i += 1;
                        match args.get(i) {
                            Some(v) => v.clone(),
                            None => flag_error(&format!("flag needs an argument: -{}", key)),
                        }
                    }
                };
                match key.as_str() {
                    "http" | "https" => {
                        let n = value.parse::<i64>().unwrap_or_else(|_| {
                            flag_error(&format!(
                                "invalid value {:?} for flag -{}: parse error",
                                value, key
                            ))
                        });
                        if key == "http" {
                            http = n;
                        } else {
                            https = n;
                        }
                    }
                    _ => dir = value,
                }
            }
            _ => flag_error(&format!("flag provided but not defined: -{}", key)),
        }
        i += 1;
    }
    (http, https, dir)
}

#[tokio::main]
async fn main() {
    // axum-server is built without a default crypto provider; install ring.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let (http_port, https_port, data_dir) = parse_args();
    let mut cfg = Config::new(http_port, https_port, data_dir);
    if let Err(e) = cfg.init() {
        log_info!("Failed to initialize: {}", e);
        std::process::exit(1);
    }

    let db = match Db::init(&cfg.data_path) {
        Ok(d) => d,
        Err(e) => {
            log_info!("Failed to init database: {}", e);
            std::process::exit(1);
        }
    };

    let sched = Scheduler::new(db.clone());
    sched.start();

    log_info!(
        "DockOps starting — HTTP :{}  data: {}",
        cfg.http_port,
        cfg.data_path
    );
    if !cfg.cert_path.is_empty() {
        log_info!("HTTPS enabled — :{}", cfg.https_port);
    }

    let state = Arc::new(AppState {
        _cfg: cfg.clone(),
        db,
        sched,
        compose: Manager::new(&cfg.data_path),
        version: env!("DOCKOPS_VERSION").to_string(),
    });
    let app = handler::router(state);

    let http_addr = SocketAddr::from(([0, 0, 0, 0], cfg.http_port));
    let listener = match TcpListener::bind(http_addr).await {
        Ok(l) => l,
        Err(e) => {
            log_info!("Server error: {}", e);
            std::process::exit(1);
        }
    };
    let http_app = app.clone();
    let http_handle = tokio::spawn(async move { axum::serve(listener, http_app).await });

    let https_handle = if !cfg.cert_path.is_empty() && !cfg.key_path.is_empty() {
        match RustlsConfig::from_pem_file(&cfg.cert_path, &cfg.key_path).await {
            Ok(tls) => {
                let addr = SocketAddr::from(([0, 0, 0, 0], cfg.https_port));
                let https_app = app.clone();
                Some(tokio::spawn(async move {
                    axum_server::bind_rustls(addr, tls)
                        .serve(https_app.into_make_service())
                        .await
                }))
            }
            Err(e) => {
                log_info!("Server error: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Go's Run() returns as soon as either listener stops (errCh).
    match https_handle {
        Some(h) => {
            tokio::select! {
                r = http_handle => {
                    match r {
                        Ok(Err(e)) => log_info!("Server error: {}", e),
                        Err(e) => log_info!("Server error: {}", e),
                        _ => {}
                    }
                }
                r = h => {
                    match r {
                        Ok(Err(e)) => log_info!("Server error: {}", e),
                        Err(e) => log_info!("Server error: {}", e),
                        _ => {}
                    }
                }
            }
        }
        None => {
            if let Ok(Err(e)) = http_handle.await {
                log_info!("Server error: {}", e);
            }
        }
    }
}
