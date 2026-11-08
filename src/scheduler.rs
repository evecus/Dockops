//! Background scheduler, equivalent to internal/scheduler/scheduler.go.
//! The Go version uses robfig/cron with a fixed set of specs; here the next
//! fire time is computed directly (same alignment semantics) so no cron crate
//! (and no extra C dependency) is needed.

use crate::db::Db;
use crate::docker::DockerClient;
use crate::error::Result;
use crate::log_info;
use chrono::{DateTime, Local, SecondsFormat, Timelike, Utc};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::watch;

#[derive(Clone, Copy)]
enum Spec {
    EveryMinute,
    EveryNMinutes(u32),
    EveryHour,
    EveryNHours(u32),
    Daily,
}

fn collect_interval_to_spec(d: &str) -> Spec {
    match d {
        "1m" => Spec::EveryMinute,
        "5m" => Spec::EveryNMinutes(5),
        "10m" => Spec::EveryNMinutes(10),
        "30m" => Spec::EveryNMinutes(30),
        _ => Spec::EveryNMinutes(10), // "*/10 * * * *"
    }
}

fn image_check_interval_to_spec(d: &str) -> Spec {
    match d {
        "1h" => Spec::EveryHour,            // "0 * * * *"
        "6h" => Spec::EveryNHours(6),      // "0 */6 * * *"
        "12h" => Spec::EveryNHours(12),    // "0 */12 * * *"
        "24h" => Spec::Daily,              // "0 0 * * *"
        _ => Spec::EveryNHours(6),
    }
}

fn next_tick(spec: Spec, now: DateTime<Local>) -> DateTime<Local> {
    let floored = now
        .with_second(0)
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(now);
    match spec {
        Spec::EveryMinute => floored + chrono::Duration::minutes(1),
        Spec::EveryNMinutes(n) => {
            let n = n.max(1);
            let mut c = floored + chrono::Duration::minutes(1);
            while (c.minute() as u32) % n != 0 {
                c = c + chrono::Duration::minutes(1);
            }
            c
        }
        Spec::EveryHour => {
            let base = floored.with_minute(0).unwrap_or(floored);
            base + chrono::Duration::hours(1)
        }
        Spec::EveryNHours(n) => {
            let n = n.max(1);
            let mut c = floored
                .with_minute(0)
                .unwrap_or(floored)
                + chrono::Duration::hours(1);
            while (c.hour() as u32) % n != 0 {
                c = c + chrono::Duration::hours(1);
            }
            c
        }
        Spec::Daily => {
            let base = floored
                .with_hour(0)
                .and_then(|d| d.with_minute(0))
                .unwrap_or(floored);
            base + chrono::Duration::days(1)
        }
    }
}

fn sleep_duration(spec: Spec) -> Duration {
    let now = Local::now();
    (next_tick(spec, now) - now)
        .to_std()
        .unwrap_or_else(|_| Duration::from_secs(60))
}

pub struct Scheduler {
    db: Arc<Db>,
    cache: RwLock<(Option<Value>, Option<Value>)>,
    img_cache: RwLock<HashMap<String, Value>>,
    collect_tx: watch::Sender<String>,
    image_tx: watch::Sender<String>,
}

impl Scheduler {
    pub fn new(db: Arc<Db>) -> Arc<Scheduler> {
        let (collect_tx, _) = watch::channel("10m".to_string());
        let (image_tx, _) = watch::channel("6h".to_string());
        Arc::new(Scheduler {
            db,
            cache: RwLock::new((None, None)),
            img_cache: RwLock::new(HashMap::new()),
            collect_tx,
            image_tx,
        })
    }

    /// Mirror of Start(): read intervals from settings, schedule, then kick
    /// off an immediate dashboard collection.
    pub fn start(self: &Arc<Self>) {
        let collect = self
            .db
            .get_setting("collect_interval")
            .unwrap_or_default();
        let collect = if collect.is_empty() {
            "10m".to_string()
        } else {
            collect
        };
        let image = self
            .db
            .get_setting("update_check_interval")
            .unwrap_or_default();
        let image = if image.is_empty() { "6h".to_string() } else { image };

        let _ = self.collect_tx.send(collect);
        let _ = self.image_tx.send(image);

        let collect_rx = self.collect_tx.subscribe();
        let image_rx = self.image_tx.subscribe();
        let s = self.clone();
        tokio::spawn(async move { s.collect_loop(collect_rx).await });
        let s = self.clone();
        tokio::spawn(async move { s.image_loop(image_rx).await });
        let s = self.clone();
        tokio::spawn(async move { s.collect_dashboard().await });
    }

