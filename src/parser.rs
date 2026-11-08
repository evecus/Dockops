//! docker run -> compose parser, ported from internal/parser/parser.go.

use crate::error::Result;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct ComposeService {
    pub image: String,
    pub container_name: String,
    pub hostname: String,
    pub restart: String,
    pub ports: Vec<String>,
    pub volumes: Vec<String>,
    pub environment: Vec<String>,
    pub networks: Vec<String>,
    pub command: String,
    pub entrypoint: String,
    pub user: String,
    pub work_dir: String,
    pub privileged: bool,
    pub network_mode: String,
    pub dns: Vec<String>,
    pub dns_search: Vec<String>,
    pub extra_hosts: Vec<String>,
    pub cap_add: Vec<String>,
    pub cap_drop: Vec<String>,
    pub devices: Vec<String>,
    pub labels: BTreeMap<String, String>,
    pub mem_limit: String,
    pub cpus: String,
    pub pid_mode: String,
    pub shm_size: String,
    pub ulimits: BTreeMap<String, String>,
    pub sysctls: BTreeMap<String, String>,
    pub security_opt: Vec<String>,
    pub init: bool,
    pub read_only: bool,
    pub stop_signal: String,
    pub log_driver: String,
    pub log_options: BTreeMap<String, String>,
    pub env_file: Vec<String>,
    pub tmpfs: Vec<String>,
    pub group_add: Vec<String>,
    pub runtime: String,
    pub platform: String,
    pub ip_address: String,
    pub mac_address: String,
    pub volumes_from: Vec<String>,
}

