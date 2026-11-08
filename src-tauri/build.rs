use std::path::Path;

/// Read `GITHUB_CLIENT_ID` from a minimal `.env` file (KEY=VALUE lines).
/// Comments, blank lines, optional `export `, and surrounding quotes are handled.
fn parse_env_file(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "GITHUB_CLIENT_ID" {
            continue;
        }
        let value = value.trim().trim_matches(|c| c == '"' || c == '\'').trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn main() {
    tauri_build::build();

    // Resolve the OAuth client id at build time so shipped binaries need no
    // per-user configuration. Precedence: process env (CI secret) > repo `.env`.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let env_path = Path::new(&manifest_dir).join("..").join(".env");
    println!("cargo:rerun-if-changed={}", env_path.display());
    println!("cargo:rerun-if-env-changed=GITHUB_CLIENT_ID");

    let client_id = std::env::var("GITHUB_CLIENT_ID")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| {
            std::fs::read_to_string(&env_path)
                .ok()
                .and_then(|contents| parse_env_file(&contents))
        });

    if let Some(client_id) = client_id {
        println!("cargo:rustc-env=GITHUB_CLIENT_ID={client_id}");
    }
}
