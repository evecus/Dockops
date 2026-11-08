//! High-level Docker operations. Each method mirrors the corresponding call in
//! the Go backend (internal/docker/client.go), including JSON field names.

use crate::docker::{check_status_err, url_encode, BodyReader, DockerClient};
use crate::error::Result;
use serde_json::{json, Map, Value};

fn s(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn i(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(|x| x.as_i64()).unwrap_or(0)
}

fn arr(v: &Value) -> &[Value] {
    v.as_array().map(|a| a.as_slice()).unwrap_or_default()
}

fn str_list(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn short_id(id: &str) -> String {
    id.chars().take(12).collect()
}

fn as_list(items: Vec<Value>) -> Value {
    if items.is_empty() {
        Value::Null
    } else {
        Value::Array(items)
    }
}

fn parse_docker_time(v: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(v)
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

fn rfc3339_secs(v: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(v)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|_| "0001-01-01T00:00:00Z".to_string())
}

/// Split "repo:tag" into fromImage/tag, leaving registry ports alone.
fn split_image_ref(reference: &str) -> (String, Option<String>) {
    if let Some(pos) = reference.rfind(':') {
        let tag = &reference[pos + 1..];
        if !tag.is_empty() && !tag.contains('/') {
            return (reference[..pos].to_string(), Some(tag.to_string()));
        }
    }
    (reference.to_string(), None)
}

const REGISTRY_AUTH: (&str, &str) = ("X-Registry-Auth", "e30="); // base64("{}")

impl DockerClient {
    // ===== SYSTEM =====

    pub async fn get_system_info(&self) -> Result<Value> {
        let v = self.call_json("GET", "/info", vec![], &[]).await?;
        Ok(json!({
            "docker_version": s(&v, "ServerVersion"),
            "os": s(&v, "OperatingSystem"),
            "arch": s(&v, "Architecture"),
            "kernel_version": s(&v, "KernelVersion"),
            "total_memory": i(&v, "MemTotal"),
            "cpus": i(&v, "NCPU"),
            "storage_driver": s(&v, "Driver"),
            "logging_driver": s(&v, "LoggingDriver"),
            "docker_root_dir": s(&v, "DockerRootDir"),
            "containers": i(&v, "Containers"),
            "containers_paused": i(&v, "ContainersPaused"),
            "containers_stopped": i(&v, "ContainersStopped"),
            "containers_running": i(&v, "ContainersRunning"),
            "images": i(&v, "Images"),
            "server_time": chrono::Local::now()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        }))
    }

    // ===== CONTAINERS =====

    pub async fn list_containers(&self) -> Result<Vec<Value>> {
        let v = self
            .call_json("GET", "/containers/json?all=1", vec![], &[])
            .await?;
        let mut result = Vec::new();
        for ct in arr(&v) {
            let name = ct
                .get("Names")
                .and_then(|n| n.as_array())
                .and_then(|a| a.first())
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim_start_matches('/')
                .to_string();

            let mut ports = Vec::new();
            for p in arr(&ct["Ports"]) {
                ports.push(json!({
                    "host_ip": s(p, "IP"),
                    "host_port": p.get("PublicPort").and_then(|x| x.as_i64()).unwrap_or(0).to_string(),
                    "container_port": p.get("PrivatePort").and_then(|x| x.as_i64()).unwrap_or(0).to_string(),
                    "protocol": s(p, "Type"),
                }));
            }
            result.push(json!({
                "id": short_id(&s(ct, "Id")),
                "name": name,
                "image": s(ct, "Image"),
                "image_id": s(ct, "ImageID"),
                "status": s(ct, "Status"),
                "state": s(ct, "State"),
                "ports": as_list(ports),
                "created": i(ct, "Created"),
                "labels": ct.get("Labels").cloned().unwrap_or(Value::Null),
            }));
        }
        Ok(result)
    }

    pub async fn inspect_raw(&self, id: &str) -> Result<Value> {
        self.call_json("GET", &format!("/containers/{}/json", id), vec![], &[])
            .await
    }

    /// Returns the Go `ContainerDetail` shape (ContainerSummary embedded).
    pub async fn inspect_container(&self, id: &str) -> Result<Value> {
        let info = self.inspect_raw(id).await?;
        let config = &info["Config"];
        let host = &info["HostConfig"];

        let mut ports = Vec::new();
        if let Some(pb) = host.get("PortBindings").and_then(|p| p.as_object()) {
            for (port, bindings) in pb {
                let parts: Vec<&str> = port.splitn(2, '/').collect();
                let proto = parts.get(1).copied().unwrap_or("tcp");
                for b in arr(bindings) {
                    ports.push(json!({
                        "host_ip": s(b, "HostIp"),
                        "host_port": s(b, "HostPort"),
                        "container_port": parts[0],
                        "protocol": proto,
                    }));
                }
            }
        }

        let mut mounts = Vec::new();
        for m in arr(&info["Mounts"]) {
            mounts.push(json!({
                "type": s(m, "Type"),
                "source": s(m, "Source"),
                "destination": s(m, "Destination"),
                "mode": s(m, "Mode"),
            }));
        }

        let mut networks = Map::new();
        if let Some(nets) = info["NetworkSettings"]["Networks"].as_object() {
            for (k, v) in nets {
                networks.insert(k.clone(), json!(s(v, "IPAddress")));
            }
        }

        Ok(json!({
            "id": short_id(&s(&info, "Id")),
            "name": s(&info, "Name").trim_start_matches('/').to_string(),
            "image": s(config, "Image"),
            "image_id": s(&info, "Image"),
            "status": s(&info["State"], "Status"),
            "state": s(&info["State"], "Status"),
            "ports": as_list(ports),
            "created": parse_docker_time(&s(&info, "Created")),
            "labels": Value::Null,
            "hostname": s(config, "Hostname"),
            "env": config.get("Env").cloned().unwrap_or(Value::Null),
            "mounts": as_list(mounts),
            "networks": Value::Object(networks),
            "restart_policy": s(&host["RestartPolicy"], "Name"),
        }))
    }

    pub async fn start_container(&self, id: &str) -> Result<()> {
        self.call_ok("POST", &format!("/containers/{}/start", id), vec![], &[])
            .await
    }

    pub async fn stop_container(&self, id: &str) -> Result<()> {
        self.call_ok(
            "POST",
            &format!("/containers/{}/stop?t=30", id),
            vec![],
            &[],
        )
        .await
    }

    pub async fn restart_container(&self, id: &str) -> Result<()> {
        self.call_ok(
            "POST",
            &format!("/containers/{}/restart?t=30", id),
            vec![],
            &[],
        )
        .await
    }

    pub async fn remove_container(&self, id: &str, force: bool) -> Result<()> {
        self.call_ok(
            "DELETE",
            &format!("/containers/{}?force={}", id, force),
            vec![],
            &[],
        )
        .await
    }

    pub async fn container_stats(&self, id: &str) -> Result<Value> {
        let v = self
            .call_json(
                "GET",
                &format!("/containers/{}/stats?stream=false", id),
                vec![],
                &[],
            )
            .await?;
        let cpu = &v["cpu_stats"];
        let pre = &v["precpu_stats"];
        let cpu_delta = (i(&cpu["cpu_usage"], "total_usage")
            - i(&pre["cpu_usage"], "total_usage")) as f64;
        let sys_delta =
            (i(cpu, "system_cpu_usage") - i(pre, "system_cpu_usage")) as f64;
        let mut num_cpu = i(cpu, "online_cpus") as f64;
        if num_cpu == 0.0 {
            num_cpu = arr(&cpu["cpu_usage"]["percpu_usage"]).len() as f64;
        }
        let cpu_pct = if sys_delta > 0.0 {
            (cpu_delta / sys_delta) * num_cpu * 100.0
        } else {
            0.0
        };

        let mem_usage = i(&v["memory_stats"], "usage").max(0) as u64;
        let mem_limit = i(&v["memory_stats"], "limit").max(0) as u64;
        let mem_pct = if mem_limit > 0 {
            mem_usage as f64 / mem_limit as f64 * 100.0
        } else {
            0.0
        };

        let mut net_rx = 0u64;
        let mut net_tx = 0u64;
        if let Some(nets) = v["networks"].as_object() {
            for n in nets.values() {
                net_rx += n.get("rx_bytes").and_then(|x| x.as_u64()).unwrap_or(0);
                net_tx += n.get("tx_bytes").and_then(|x| x.as_u64()).unwrap_or(0);
            }
        }
        let mut block_r = 0u64;
        let mut block_w = 0u64;
        for b in arr(&v["blkio_stats"]["io_service_bytes_recursive"]) {
            match s(b, "op").as_str() {
                "read" => block_r += b.get("value").and_then(|x| x.as_u64()).unwrap_or(0),
                "write" => block_w += b.get("value").and_then(|x| x.as_u64()).unwrap_or(0),
                _ => {}
            }
        }

        Ok(json!({
            "cpu_percent": cpu_pct,
            "memory_usage": mem_usage,
            "memory_limit": mem_limit,
            "memory_percent": mem_pct,
            "net_rx_bytes": net_rx,
            "net_tx_bytes": net_tx,
            "block_read": block_r,
            "block_write": block_w,
        }))
    }

    // ===== LOGS =====

    pub async fn container_logs(&self, id: &str, tail: &str) -> Result<Vec<u8>> {
        let path = format!(
            "/containers/{}/logs?stdout=true&stderr=true&follow=false&tail={}&timestamps=true",
            id,
            url_encode(tail)
        );
        let mut reader = self.call_stream("GET", &path, vec![], &[]).await?;
        reader.read_all().await.map_err(|e| e.to_string())
    }

    pub async fn stream_logs(&self, id: &str) -> Result<BodyReader> {
        let path = format!(
            "/containers/{}/logs?stdout=true&stderr=true&follow=true&tail=200&timestamps=true",
            id
        );
        self.call_stream("GET", &path, vec![], &[]).await
    }

    // ===== EXEC =====

    pub async fn exec_create(
        &self,
        container: &str,
        cmd: &[String],
        tty: bool,
        stdin: bool,
        stdout: bool,
        stderr: bool,
    ) -> Result<String> {
        let body = serde_json::to_vec(&json!({
            "AttachStdin": stdin,
            "AttachStdout": stdout,
            "AttachStderr": stderr,
            "Tty": tty,
            "Cmd": cmd,
        }))
        .unwrap();
        let v = self
            .call_json(
                "POST",
                &format!("/containers/{}/exec", container),
                body,
                &[],
            )
            .await?;
        let id = s(&v, "Id");
        if id.is_empty() {
            return Err("exec create returned no id".to_string());
        }
        Ok(id)
    }

    /// Attach to an exec instance and return the raw hijacked stream plus any
    /// bytes that arrived together with the 101 response headers.
    pub async fn exec_attach(
        &self,
        exec_id: &str,
        tty: bool,
    ) -> Result<(crate::docker::BoxIo, Vec<u8>)> {
        let body = serde_json::to_vec(&json!({ "Detach": false, "Tty": tty })).unwrap();
        let mut resp = self
            .call_hijack("POST", &format!("/exec/{}/start", exec_id), body, &[])
            .await?;
        if !(200..300).contains(&resp.status) && resp.status != 101 {
            let data = resp.body.read_all().await.map_err(|e| e.to_string())?;
            return Err(check_status_err(resp.status, &data));
        }
        let (io, leftover) = resp.body.into_io();
        let io = io.ok_or_else(|| "hijacked connection missing".to_string())?;
        Ok((io, leftover))
    }

    pub async fn exec_capture(&self, container: &str, cmd: &[String]) -> Result<String> {
        let id = self
            .exec_create(container, cmd, false, false, true, false)
            .await?;
        let body = serde_json::to_vec(&json!({ "Detach": false, "Tty": false })).unwrap();
        let mut resp = self
            .call_hijack("POST", &format!("/exec/{}/start", id), body, &[])
            .await?;
        if !(200..300).contains(&resp.status) && resp.status != 101 {
            let data = resp.body.read_all().await.map_err(|e| e.to_string())?;
            return Err(check_status_err(resp.status, &data));
        }
        let data = resp.body.read_all().await.map_err(|e| e.to_string())?;
        Ok(crate::docker::decode_exec_output(&data))
    }

    pub async fn exec_start_detached(&self, exec_id: &str) -> Result<()> {
        let body = serde_json::to_vec(&json!({ "Detach": false, "Tty": false })).unwrap();
        let mut resp = self
            .call(
                "POST",
                &format!("/exec/{}/start", exec_id),
                body,
                &[],
                false,
            )
            .await?;
        let data = resp.body.read_all().await.map_err(|e| e.to_string())?;
        if resp.status >= 400 {
            return Err(check_status_err(resp.status, &data));
        }
        Ok(())
    }

    pub async fn resize_tty(&self, exec_id: &str, h: u32, w: u32) -> Result<()> {
        self.call_ok(
            "POST",
            &format!("/exec/{}/resize?h={}&w={}", exec_id, h, w),
            vec![],
            &[],
        )
        .await
    }

    // ===== FILE BROWSER =====

    pub async fn download_container_file(&self, id: &str, path: &str) -> Result<BodyReader> {
        self.call_stream(
            "GET",
            &format!("/containers/{}/archive?path={}", id, url_encode(path)),
            vec![],
            &[],
        )
        .await
    }

    pub async fn upload_to_container(
        &self,
        id: &str,
        path: &str,
        body: Vec<u8>,
    ) -> Result<()> {
        self.call_ok(
            "PUT",
            &format!("/containers/{}/archive?path={}", id, url_encode(path)),
            body,
            &[("Content-Type", "application/x-tar")],
        )
        .await
    }

    pub async fn delete_container_file(&self, id: &str, path: &str) -> Result<()> {
        let cmd = vec!["rm".to_string(), "-rf".to_string(), path.to_string()];
        let exec_id = self
            .exec_create(id, &cmd, false, false, true, true)
            .await?;
        self.exec_start_detached(&exec_id).await
    }

    // ===== IMAGES =====

    pub async fn list_images(&self) -> Result<Vec<Value>> {
        let v = self.call_json("GET", "/images/json", vec![], &[]).await?;
        let mut result = Vec::new();
        for img in arr(&v) {
            let mut id = s(img, "Id");
            if id.starts_with("sha256:") && id.len() > 19 {
                id = id[7..19].to_string();
            }
            result.push(json!({
                "id": id,
                "repo_tags": img.get("RepoTags").cloned().unwrap_or(Value::Null),
                "size": i(img, "Size"),
                "created": i(img, "Created"),
            }));
        }
        Ok(result)
    }

    pub async fn remove_image(&self, id: &str, force: bool) -> Result<()> {
        self.call_ok(
            "DELETE",
            &format!("/images/{}?force={}", id, force),
            vec![],
            &[],
        )
        .await
    }

    pub async fn load_image(&self, body: Vec<u8>) -> Result<()> {
        self.call_ok(
            "POST",
            "/images/load?quiet=false",
            body,
            &[("Content-Type", "application/x-tar")],
        )
        .await
    }

    pub async fn stream_pull_image(&self, reference: &str) -> Result<BodyReader> {
        let path = self.image_pull_path(reference);
        self.call_stream("POST", &path, vec![], &[REGISTRY_AUTH]).await
    }

    fn image_pull_path(&self, reference: &str) -> String {
        let (from, tag) = split_image_ref(reference);
        let mut path = format!("/images/create?fromImage={}", url_encode(&from));
        if let Some(t) = tag {
            path.push_str(&format!("&tag={}", url_encode(&t)));
        }
        path
    }

    pub async fn image_inspect(&self, reference: &str) -> Result<Value> {
        self.call_json("GET", &format!("/images/{}/json", reference), vec![], &[])
            .await
    }

    pub async fn get_image_id(&self, reference: &str) -> Result<String> {
        let v = self.image_inspect(reference).await?;
        Ok(s(&v, "Id"))
    }

    pub async fn get_local_digests(&self, reference: &str) -> Result<Vec<String>> {
        let v = self.image_inspect(reference).await?;
        Ok(str_list(&v["RepoDigests"]))
    }

    pub async fn get_remote_digest(&self, reference: &str) -> Result<String> {
        let v = self
            .call_json(
                "GET",
                &format!("/distribution/{}/json", reference),
                vec![],
                &[],
            )
            .await?;
        Ok(s(&v["Descriptor"], "Digest"))
    }

    // ===== NETWORKS =====

    pub async fn list_networks(&self) -> Result<Vec<Value>> {
        let v = self.call_json("GET", "/networks", vec![], &[]).await?;
        let mut result = Vec::new();
        for n in arr(&v) {
            let mut ipv4 = String::new();
            let mut ipv6 = String::new();
            for cfg in arr(&n["IPAM"]["Config"]) {
                let subnet = s(cfg, "Subnet");
                if subnet.contains(':') {
                    ipv6 = subnet;
                } else {
                    ipv4 = subnet;
                }
            }
            result.push(json!({
                "id": short_id(&s(n, "Id")),
                "name": s(n, "Name"),
                "driver": s(n, "Driver"),
                "scope": s(n, "Scope"),
                "ipv4": ipv4,
                "ipv6": ipv6,
                "internal": n.get("Internal").and_then(|x| x.as_bool()).unwrap_or(false),
                "created": rfc3339_secs(&s(n, "Created")),
                "containers": n.get("Containers").and_then(|x| x.as_object()).map(|o| o.len()).unwrap_or(0),
            }));
        }
        Ok(result)
    }

    pub async fn create_network(&self, name: &str, driver: &str) -> Result<()> {
        let body = serde_json::to_vec(&json!({ "Name": name, "Driver": driver })).unwrap();
        self.call_ok("POST", "/networks/create", body, &[]).await
    }

    pub async fn remove_network(&self, id: &str) -> Result<()> {
        self.call_ok("DELETE", &format!("/networks/{}", id), vec![], &[])
            .await
    }

    pub async fn prune_networks(&self) -> Result<()> {
        self.call_ok("POST", "/networks/prune", b"{}".to_vec(), &[])
            .await
    }

    // ===== VOLUMES =====

    pub async fn list_volumes(&self) -> Result<Vec<Value>> {
        let v = self.call_json("GET", "/volumes", vec![], &[]).await?;
        let mut result = Vec::new();
        for vol in arr(&v["Volumes"]) {
            result.push(json!({
                "name": s(vol, "Name"),
                "driver": s(vol, "Driver"),
                "mountpoint": s(vol, "Mountpoint"),
                "scope": s(vol, "Scope"),
                "created": s(vol, "CreatedAt"),
            }));
        }
        Ok(result)
    }

    pub async fn create_volume(&self, name: &str) -> Result<()> {
        let body = serde_json::to_vec(&json!({ "Name": name })).unwrap();
        self.call_ok("POST", "/volumes/create", body, &[]).await
    }

    pub async fn remove_volume(&self, name: &str, force: bool) -> Result<()> {
        self.call_ok(
            "DELETE",
            &format!("/volumes/{}?force={}", name, force),
            vec![],
            &[],
        )
        .await
    }

    pub async fn prune_volumes(&self) -> Result<()> {
        self.call_ok("POST", "/volumes/prune", b"{}".to_vec(), &[])
            .await
    }

    // ===== REBUILD (update container image) =====

    pub async fn rebuild_container(&self, name: &str) -> Result<()> {
        let info = self
            .inspect_raw(name)
            .await
            .map_err(|e| format!("inspect failed: {}", e))?;
        let config = &info["Config"];
        let host = &info["HostConfig"];
        let img = s(config, "Image");

        // 2. Pull latest image
        let path = self.image_pull_path(&img);
        let mut reader = self
            .call_stream("POST", &path, vec![], &[REGISTRY_AUTH])
            .await
            .map_err(|e| format!("pull failed: {}", e))?;
        while let Some(chunk) = reader.next_chunk().await.map_err(|e| e.to_string())? {
            let _ = chunk;
        }

        // 3. Stop (no explicit timeout) and force-remove the old container.
        let _ = self
            .call_ok("POST", &format!("/containers/{}/stop", name), vec![], &[])
            .await;
        self.call_ok(
            "DELETE",
            &format!("/containers/{}?force=true", name),
            vec![],
            &[],
        )
        .await
        .map_err(|e| format!("remove failed: {}", e))?;

        // 4/5/6. Rebuild port bindings, binds and network endpoints.
        let port_bindings = host.get("PortBindings").cloned().unwrap_or(Value::Null);
        let mut exposed = Map::new();
        if let Some(pb) = port_bindings.as_object() {
            for k in pb.keys() {
                exposed.insert(k.clone(), json!({}));
            }
        }
        let mut endpoints = Map::new();
        if let Some(nets) = info["NetworkSettings"]["Networks"].as_object() {
            for (net, ep) in nets {
                endpoints.insert(net.clone(), json!({ "Aliases": ep.get("Aliases").cloned().unwrap_or(Value::Null) }));
            }
        }

        let body = serde_json::to_vec(&json!({
            "Image": img,
            "Env": config.get("Env").cloned().unwrap_or(Value::Null),
            "Cmd": config.get("Cmd").cloned().unwrap_or(Value::Null),
            "Entrypoint": config.get("Entrypoint").cloned().unwrap_or(Value::Null),
            "ExposedPorts": Value::Object(exposed),
            "Labels": config.get("Labels").cloned().unwrap_or(Value::Null),
            "User": config.get("User").cloned().unwrap_or(Value::Null),
            "WorkingDir": config.get("WorkingDir").cloned().unwrap_or(Value::Null),
            "Hostname": config.get("Hostname").cloned().unwrap_or(Value::Null),
            "HostConfig": {
                "PortBindings": port_bindings,
                "Binds": host.get("Binds").cloned().unwrap_or(Value::Null),
                "RestartPolicy": host.get("RestartPolicy").cloned().unwrap_or(Value::Null),
                "NetworkMode": host.get("NetworkMode").cloned().unwrap_or(Value::Null),
                "Privileged": host.get("Privileged").cloned().unwrap_or(Value::Null),
                "CapAdd": host.get("CapAdd").cloned().unwrap_or(Value::Null),
                "CapDrop": host.get("CapDrop").cloned().unwrap_or(Value::Null),
                "ExtraHosts": host.get("ExtraHosts").cloned().unwrap_or(Value::Null),
                "ShmSize": host.get("ShmSize").cloned().unwrap_or(Value::Null),
                "LogConfig": host.get("LogConfig").cloned().unwrap_or(Value::Null),
            },
            "NetworkingConfig": { "EndpointsConfig": Value::Object(endpoints) },
        }))
        .unwrap();

        let created = self
            .call_json(
                "POST",
                &format!("/containers/create?name={}", url_encode(name)),
                body,
                &[],
            )
            .await
            .map_err(|e| format!("create failed: {}", e))?;
        let new_id = s(&created, "Id");

        self.call_ok("POST", &format!("/containers/{}/start", new_id), vec![], &[])
            .await
            .map_err(|e| format!("start failed: {}", e))
    }
}
