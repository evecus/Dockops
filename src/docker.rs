//! Low-level Docker Engine API client over a Unix socket (or plain TCP),
//! equivalent to github.com/docker/docker/client `FromEnv` +
//! `WithAPIVersionNegotiation`. Implemented directly on tokio so the musl
//! build has no C/HTTP-stack dependencies.

use crate::error::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};
use tokio::sync::OnceCell;

pub trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> Io for T where T: AsyncRead + AsyncWrite + Unpin + Send {}
pub type BoxIo = Box<dyn Io>;

#[derive(Clone, Debug)]
pub enum Endpoint {
    Unix(String),
    Tcp(String),
}

impl Endpoint {
    fn from_env() -> Endpoint {
        let host = std::env::var("DOCKER_HOST").unwrap_or_default();
        if host.is_empty() {
            return Endpoint::Unix("/var/run/docker.sock".to_string());
        }
        if let Some(p) = host.strip_prefix("unix://") {
            return Endpoint::Unix(p.to_string());
        }
        if host.starts_with("unix:") {
            return Endpoint::Unix(host.trim_start_matches("unix:").to_string());
        }
        let addr = host
            .strip_prefix("tcp://")
            .or_else(|| host.strip_prefix("http://"))
            .or_else(|| host.strip_prefix("https://"))
            .unwrap_or(&host);
        Endpoint::Tcp(addr.to_string())
    }

    fn display(&self) -> String {
        match self {
            Endpoint::Unix(p) => format!("unix://{}", p),
            Endpoint::Tcp(a) => format!("tcp://{}", a),
        }
    }
}

/// Docker SDK's `FromEnv` enables TLS when DOCKER_TLS_VERIFY is set and
/// DOCKER_CERT_PATH contains ca.pem / cert.pem / key.pem.
fn docker_tls_requested() -> bool {
    let verify = std::env::var("DOCKER_TLS_VERIFY").unwrap_or_default();
    !verify.is_empty()
        && verify != "0"
        && !std::env::var("DOCKER_CERT_PATH").unwrap_or_default().is_empty()
}

fn tls_client_config() -> io::Result<std::sync::Arc<rustls::ClientConfig>> {
    let dir = std::path::PathBuf::from(std::env::var("DOCKER_CERT_PATH").unwrap_or_default());
    fn invalid<E: std::fmt::Display>(e: E) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, e.to_string())
    }

    let ca_data = std::fs::read(dir.join("ca.pem"))?;
    let mut roots = rustls::RootCertStore::empty();
    let mut ca_reader = std::io::BufReader::new(&ca_data[..]);
    for cert in rustls_pemfile::certs(&mut ca_reader) {
        let cert = cert.map_err(invalid)?;
        roots.add(cert).map_err(invalid)?;
    }

    let cert_data = std::fs::read(dir.join("cert.pem"))?;
    let key_data = std::fs::read(dir.join("key.pem"))?;
    let mut cert_reader = std::io::BufReader::new(&cert_data[..]);
    let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(invalid)?;
    let mut key_reader = std::io::BufReader::new(&key_data[..]);
    let key = rustls_pemfile::private_key(&mut key_reader)
        .map_err(invalid)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no private key in key.pem"))?;

    let cfg = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(certs, key)
        .map_err(invalid)?;
    Ok(std::sync::Arc::new(cfg))
}

async fn connect(ep: &Endpoint) -> io::Result<BoxIo> {
    match ep {
        Endpoint::Unix(p) => Ok(Box::new(UnixStream::connect(p).await?)),
        Endpoint::Tcp(a) => {
            let stream = TcpStream::connect(a).await?;
            if docker_tls_requested() {
                let cfg = tls_client_config()?;
                let host = a.split(':').next().unwrap_or("localhost").to_string();
                let name = rustls::pki_types::ServerName::try_from(host)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
                let conn = tokio_rustls::TlsConnector::from(cfg)
                    .connect(name, stream)
                    .await?;
                Ok(Box::new(conn))
            } else {
                Ok(Box::new(stream))
            }
        }
    }
}

static API_VERSION: OnceCell<String> = OnceCell::const_new();

