//! Compose file storage + container lifecycle, mirroring
//! internal/compose/manager.go. All runtime state comes from Docker directly.

use crate::error::Result;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

pub struct Manager {
    data_path: String,
}

impl Manager {
    pub fn new(data_path: &str) -> Manager {
        Manager {
            data_path: data_path.to_string(),
        }
    }

    fn compose_dir(&self, name: &str) -> PathBuf {
        PathBuf::from(&self.data_path).join("compose").join(name)
    }

    fn compose_path(&self, name: &str) -> PathBuf {
        self.compose_dir(name).join("docker-compose.yml")
    }

    pub fn has_compose_file(&self, name: &str) -> bool {
        self.compose_path(name).exists()
    }

    pub fn get_compose_dir(&self, name: &str) -> String {
        self.compose_dir(name).to_string_lossy().to_string()
    }

    pub fn write_compose(&self, name: &str, content: &str) -> Result<()> {
        let dir = self.compose_dir(name);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("failed to create directory: {}", e))?;
        std::fs::write(self.compose_path(name), content).map_err(|e| e.to_string())
    }

    pub fn remove_compose_dir(&self, name: &str) {
        let _ = std::fs::remove_dir_all(self.compose_dir(name));
    }

    /// `docker compose -f <p> up -d --pull missing`
    pub async fn up(&self, name: &str) -> Result<()> {
        let p = self.compose_path(name);
        let out = Command::new("docker")
            .args([
                "compose",
                "-f",
                &p.to_string_lossy(),
                "up",
                "-d",
                "--pull",
                "missing",
            ])
            .current_dir(self.compose_dir(name))
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            let mut combined = String::new();
            combined.push_str(&String::from_utf8_lossy(&out.stdout));
            combined.push_str(&String::from_utf8_lossy(&out.stderr));
            return Err(format!("compose up failed: {}: {}", combined, out.status));
        }
        Ok(())
    }

    /// `docker compose -f <p> up -d --pull always`, streaming output lines.
    pub async fn up_stream(&self, name: &str, tx: mpsc::Sender<String>) -> Result<()> {
        let p = self.compose_path(name);
        let mut child = Command::new("docker")
            .args([
                "compose",
                "-f",
                &p.to_string_lossy(),
                "up",
                "-d",
                "--pull",
                "always",
            ])
            .current_dir(self.compose_dir(name))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        // ChildStdout and ChildStderr are different types; erase them behind a
        // trait object so both can be read line-by-line into one channel.
        let mut streams: Vec<Box<dyn tokio::io::AsyncRead + Unpin + Send>> = Vec::new();
        if let Some(o) = stdout {
            streams.push(Box::new(o));
        }
        if let Some(e) = stderr {
            streams.push(Box::new(e));
        }
        let mut tasks = Vec::new();
        for stream in streams {
            let tx = tx.clone();
            tasks.push(tokio::spawn(async move {
                let mut lines = BufReader::new(stream).lines();
                let mut collected = Vec::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    if line.trim().is_empty() {
                        continue;
                    }
                    collected.push(line.clone());
                    let _ = tx.send(line).await;
                }
                collected
            }));
        }
        drop(tx);

        let status = child.wait().await.map_err(|e| e.to_string())?;
        let mut all = Vec::new();
        for t in tasks {
            if let Ok(mut lines) = t.await {
                all.append(&mut lines);
            }
        }
        if !status.success() {
            return Err(format!(
                "compose up failed: {}: {}",
                all.join("\n"),
                status
            ));
        }
        Ok(())
    }

    /// `docker compose -f <p> down`. Missing compose file is not an error;
    /// command errors are swallowed (same as Go).
    pub async fn down(&self, name: &str) -> Result<()> {
        let p = self.compose_path(name);
        if !p.exists() {
            return Ok(());
        }
        let _ = Command::new("docker")
            .args(["compose", "-f", &p.to_string_lossy(), "down"])
            .current_dir(self.compose_dir(name))
            .status()
            .await;
        Ok(())
    }

    /// `docker compose -f <p> pull`
    pub async fn pull(&self, name: &str) -> Result<()> {
        let p = self.compose_path(name);
        let status = Command::new("docker")
            .args(["compose", "-f", &p.to_string_lossy(), "pull"])
            .current_dir(self.compose_dir(name))
            .status()
            .await
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("exit status: {}", status));
        }
        Ok(())
    }
}

