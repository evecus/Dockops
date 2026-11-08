//! Container file browser, ported from internal/docker/client.go
//! (ListContainerFiles / listFilesViaStat / parseLsLine).

use crate::docker::DockerClient;
use crate::error::Result;
use serde_json::{json, Value};

/// Go's `fmt.Sscanf("%d")` semantics: parse a leading (optionally signed)
/// integer and ignore any trailing garbage.
fn sscanf_int(s: &str) -> Option<i64> {
    let t = s.trim_start();
    let mut end = 0usize;
    for (idx, ch) in t.char_indices() {
        if idx == 0 && (ch == '+' || ch == '-') {
            end = ch.len_utf8();
            continue;
        }
        if ch.is_ascii_digit() {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    t[..end].parse::<i64>().ok()
}

fn month_num(s: &str) -> Option<u32> {
    let m = match s {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    Some(m)
}

/// Reproduce Go's time.Parse with layouts "Jan 2 15:04" / "Jan 2 2006":
/// a missing year means year 0 (UTC), so the Unix timestamp is negative.
fn parse_ls_date(fields: &[String]) -> i64 {
    if fields.len() < 3 {
        return 0;
    }
    let mon = match month_num(&fields[0]) {
        Some(m) => m,
        None => return 0,
    };
    let day: u32 = match fields[1].parse() {
        Ok(d) => d,
        Err(_) => return 0,
    };
    if fields[2].contains(':') {
        let hm: Vec<&str> = fields[2].split(':').collect();
        let h: u32 = hm.first().and_then(|x| x.parse().ok()).unwrap_or(0);
        let m: u32 = hm.get(1).and_then(|x| x.parse().ok()).unwrap_or(0);
        chrono::NaiveDate::from_ymd_opt(0, mon, day)
            .and_then(|d| d.and_hms_opt(h, m, 0))
            .map(|dt| dt.and_utc().timestamp())
            .unwrap_or(0)
    } else {
        let year: i32 = match fields[2].parse() {
            Ok(y) => y,
            Err(_) => return 0,
        };
        chrono::NaiveDate::from_ymd_opt(year, mon, day)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc().timestamp())
            .unwrap_or(0)
    }
}

/// filepath.Join + Clean (Go semantics).
fn path_join(dir: &str, name: &str) -> String {
    let raw = format!("{}/{}", dir.trim_end_matches('/'), name);
    let mut parts: Vec<&str> = Vec::new();
    for seg in raw.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(seg),
        }
    }
    let joined = parts.join("/");
    if raw.starts_with('/') {
        format!("/{}", joined)
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

fn file_entry(name: String, path: String, size: i64, mode: String, mod_time: i64, is_dir: bool) -> Value {
    json!({
        "name": name,
        "path": path,
        "size": size,
        "mode": mode,
        "mod_time": mod_time,
        "is_dir": is_dir,
    })
}

/// GNU ls format check: first meaningful line has >=7 fields and field[5]
/// parses as a positive integer (unix timestamp).
fn looks_like_gnu_ls(output: &str) -> bool {
    for line in output.split('\n') {
        let line = line.trim();
        if line.is_empty() || line.starts_with("total ") {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() >= 7 {
            if let Some(ts) = sscanf_int(fields[5]) {
                if ts > 0 {
                    return true;
                }
            }
        }
        return false;
    }
    false
}

fn parse_ls_line(line: &str, dir_path: &str, gnu_format: bool) -> Option<Value> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 5 {
        return None;
    }
    let perms = fields[0];
    let first = perms.chars().next()?;
    if !"-dlcbsp".contains(first) {
        return None;
    }
    let is_dir = first == 'd';
    let size = sscanf_int(fields[4]).unwrap_or(0);
    let mut mod_time = 0i64;
    let name: String;

    if gnu_format && fields.len() >= 7 {
        mod_time = sscanf_int(fields[5]).unwrap_or(0);
        name = fields[6..].join(" ");
    } else if !gnu_format && fields.len() >= 9 {
        name = fields[8..].join(" ");
        let date_fields: Vec<String> = fields[5..8].iter().map(|s| s.to_string()).collect();
        mod_time = parse_ls_date(&date_fields);
    } else if fields.len() >= 7 {
        name = fields[6..].join(" ");
    } else {
        return None;
    }

    let mut name = name;
    if let Some(idx) = name.find(" -> ") {
        name = name[..idx].to_string();
    }
    let name = name.trim().to_string();
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    Some(file_entry(
        name.clone(),
        path_join(dir_path, &name),
        size,
        perms.to_string(),
        mod_time,
        is_dir,
    ))
}

/// BusyBox fallback: find + stat.
async fn list_files_via_stat(client: &DockerClient, id: &str, path: &str) -> Result<Vec<Value>> {
    let script = format!(
        "find \"{}\" -maxdepth 1 -mindepth 1 -exec stat -c '%n\\t%s\\t%A\\t%Y\\t%F' {{}} \\; 2>/dev/null",
        path
    );
    let cmd = vec!["sh".to_string(), "-c".to_string(), script];
    let output = client.exec_capture(id, &cmd).await?;
    if output.trim().is_empty() {
        return Err("stat failed".to_string());
    }
    let mut entries = Vec::new();
    for line in output.split('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 5 {
            continue;
        }
        let full_path = parts[0];
        let name = std::path::Path::new(full_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if name == "." || name == ".." {
            continue;
        }
        let size = sscanf_int(parts[1]).unwrap_or(0);
        let mode = parts[2].to_string();
        let mod_time = sscanf_int(parts[3]).unwrap_or(0);
        let is_dir = parts[4] == "directory";
        entries.push(file_entry(name, full_path.to_string(), size, mode, mod_time, is_dir));
    }
    if entries.is_empty() {
        return Err("no entries".to_string());
    }
    Ok(entries)
}

pub async fn list_container_files(client: &DockerClient, id: &str, path: &str) -> Result<Vec<Value>> {
    // Strategy 1: GNU ls with unix timestamp
    let cmd = vec![
        "ls".to_string(),
        "-la".to_string(),
        "--time-style=+%s".to_string(),
        path.to_string(),
    ];
    let first = client.exec_capture(id, &cmd).await;
    let gnu_format = matches!(&first, Ok(out) if looks_like_gnu_ls(out));

    let mut output = first.unwrap_or_default();
    if !gnu_format {
        // Strategy 2: find + stat (BusyBox)
        if let Ok(entries) = list_files_via_stat(client, id, path).await {
            return Ok(entries);
        }
        // Strategy 3: plain ls -la, best-effort parsing
        let cmd = vec!["ls".to_string(), "-la".to_string(), path.to_string()];
        output = client.exec_capture(id, &cmd).await.unwrap_or_default();
    }

    let mut entries = Vec::new();
    for line in output.split('\n') {
        let line = line.trim();
        if line.is_empty() || line.starts_with("total ") {
            continue;
        }
        if let Some(entry) = parse_ls_line(line, path, gnu_format) {
            let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name == "." || name == ".." {
                continue;
            }
            entries.push(entry);
        }
    }
    Ok(entries)
}