/// Negotiate the API version once with GET /_ping (header `Api-Version`),
/// honouring DOCKER_API_VERSION when set. Falls back to a widely supported
/// default if the daemon cannot be reached (the real request then reports the
/// connection error, just like the Go SDK).
async fn negotiated_version(ep: &Endpoint) -> String {
    let ep = ep.clone();
    API_VERSION
        .get_or_init(|| async move {
            if let Ok(v) = std::env::var("DOCKER_API_VERSION") {
                if !v.is_empty() {
                    return v;
                }
            }
            match ping(&ep).await {
                Some(v) => v,
                None => "1.41".to_string(),
            }
        })
        .await
        .clone()
}

async fn ping(ep: &Endpoint) -> Option<String> {
    let mut io = connect(ep).await.ok()?;
    let req = "GET /_ping HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n";
    io.write_all(req.as_bytes()).await.ok()?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    loop {
        let n = io.read(&mut tmp).await.ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if find_sub(&buf, b"\r\n\r\n").is_some() {
            break;
        }
        if buf.len() > 65536 {
            break;
        }
    }
    let head = String::from_utf8_lossy(&buf).to_string();
    for line in head.lines().skip(1) {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("api-version") {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn find_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[derive(Clone, Copy)]
enum BodyMode {
    Length(u64),
    Chunked { remaining: u64, need_crlf: bool },
    Eof,
}

/// Reads an HTTP response body, transparently decoding chunked transfer
/// encoding and exposing raw chunks for streaming endpoints.
pub struct BodyReader {
    io: Option<BoxIo>,
    leftover: Vec<u8>,
    mode: BodyMode,
    done: bool,
}

impl BodyReader {
    fn new(io: BoxIo, leftover: Vec<u8>, mode: BodyMode) -> Self {
        BodyReader {
            io: Some(io),
            leftover,
            mode,
            done: false,
        }
    }

    async fn fill(&mut self, max: usize) -> io::Result<Vec<u8>> {
        if !self.leftover.is_empty() {
            let n = max.min(self.leftover.len());
            let chunk: Vec<u8> = self.leftover.drain(..n).collect();
            return Ok(chunk);
        }
        let mut buf = vec![0u8; max];
        let io = self
            .io
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "stream closed"))?;
        let n = io.read(&mut buf).await?;
        buf.truncate(n);
        Ok(buf)
    }

    async fn read_byte(&mut self) -> io::Result<Option<u8>> {
        if !self.leftover.is_empty() {
            return Ok(Some(self.leftover.remove(0)));
        }
        let mut b = [0u8; 1];
        let io = self
            .io
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "stream closed"))?;
        let n = io.read(&mut b).await?;
        if n == 0 {
            Ok(None)
        } else {
            Ok(Some(b[0]))
        }
    }

    async fn read_line(&mut self) -> io::Result<String> {
        let mut v = Vec::new();
        while let Some(b) = self.read_byte().await? {
            if b == b'\n' {
                break;
            }
            v.push(b);
        }
        Ok(String::from_utf8_lossy(&v)
            .trim_end_matches('\r')
            .to_string())
    }

    pub async fn next_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        if self.done {
            return Ok(None);
        }
        loop {
            match self.mode {
                BodyMode::Length(n) => {
                    if n == 0 {
                        self.done = true;
                        return Ok(None);
                    }
                    let buf = self.fill(n.min(4096) as usize).await?;
                    if buf.is_empty() {
                        self.done = true;
                        return Ok(None);
                    }
                    self.mode = BodyMode::Length(n - buf.len() as u64);
                    return Ok(Some(buf));
                }
                BodyMode::Eof => {
                    let buf = self.fill(4096).await?;
                    if buf.is_empty() {
                        self.done = true;
                        return Ok(None);
                    }
                    return Ok(Some(buf));
                }
                BodyMode::Chunked {
                    remaining,
                    need_crlf,
                } => {
                    if need_crlf {
                        // consume the CRLF that terminates a chunk
                        if self.read_byte().await?.is_none()
                            || self.read_byte().await?.is_none()
                        {
                            self.done = true;
                            return Ok(None);
                        }
                        self.mode = BodyMode::Chunked {
                            remaining: 0,
                            need_crlf: false,
                        };
                        continue;
                    }
                    if remaining == 0 {
                        let line = self.read_line().await?;
                        let size_str = line.split(';').next().unwrap_or("").trim();
                        let size = u64::from_str_radix(size_str, 16).unwrap_or(0);
                        if size == 0 {
                            // consume trailers up to the empty line
                            loop {
                                if self.read_line().await?.is_empty() {
                                    break;
                                }
                            }
                            self.done = true;
                            return Ok(None);
                        }
                        self.mode = BodyMode::Chunked {
                            remaining: size,
                            need_crlf: false,
                        };
                        continue;
                    }
                    let n = remaining.min(4096) as usize;
                    let buf = self.fill(n).await?;
                    if buf.is_empty() {
                        self.done = true;
                        return Ok(None);
                    }
                    let left = remaining - buf.len() as u64;
                    self.mode = BodyMode::Chunked {
                        remaining: left,
                        need_crlf: left == 0,
                    };
                    return Ok(Some(buf));
                }
            }
        }
    }

    pub async fn read_all(&mut self) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();
        while let Some(chunk) = self.next_chunk().await? {
            out.extend_from_slice(&chunk);
        }
        Ok(out)
    }

    /// Take over the raw connection (used for exec attach / terminal hijack).
    pub fn into_io(self) -> (Option<BoxIo>, Vec<u8>) {
        (self.io, self.leftover)
    }
}

