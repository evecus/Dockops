use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Version is injected at compile time (equivalent to Go's -ldflags -X main.Version).
    // `cross` does not pass arbitrary env vars into its container, so CI writes
    // a VERSION file which build.rs also reads.
    let version = env::var("DOCKOPS_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| {
            std::fs::read_to_string("VERSION")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|v| !v.is_empty())
        })
        .unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=DOCKOPS_VERSION={}", version);
    println!("cargo:rerun-if-env-changed=DOCKOPS_VERSION");
    println!("cargo:rerun-if-changed=VERSION");
    println!("cargo:rerun-if-changed=web/dist");

    // rust-embed expands at compile time and requires the folder to exist.
    // If the frontend has not been built yet, drop in a placeholder index.html
    // so `cargo build` never fails; a real build overwrites it.
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let dist = manifest.join("web").join("dist");
    let index = dist.join("index.html");
    if !index.exists() {
        let _ = fs::create_dir_all(&dist);
        let _ = fs::write(
            &index,
            "<!doctype html><html><body><h1>DockOps</h1><p>Frontend assets are not built. Run: cd web &amp;&amp; npm ci &amp;&amp; npm run build</p></body></html>",
        );
    }
}