/// `docker ps -a --filter name=^/<name>$ --format {{.Names}}`
pub async fn container_exists(name: &str) -> bool {
    let filter = format!("name=^/{}$", name);
    let out = match Command::new("docker")
        .args(["ps", "-a", "--filter", &filter, "--format", "{{.Names}}"])
        .output()
        .await
    {
        Ok(o) => o,
        Err(_) => return false,
    };
    if !out.status.success() {
        return false;
    }
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .any(|n| n == name)
}

/// `docker rm -f <name>` (errors ignored)
pub async fn stop_and_remove(name: &str) {
    let _ = Command::new("docker").args(["rm", "-f", name]).status().await;
}

/// Prefer container_name, otherwise use the first service name.
pub fn extract_name_from_compose(content: &str) -> String {
    let mut first_service = String::new();
    let mut in_services = false;
    for line in content.split('\n') {
        let trimmed = line.trim();
        if trimmed == "services:" {
            in_services = true;
            continue;
        }
        if !in_services {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("container_name:") {
            let val = rest.trim().trim_matches(|c| c == '"' || c == '\'');
            if !val.is_empty() {
                return val.to_string();
            }
        }
        if first_service.is_empty()
            && trimmed.ends_with(':')
            && !trimmed.contains(' ')
            && line.len() - line.trim_start_matches(|c| c == ' ' || c == '\t').len() > 0
        {
            first_service = trimmed.trim_end_matches(':').to_string();
        }
    }
    first_service
}

#[allow(dead_code)]
/// Structured data for the create/edit form (Go's compose.FormFields).
#[allow(dead_code)]
#[derive(Default)]
pub struct FormFields {
    pub image: String,
    pub restart: String,
    pub hostname: String,
    pub privileged: bool,
    pub ports: Vec<String>,
    pub volumes: Vec<String>,
    pub env: Vec<String>,
    pub command: String,
    pub entrypoint: String,
    pub user: String,
    pub network_mode: String,
}

fn filter_empty(ss: &[String]) -> Vec<&str> {
    ss.iter()
        .map(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
        .collect()
}

/// Go's BuildComposeFromFields: generate docker-compose.yml from form fields.
#[allow(dead_code)]
pub fn build_compose_from_fields(name: &str, f: &FormFields) -> String {
    let svc_name = if name.is_empty() { "app" } else { name };
    let mut y = format!(
        "version: '3.8'\n\nservices:\n  {}:\n    image: {}\n    container_name: {}\n",
        svc_name, f.image, svc_name
    );
    if !f.restart.is_empty() {
        y.push_str(&format!("    restart: {}\n", f.restart));
    }
    if !f.hostname.is_empty() {
        y.push_str(&format!("    hostname: {}\n", f.hostname));
    }
    if f.privileged {
        y.push_str("    privileged: true\n");
    }
    if !f.network_mode.is_empty() {
        y.push_str(&format!("    network_mode: {}\n", f.network_mode));
    }
    if !f.command.is_empty() {
        y.push_str(&format!("    command: {}\n", f.command));
    }
    if !f.entrypoint.is_empty() {
        y.push_str(&format!("    entrypoint: {}\n", f.entrypoint));
    }
    if !f.user.is_empty() {
        y.push_str(&format!("    user: \"{}\"\n", f.user));
    }
    let ports = filter_empty(&f.ports);
    if !ports.is_empty() {
        y.push_str("    ports:\n");
        for p in ports {
            y.push_str(&format!("      - \"{}\"\n", p));
        }
    }
    let vols = filter_empty(&f.volumes);
    if !vols.is_empty() {
        y.push_str("    volumes:\n");
        for v in vols {
            y.push_str(&format!("      - {}\n", v));
        }
    }
    let envs = filter_empty(&f.env);
    if !envs.is_empty() {
        y.push_str("    environment:\n");
        for e in envs {
            y.push_str(&format!("      - {}\n", e));
        }
    }
    y
}

#[allow(dead_code)]
pub fn extract_image_from_compose(content: &str) -> Vec<String> {
    let mut images = Vec::new();
    for line in content.split('\n') {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("image:") {
            let img = rest.trim().trim_matches(|c| c == '"' || c == '\'');
            if !img.is_empty() {
                images.push(img.to_string());
            }
        }
    }
    images
}