pub struct HttpResponse {
    pub status: u16,
    #[allow(dead_code)]
    pub headers: HashMap<String, String>,
    pub body: BodyReader,
}

pub struct DockerClient {
    endpoint: Endpoint,
}

impl DockerClient {
    pub fn new() -> DockerClient {
        DockerClient {
            endpoint: Endpoint::from_env(),
        }
    }

    async fn version(&self) -> String {
        negotiated_version(&self.endpoint).await
    }

    /// Send one request. `api_path` is the Docker path *without* the /vX.Y
    /// prefix, e.g. `/containers/json?all=1`.
    pub async fn call(
        &self,
        method: &str,
        api_path: &str,
        body: Vec<u8>,
        headers: &[(&str, &str)],
        hijack: bool,
    ) -> Result<HttpResponse> {
        let version = self.version().await;
        let full_path = format!("/v{}{}", version, api_path);

        let mut req = format!("{} {} HTTP/1.1\r\n", method, full_path);
        req.push_str("Host: docker\r\n");
        req.push_str("User-Agent: DockOps/1.0\r\n");
        if hijack {
            req.push_str("Connection: Upgrade\r\nUpgrade: tcp\r\n");
        } else {
            req.push_str("Connection: close\r\n");
        }
        let has_ct = headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"));
        if !has_ct {
            req.push_str("Content-Type: application/json\r\n");
        }
        req.push_str(&format!("Content-Length: {}\r\n", body.len()));
        for (k, v) in headers {
            req.push_str(k);
            req.push_str(": ");
            req.push_str(v);
            req.push_str("\r\n");
        }
        req.push_str("\r\n");

        let mut io = connect(&self.endpoint).await.map_err(|e| {
            format!(
                "Cannot connect to the Docker daemon at {}. Is the docker daemon running? ({})",
                self.endpoint.display(),
                e
            )
        })?;
        io.write_all(req.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        if !body.is_empty() {
            io.write_all(&body).await.map_err(|e| e.to_string())?;
        }
        io.flush().await.map_err(|e| e.to_string())?;

        let mut buf: Vec<u8> = Vec::new();
        let head_end;
        loop {
            if let Some(pos) = find_sub(&buf, b"\r\n\r\n") {
                head_end = pos;
                break;
            }
            let mut tmp = [0u8; 4096];
            let n = io.read(&mut tmp).await.map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("connection closed before response headers".to_string());
            }
            buf.extend_from_slice(&tmp[..n]);
            if buf.len() > 65536 {
                return Err("response headers too large".to_string());
            }
        }
        let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
        let leftover = buf[head_end + 4..].to_vec();

        let status = head
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);

        let mut hmap = HashMap::new();
        for line in head.lines().skip(1) {
            if let Some((k, v)) = line.split_once(':') {
                hmap.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
            }
        }