    async fn collect_loop(self: Arc<Self>, mut rx: watch::Receiver<String>) {
        loop {
            let interval = rx.borrow().clone();
            if interval == "off" {
                if rx.changed().await.is_err() {
                    return;
                }
                continue;
            }
            let spec = collect_interval_to_spec(&interval);
            tokio::select! {
                _ = tokio::time::sleep(sleep_duration(spec)) => {
                    self.collect_dashboard().await;
                }
                _ = rx.changed() => {}
            }
        }
    }

    async fn image_loop(self: Arc<Self>, mut rx: watch::Receiver<String>) {
        loop {
            let interval = rx.borrow().clone();
            if interval == "off" {
                if rx.changed().await.is_err() {
                    return;
                }
                continue;
            }
            let spec = image_check_interval_to_spec(&interval);
            tokio::select! {
                _ = tokio::time::sleep(sleep_duration(spec)) => {
                    self.check_image_updates().await;
                }
                _ = rx.changed() => {}
            }
        }
    }

    pub fn update_collect_interval(&self, interval: &str) -> Result<()> {
        self.db.set_setting("collect_interval", interval)?;
        let _ = self.collect_tx.send(interval.to_string());
        Ok(())
    }

    pub fn update_image_check_interval(&self, interval: &str) -> Result<()> {
        self.db.set_setting("update_check_interval", interval)?;
        let _ = self.image_tx.send(interval.to_string());
        Ok(())
    }

    pub fn collect_now(self: &Arc<Self>) {
        let s = self.clone();
        tokio::spawn(async move { s.collect_dashboard().await });
    }

    #[allow(dead_code)]
    pub fn check_image_updates_now(self: &Arc<Self>) {
        let s = self.clone();
        tokio::spawn(async move { s.check_image_updates().await });
    }

    pub fn cache_get(&self) -> (Option<Value>, Option<Value>) {
        self.cache.read().unwrap().clone()
    }

    pub fn get_image_update_status(&self) -> Value {
        let map = self.img_cache.read().unwrap().clone();
        let mut out = serde_json::Map::new();
        for (k, v) in map {
            out.insert(k, v);
        }
        Value::Object(out)
    }

    pub fn mark_image_updated(&self, tag: &str) {
        let status = json!({
            "tag": tag,
            "has_update": false,
            "checked_at": Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
        });
        self.img_cache
            .write()
            .unwrap()
            .insert(tag.to_string(), status);
    }

    fn set_image_status(&self, tag: &str, has_update: bool) {
        let status = json!({
            "tag": tag,
            "has_update": has_update,
            "checked_at": Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
        });
        self.img_cache
            .write()
            .unwrap()
            .insert(tag.to_string(), status);
    }

    pub async fn collect_dashboard(self: &Arc<Self>) {
        let fut = async {
            let client = DockerClient::new();
            let info = client.get_system_info().await?;
            let containers = client.list_containers().await?;

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
            let stats = json!({
                "total_cpu_percent": total_cpu,
                "total_mem_usage": total_mem,
                "total_mem_limit": total_mem_limit,
                "containers": containers.len(),
            });
            {
                let mut cache = self.cache.write().unwrap();
                cache.0 = Some(info);
                cache.1 = Some(stats);
            }
            Ok::<(), String>(())
        };
        match tokio::time::timeout(Duration::from_secs(30), fut).await {
            Ok(Err(e)) => log_info!("Dashboard collect: {}", e),
            Err(_) => log_info!("Dashboard collect: timeout"),
            _ => {}
        }
    }

    pub async fn check_image_updates(self: &Arc<Self>) {
        let fut = async {
            let client = DockerClient::new();
            let containers = client.list_containers().await?;
            let mut seen = HashSet::new();
            for ct in containers {
                let tag = ct.get("image").and_then(|v| v.as_str()).unwrap_or("");
                if tag.is_empty() || tag == "<none>:<none>" || !seen.insert(tag.to_string()) {
                    continue;
                }
                match check_tag_has_update(&client, tag).await {
                    Ok(has_update) => self.set_image_status(tag, has_update),
                    Err(e) => {
                        log_info!("Image update check [{}]: {}", tag, e);
                    }
                }
            }
            Ok::<(), String>(())
        };
        match tokio::time::timeout(Duration::from_secs(600), fut).await {
            Ok(Err(e)) => log_info!("Image update check: {}", e),
            Err(_) => log_info!("Image update check: timeout"),
            _ => {}
        }
    }
}

async fn check_tag_has_update(client: &DockerClient, tag: &str) -> Result<bool> {
    let _local_id = client.get_image_id(tag).await?;
    let remote_digest = match client.get_remote_digest(tag).await {
        Ok(d) => d,
        Err(_) => return Ok(false), // network / private registry: skip silently
    };
    let local_digests = client.get_local_digests(tag).await?;
    for d in local_digests {
        if d.contains(&remote_digest) || d == remote_digest {
            return Ok(false);
        }
    }
    Ok(true)
}