impl ComposeService {
    /// JSON representation with Go `omitempty` semantics.
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        macro_rules! put_str {
            ($k:expr, $v:expr) => {
                if !$v.is_empty() {
                    m.insert($k.to_string(), json!($v));
                }
            };
        }
        macro_rules! put_list {
            ($k:expr, $v:expr) => {
                if !$v.is_empty() {
                    m.insert($k.to_string(), json!($v));
                }
            };
        }
        macro_rules! put_map {
            ($k:expr, $v:expr) => {
                if !$v.is_empty() {
                    m.insert($k.to_string(), json!($v));
                }
            };
        }
        put_str!("image", self.image);
        put_str!("container_name", self.container_name);
        put_str!("hostname", self.hostname);
        put_str!("restart", self.restart);
        put_list!("ports", self.ports);
        put_list!("volumes", self.volumes);
        put_list!("environment", self.environment);
        put_list!("networks", self.networks);
        put_str!("command", self.command);
        put_str!("entrypoint", self.entrypoint);
        put_str!("user", self.user);
        put_str!("working_dir", self.work_dir);
        if self.privileged {
            m.insert("privileged".into(), json!(true));
        }
        put_str!("network_mode", self.network_mode);
        put_list!("dns", self.dns);
        put_list!("dns_search", self.dns_search);
        put_list!("extra_hosts", self.extra_hosts);
        put_list!("cap_add", self.cap_add);
        put_list!("cap_drop", self.cap_drop);
        put_list!("devices", self.devices);
        put_map!("labels", self.labels);
        put_str!("mem_limit", self.mem_limit);
        put_str!("cpus", self.cpus);
        put_str!("pid", self.pid_mode);
        put_str!("shm_size", self.shm_size);
        put_map!("ulimits", self.ulimits);
        put_map!("sysctls", self.sysctls);
        put_list!("security_opt", self.security_opt);
        if self.init {
            m.insert("init".into(), json!(true));
        }
        if self.read_only {
            m.insert("read_only".into(), json!(true));
        }
        put_str!("stop_signal", self.stop_signal);
        put_str!("logging_driver", self.log_driver);
        put_map!("logging_options", self.log_options);
        put_list!("env_file", self.env_file);
        put_list!("tmpfs", self.tmpfs);
        put_list!("group_add", self.group_add);
        put_str!("runtime", self.runtime);
        put_str!("platform", self.platform);
        put_str!("ip", self.ip_address);
        put_str!("mac_address", self.mac_address);
        put_list!("volumes_from", self.volumes_from);
        Value::Object(m)
    }

    pub fn to_yaml(&self) -> String {
        let name = if self.container_name.is_empty() {
            let last = self.image.rsplit('/').next().unwrap_or(&self.image);
            last.split(':').next().unwrap_or(last).to_string()
        } else {
            self.container_name.clone()
        };

        let mut y = String::new();
        y.push_str("version: '3.8'\n\nservices:\n");
        y.push_str(&format!("  {}:\n", name));
        y.push_str(&format!("    image: {}\n", self.image));
        if !self.container_name.is_empty() {
            y.push_str(&format!("    container_name: {}\n", self.container_name));
        }
        if !self.hostname.is_empty() {
            y.push_str(&format!("    hostname: {}\n", self.hostname));
        }
        if !self.restart.is_empty() {
            y.push_str(&format!("    restart: {}\n", self.restart));
        }
        if self.privileged {
            y.push_str("    privileged: true\n");
        }
        if !self.network_mode.is_empty() {
            y.push_str(&format!("    network_mode: {}\n", self.network_mode));
        }
        if !self.user.is_empty() {
            y.push_str(&format!("    user: {}\n", self.user));
        }
        if !self.work_dir.is_empty() {
            y.push_str(&format!("    working_dir: {}\n", self.work_dir));
        }
        if !self.entrypoint.is_empty() {
            y.push_str(&format!("    entrypoint: {}\n", self.entrypoint));
        }
        if !self.command.is_empty() {
            y.push_str(&format!("    command: {}\n", self.command));
        }
        if !self.mem_limit.is_empty() {
            y.push_str(&format!("    mem_limit: {}\n", self.mem_limit));
        }
        if !self.cpus.is_empty() {
            y.push_str(&format!("    cpus: {}\n", self.cpus));
        }
        if !self.shm_size.is_empty() {
            y.push_str(&format!("    shm_size: {}\n", self.shm_size));
        }
        if self.init {
            y.push_str("    init: true\n");
        }
        if self.read_only {
            y.push_str("    read_only: true\n");
        }
        if !self.stop_signal.is_empty() {
            y.push_str(&format!("    stop_signal: {}\n", self.stop_signal));
        }
        if !self.runtime.is_empty() {
            y.push_str(&format!("    runtime: {}\n", self.runtime));
        }
        if !self.platform.is_empty() {
            y.push_str(&format!("    platform: {}\n", self.platform));
        }
        if !self.mac_address.is_empty() {
            y.push_str(&format!("    mac_address: {}\n", self.mac_address));
        }
        if !self.pid_mode.is_empty() {
            y.push_str(&format!("    pid: {}\n", self.pid_mode));
        }

        let mut write_list = |key: &str, items: &Vec<String>| {
            if items.is_empty() {
                return;
            }
            y.push_str(&format!("    {}:\n", key));
            for item in items {
                y.push_str(&format!("      - {}\n", item));
            }
        };
        write_list("ports", &self.ports);
        write_list("volumes", &self.volumes);
        write_list("environment", &self.environment);
        write_list("env_file", &self.env_file);
        write_list("networks", &self.networks);
        write_list("dns", &self.dns);
        write_list("dns_search", &self.dns_search);
        write_list("extra_hosts", &self.extra_hosts);
        write_list("cap_add", &self.cap_add);
        write_list("cap_drop", &self.cap_drop);
        write_list("devices", &self.devices);
        write_list("security_opt", &self.security_opt);
        write_list("tmpfs", &self.tmpfs);
        write_list("group_add", &self.group_add);
        write_list("volumes_from", &self.volumes_from);

        if !self.labels.is_empty() {
            y.push_str("    labels:\n");
            for (k, v) in &self.labels {
                y.push_str(&format!("      {}: {}\n", k, v));
            }
        }
        if !self.sysctls.is_empty() {
            y.push_str("    sysctls:\n");
            for (k, v) in &self.sysctls {
                y.push_str(&format!("      {}: {}\n", k, v));
            }
        }
        if !self.log_driver.is_empty() || !self.log_options.is_empty() {
            y.push_str("    logging:\n");
            if !self.log_driver.is_empty() {
                y.push_str(&format!("      driver: {}\n", self.log_driver));
            }
            if !self.log_options.is_empty() {
                y.push_str("      options:\n");
                for (k, v) in &self.log_options {
                    y.push_str(&format!("        {}: {}\n", k, v));
                }
            }
        }
        if !self.networks.is_empty() {
            y.push_str("\nnetworks:\n");
            for n in &self.networks {
                y.push_str(&format!("  {}:\n    external: true\n", n));
            }
        }
        y
    }
}