        let mode = if status == 204 || status == 304 {
            BodyMode::Length(0)
        } else if hmap
            .get("transfer-encoding")
            .map(|v| v.to_ascii_lowercase().contains("chunked"))
            .unwrap_or(false)
        {
            BodyMode::Chunked {
                remaining: 0,
                need_crlf: false,
            }
        } else if let Some(cl) = hmap
            .get("content-length")
            .and_then(|v| v.parse::<u64>().ok())
        {
            BodyMode::Length(cl)
        } else {
            BodyMode::Eof
        };

        Ok(HttpResponse {
            status,
            headers: hmap,
            body: BodyReader::new(io, leftover, mode),
        })
    }

    /// Request + decode a JSON response.
    pub async fn call_json(
        &self,
        method: &str,
        path: &str,
        body: Vec<u8>,
        headers: &[(&str, &str)],
    ) -> Result<Value> {
        let mut resp = self.call(method, path, body, headers, false).await?;
        let data = resp.body.read_all().await.map_err(|e| e.to_string())?;
        check_status(resp.status, &data)?;
        if data.is_empty() {
            return Ok(Value::Null);
        }
        Ok(serde_json::from_slice(&data).unwrap_or(Value::Null))
    }

    /// Request that only needs a successful status code.
    pub async fn call_ok(
        &self,
        method: &str,
        path: &str,
        body: Vec<u8>,
        headers: &[(&str, &str)],
    ) -> Result<()> {
        let mut resp = self.call(method, path, body, headers, false).await?;
        let data = resp.body.read_all().await.map_err(|e| e.to_string())?;
        check_status(resp.status, &data)
    }

    /// Request returning a streaming body (SSE / logs / tar / pull progress).
    pub async fn call_stream(
        &self,
        method: &str,
        path: &str,
        body: Vec<u8>,
        headers: &[(&str, &str)],
    ) -> Result<BodyReader> {
        let mut resp = self.call(method, path, body, headers, false).await?;
        if !(200..300).contains(&resp.status) {
            let data = resp.body.read_all().await.map_err(|e| e.to_string())?;
            return Err(check_status_err(resp.status, &data));
        }
        Ok(resp.body)
    }

    /// Exec-attach style hijacked connection (101 Switching Protocols).
    pub async fn call_hijack(
        &self,
        method: &str,
        path: &str,
        body: Vec<u8>,
        headers: &[(&str, &str)],
    ) -> Result<HttpResponse> {
        self.call(method, path, body, headers, true).await
    }
}

pub fn check_status(status: u16, data: &[u8]) -> Result<()> {
    if (200..300).contains(&status) || status == 101 {
        Ok(())
    } else {
        Err(check_status_err(status, data))
    }
}

pub fn check_status_err(status: u16, data: &[u8]) -> String {
    let mut msg = if let Ok(v) = serde_json::from_slice::<Value>(data) {
        v.get("message")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| String::from_utf8_lossy(data).trim().to_string())
    } else {
        String::from_utf8_lossy(data).trim().to_string()
    };
    if msg.is_empty() {
        msg = format!("HTTP status {}", status);
    }
    format!("Error response from daemon: {}", msg)
}

/// Strip the Docker multiplexed-stream 8-byte frame headers (see
/// stripDockerHeaders in the Go handler). If nothing was decoded, return the
/// original bytes.
pub fn strip_docker_headers(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        if i + 8 > data.len() {
            break;
        }
        let size = u32::from_be_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]])
            as usize;
        i += 8;
        if i + size > data.len() {
            result.extend_from_slice(&data[i..]);
            break;
        }
        result.extend_from_slice(&data[i..i + size]);
        i += size;
    }
    if result.is_empty() {
        data.to_vec()
    } else {
        result
    }
}

/// Decode exec output frames (stdout/stderr) into a single string, matching
/// the Go `runExec` helper used by the file browser.
pub fn decode_exec_output(data: &[u8]) -> String {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 8 <= data.len() {
        let size = u32::from_be_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]])
            as usize;
        i += 8;
        if size == 0 {
            continue;
        }
        if i + size > data.len() {
            break;
        }
        out.extend_from_slice(&data[i..i + size]);
        i += size;
    }
    String::from_utf8_lossy(&out).to_string()
}
