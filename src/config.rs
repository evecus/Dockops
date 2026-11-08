use crate::error::Result;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub http_port: u16,
    pub https_port: u16,
    pub cert_path: String,
    pub key_path: String,
    pub data_path: String,
}

impl Config {
    /// Zero values mean "use default", exactly like Go's flag handling.
    pub fn new(http_port: i64, https_port: i64, data_dir: String) -> Self {
        let mut c = Config {
            http_port: 9080,
            https_port: 9443,
            cert_path: String::new(),
            key_path: String::new(),
            data_path: default_data_path(),
        };
        if http_port > 0 {
            c.http_port = http_port as u16;
        }
        if https_port > 0 {
            c.https_port = https_port as u16;
        }
        if !data_dir.is_empty() {
            c.data_path = data_dir;
        }
        c
    }

    /// Creates required directories and auto-detects TLS certs in <data>/cert/.
    pub fn init(&mut self) -> Result<()> {
        let cert_dir = PathBuf::from(&self.data_path).join("cert");
        let compose_dir = PathBuf::from(&self.data_path).join("compose");
        for d in [&PathBuf::from(&self.data_path), &cert_dir, &compose_dir] {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }

        // cert.pem (or server.crt / fullchain.pem), key.pem (or server.key / privkey.pem)
        for cert in ["cert.pem", "fullchain.pem", "server.crt"] {
            let p = cert_dir.join(cert);
            if p.exists() {
                self.cert_path = p.to_string_lossy().to_string();
                break;
            }
        }
        for key in ["key.pem", "privkey.pem", "server.key"] {
            let p = cert_dir.join(key);
            if p.exists() {
                self.key_path = p.to_string_lossy().to_string();
                break;
            }
        }
        // Only enable HTTPS when both files are found.
        if self.cert_path.is_empty() || self.key_path.is_empty() {
            self.cert_path.clear();
            self.key_path.clear();
        }
        Ok(())
    }
}

fn default_data_path() -> String {
    match std::env::current_exe() {
        Ok(exe) => match exe.parent() {
            Some(dir) => dir.join("data").to_string_lossy().to_string(),
            None => "./data".to_string(),
        },
        Err(_) => "./data".to_string(),
    }
}
