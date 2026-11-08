use axum::body::Body;
use axum::http::{header, StatusCode};
use axum::response::Response;
use rust_embed::RustEmbed;

/// Frontend build output is embedded into the binary at compile time,
/// equivalent to Go's `//go:embed all:web/dist`.
#[derive(RustEmbed)]
#[folder = "web/dist"]
pub struct Assets;

/// Go's NoRoute decides whether a path is served as a real asset or falls
/// back to index.html (SPA routing).
pub fn is_asset_path(path: &str) -> bool {
    if path.starts_with("/assets/") {
        return true;
    }
    for ext in [
        ".js", ".css", ".png", ".ico", ".svg", ".woff2",
    ] {
        if path.ends_with(ext) {
            return true;
        }
    }
    false
}

pub fn is_api_or_ws(path: &str) -> bool {
    path.starts_with("/api") || path.starts_with("/ws")
}

fn mime_for(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".html") || lower.ends_with(".htm") {
        "text/html; charset=utf-8"
    } else if lower.ends_with(".js") || lower.ends_with(".mjs") {
        "text/javascript; charset=utf-8"
    } else if lower.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if lower.ends_with(".json") || lower.ends_with(".map") {
        "application/json"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".ico") {
        "image/x-icon"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".woff2") {
        "font/woff2"
    } else if lower.ends_with(".woff") {
        "font/woff"
    } else if lower.ends_with(".ttf") {
        "font/ttf"
    } else if lower.ends_with(".txt") {
        "text/plain; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

fn file_response(path: &str) -> Option<Response> {
    let key = path.trim_start_matches('/');
    let file = Assets::get(key)?;
    let data: Vec<u8> = file.data.into_owned();
    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime_for(key))
            .header(header::CONTENT_LENGTH, data.len())
            .body(Body::from(data))
            .unwrap(),
    )
}

fn not_found() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from("404 page not found\n"))
        .unwrap()
}

/// Serve an embedded asset, or fall back to index.html like Go's
/// `c.Request.URL.Path = "/"`.
pub fn serve(path: &str) -> Response {
    if !is_api_or_ws(path) {
        if is_asset_path(path) {
            return file_response(path).unwrap_or_else(not_found);
        }
        if let Some(resp) = file_response("/index.html") {
            return resp;
        }
    }
    not_found()
}