pub fn parse_docker_run(cmd: &str) -> Result<ComposeService> {
    let mut cmd = cmd.trim().to_string();
    for prefix in ["docker container run", "docker run"] {
        if cmd.starts_with(prefix) {
            cmd = cmd[prefix.len()..].trim().to_string();
            break;
        }
    }
    let mut tokens = tokenize(&cmd)?;
    let mut svc = ComposeService::default();

    let mut i = 0usize;
    while i < tokens.len() {
        let tok = tokens[i].clone();
        match tok.as_str() {
            "--detach" | "-d" | "--rm" | "--tty" | "-t" | "--interactive" | "-i"
            | "-it" | "-ti" | "-P" | "--publish-all" | "--privileged" | "--init"
            | "--read-only" => {
                match tok.as_str() {
                    "--privileged" => svc.privileged = true,
                    "--init" => svc.init = true,
                    "--read-only" => svc.read_only = true,
                    _ => {}
                }
            }
            "--name" => {
                i += 1;
                if i < tokens.len() {
                    svc.container_name = tokens[i].clone();
                }
            }
            "--hostname" | "-h" => {
                i += 1;
                if i < tokens.len() {
                    svc.hostname = tokens[i].clone();
                }
            }
            "--restart" => {
                i += 1;
                if i < tokens.len() {
                    svc.restart = tokens[i].clone();
                }
            }
            "-p" | "--publish" => {
                i += 1;
                if i < tokens.len() {
                    svc.ports.push(tokens[i].clone());
                }
            }
            "-v" | "--volume" => {
                i += 1;
                if i < tokens.len() {
                    svc.volumes.push(tokens[i].clone());
                }
            }
            "--mount" => {
                i += 1;
                if i < tokens.len() {
                    svc.volumes.push(convert_mount(&tokens[i]));
                }
            }
            "-e" | "--env" => {
                i += 1;
                if i < tokens.len() {
                    svc.environment.push(tokens[i].clone());
                }
            }
            "--env-file" => {
                i += 1;
                if i < tokens.len() {
                    svc.env_file.push(tokens[i].clone());
                }
            }
            "--network" | "--net" => {
                i += 1;
                if i < tokens.len() {
                    let n = tokens[i].clone();
                    if n == "host" || n == "none" || n.starts_with("container:") {
                        svc.network_mode = n;
                    } else {
                        svc.networks.push(n);
                    }
                }
            }
            "--network-alias" | "--cpu-shares" | "--link" | "--expose" => {
                i += 1; // skip value; no direct compose equivalent
            }
            "--ip" => {
                i += 1;
                if i < tokens.len() {
                    svc.ip_address = tokens[i].clone();
                }
            }
            "--mac-address" => {
                i += 1;
                if i < tokens.len() {
                    svc.mac_address = tokens[i].clone();
                }
            }
            "--dns" => {
                i += 1;
                if i < tokens.len() {
                    svc.dns.push(tokens[i].clone());
                }
            }
            "--dns-search" => {
                i += 1;
                if i < tokens.len() {
                    svc.dns_search.push(tokens[i].clone());
                }
            }
            "--add-host" => {
                i += 1;
                if i < tokens.len() {
                    svc.extra_hosts.push(tokens[i].clone());
                }
            }
            "--entrypoint" => {
                i += 1;
                if i < tokens.len() {
                    svc.entrypoint = tokens[i].clone();
                }
            }
            "-u" | "--user" => {
                i += 1;
                if i < tokens.len() {
                    svc.user = tokens[i].clone();
                }
            }
            "-w" | "--workdir" => {
                i += 1;
                if i < tokens.len() {
                    svc.work_dir = tokens[i].clone();
                }
            }
            "--cap-add" => {
                i += 1;
                if i < tokens.len() {
                    svc.cap_add.push(tokens[i].clone());
                }
            }
            "--cap-drop" => {
                i += 1;
                if i < tokens.len() {
                    svc.cap_drop.push(tokens[i].clone());
                }
            }
            "--device" => {
                i += 1;
                if i < tokens.len() {
                    svc.devices.push(tokens[i].clone());
                }
            }
            "-l" | "--label" => {
                i += 1;
                if i < tokens.len() {
                    let parts: Vec<&str> = tokens[i].splitn(2, '=').collect();
                    svc.labels.insert(
                        parts[0].to_string(),
                        parts.get(1).copied().unwrap_or("").to_string(),
                    );
                }
            }
            "-m" | "--memory" => {
                i += 1;
                if i < tokens.len() {
                    svc.mem_limit = tokens[i].clone();
                }
            }
            "--cpus" => {
                i += 1;
                if i < tokens.len() {
                    svc.cpus = tokens[i].clone();
                }
            }
            "--pid" => {
                i += 1;
                if i < tokens.len() {
                    svc.pid_mode = tokens[i].clone();
                }
            }
            "--shm-size" => {
                i += 1;
                if i < tokens.len() {
                    svc.shm_size = tokens[i].clone();
                }
            }
            "--ulimit" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((k, v)) = tokens[i].split_once('=') {
                        svc.ulimits.insert(k.to_string(), v.to_string());
                    }
                }
            }
            "--sysctl" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((k, v)) = tokens[i].split_once('=') {
                        svc.sysctls.insert(k.to_string(), v.to_string());
                    }
                }
            }
            "--security-opt" => {
                i += 1;
                if i < tokens.len() {
                    svc.security_opt.push(tokens[i].clone());
                }
            }
            "--stop-signal" => {
                i += 1;
                if i < tokens.len() {
                    svc.stop_signal = tokens[i].clone();
                }
            }
            "--log-driver" => {
                i += 1;
                if i < tokens.len() {
                    svc.log_driver = tokens[i].clone();
                }
            }
            "--log-opt" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((k, v)) = tokens[i].split_once('=') {
                        svc.log_options.insert(k.to_string(), v.to_string());
                    }
                }
            }
            "--tmpfs" => {
                i += 1;
                if i < tokens.len() {
                    svc.tmpfs.push(tokens[i].clone());
                }
            }
            "--group-add" => {
                i += 1;
                if i < tokens.len() {
                    svc.group_add.push(tokens[i].clone());
                }
            }
            "--runtime" => {
                i += 1;
                if i < tokens.len() {
                    svc.runtime = tokens[i].clone();
                }
            }
            "--platform" => {
                i += 1;
                if i < tokens.len() {
                    svc.platform = tokens[i].clone();
                }
            }
            "--volumes-from" => {
                i += 1;
                if i < tokens.len() {
                    svc.volumes_from.push(tokens[i].clone());
                }
            }
            _ => {
                if tok.starts_with("--") || tok.starts_with("-") {
                    if tok.contains('=') {
                        let parts: Vec<String> =
                            tok.splitn(2, '=').map(|s| s.to_string()).collect();
                        let rest: Vec<String> = tokens.split_off(i + 1);
                        tokens.truncate(i);
                        tokens.push(parts[0].clone());
                        tokens.push(parts[1].clone());
                        tokens.extend(rest);
                        continue;
                    }
                    // Unknown flag without '=' is ignored (no image/command set).
                } else if svc.image.is_empty() {
                    svc.image = tok;
                } else {
                    // NOTE: Go's `break` here exits the switch, not the loop;
                    // subsequent tokens keep being processed. Preserved as-is.
                    svc.command = tokens[i..].join(" ");
                }
            }
        }
        i += 1;
    }

    if svc.image.is_empty() {
        return Err("no image specified in docker run command".to_string());
    }
    if !svc.ports.is_empty() && svc.network_mode.is_empty() {
        svc.network_mode = "bridge".to_string();
    }
    Ok(svc)
}

fn convert_mount(mount_str: &str) -> String {
    let mut source = String::new();
    let mut target = String::new();
    let mut mtype = String::new();
    for p in mount_str.split(',') {
        let kv: Vec<&str> = p.splitn(2, '=').collect();
        if kv.len() != 2 {
            continue;
        }
        match kv[0] {
            "source" | "src" => source = kv[1].to_string(),
            "target" | "dst" | "destination" => target = kv[1].to_string(),
            "type" => mtype = kv[1].to_string(),
            _ => {}
        }
    }
    if mtype == "tmpfs" {
        return target;
    }
    if !source.is_empty() && !target.is_empty() {
        return format!("{}:{}", source, target);
    }
    mount_str.to_string()
}

/// Shell-like tokenization respecting quotes and backslash-newline
/// continuations (byte-for-byte behaviour of the Go implementation).
fn tokenize(cmd: &str) -> Result<Vec<String>> {
    let bytes = cmd.as_bytes();
    let mut tokens = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    let mut in_single = false;
    let mut in_double = false;

    let mut i = 0usize;
    while i < bytes.len() {
        let ch = bytes[i];
        if ch == b'\\' && !in_single && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            i += 1; // skip backslash + newline
        } else if ch == b'\\' && !in_single && !in_double && i + 1 < bytes.len() {
            i += 1;
            let next = bytes[i];
            if next != b'\n' {
                current.push(next);
            }
        } else if ch == b'\'' && !in_double {
            in_single = !in_single;
        } else if ch == b'"' && !in_single {
            in_double = !in_double;
        } else if (ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r')
            && !in_single
            && !in_double
        {
            if !current.is_empty() {
                tokens.push(String::from_utf8_lossy(&current).to_string());
                current.clear();
            }
        } else {
            current.push(ch);
        }
        i += 1;
    }
    if !current.is_empty() {
        tokens.push(String::from_utf8_lossy(&current).to_string());
    }
    if in_single || in_double {
        return Err("unclosed quote in command".to_string());
    }
    Ok(tokens)
}
