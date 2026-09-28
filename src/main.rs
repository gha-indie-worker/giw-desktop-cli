#![allow(clippy::needless_return)]

use anyhow::{Context as _, Result, anyhow, bail};
use flags2env::BundledFlags2Env;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    env,
    net::IpAddr,
    path::{Path, PathBuf},
    time::Duration,
};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_TOKEN_FILE_BYTES: u64 = 16 * 1024;
const MAX_TOKEN_BYTES: usize = 4096;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct CliConfig {
    GIW_DESKTOP_DAEMON_URL: String,
    GIW_DESKTOP_TIMEOUT_MS: i64,
    GIW_DESKTOP_PAYLOAD: Option<Value>,
    FLAGS2ENV_COMMAND: Option<String>,
}

#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("giw-desktop: {error}");
            2
        }
    };
    std::process::exit(code);
}

async fn run() -> Result<()> {
    let config_path = resolve_config_path()?;
    let config_path_text = config_path
        .to_str()
        .ok_or_else(|| anyhow!(".cli-flags.toml path is not UTF-8"))?;
    let parser = BundledFlags2Env::new();
    parser
        .audit_config(Some(config_path_text))
        .map_err(|error| anyhow!("flags-2-env configuration audit failed: {error}"))?;

    let argv = env::args().collect::<Vec<_>>();
    let parsed = parser
        .parse_structured(&argv, Some(config_path_text))
        .map_err(|error| anyhow!("flags-2-env parse failed: {error}"))?;
    if !parsed.unknown_options.is_empty() {
        bail!(
            "unknown command-line options: {}",
            parsed.unknown_options.len()
        );
    }
    if !parsed.errors.is_empty() {
        bail!("invalid command-line values: {}", parsed.errors.join("; "));
    }
    if !parsed.extras.is_empty() {
        bail!("unexpected positional arguments: {}", parsed.extras.len());
    }

    let mut raw = env::vars().collect::<HashMap<_, _>>();
    raw.remove("FLAGS2ENV_COMMAND");
    raw.extend(parsed.provided_flags);
    let config = parser
        .coerce::<CliConfig, _>(&raw, Some(config_path_text))
        .map_err(|error| anyhow!("flags-2-env typed configuration failed: {error}"))?;
    let timeout_ms = u64::try_from(config.GIW_DESKTOP_TIMEOUT_MS)
        .ok()
        .filter(|value| *value > 0 && *value <= 1_200_000)
        .ok_or_else(|| anyhow!("--timeout must be between 1 and 1200000 ms"))?;
    let token = read_token()?;
    let base = validate_daemon_url(&config.GIW_DESKTOP_DAEMON_URL)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_millis(timeout_ms))
        .build()?;

    match config.FLAGS2ENV_COMMAND.as_deref().unwrap_or("") {
        "status" => {
            let response = client
                .get(format!("{base}/v1/status"))
                .bearer_auth(&token)
                .send()
                .await?;
            print_response(response).await?;
        }
        "dispatch" => {
            let payload = config.GIW_DESKTOP_PAYLOAD.unwrap_or_else(|| json!({}));
            let response = client
                .post(format!("{base}/v1/jobs/dispatch"))
                .bearer_auth(&token)
                .json(&payload)
                .send()
                .await?;
            print_response(response).await?;
        }
        _ => {
            bail!("command required: status or dispatch");
        }
    }
    return Ok(());
}

async fn read_json_response(mut response: reqwest::Response) -> Result<Value> {
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        bail!("daemon response exceeds {MAX_RESPONSE_BYTES} bytes");
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            bail!("daemon response exceeds {MAX_RESPONSE_BYTES} bytes");
        }
        body.extend_from_slice(&chunk);
    }

    if !status.is_success() {
        let text = String::from_utf8_lossy(&body);
        bail!("daemon returned {status}: {text}");
    }
    return serde_json::from_slice(&body).context("daemon response was not JSON");
}

async fn print_response(response: reqwest::Response) -> Result<()> {
    let value = read_json_response(response).await?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    return Ok(());
}

fn parse_literal_loopback_host(host: &str) -> Result<IpAddr> {
    let normalized = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    let ip = normalized
        .parse::<IpAddr>()
        .context("GIW desktop daemon URL host must be a literal IP address")?;
    if !ip.is_loopback() {
        bail!("GIW desktop daemon URL must target a literal loopback address");
    }
    return Ok(ip);
}

fn validate_daemon_url(raw: &str) -> Result<String> {
    let url = reqwest::Url::parse(raw).context("GIW_DESKTOP_DAEMON_URL is not a valid URL")?;
    if url.scheme() != "http" {
        bail!("GIW_DESKTOP_DAEMON_URL must use http://");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("GIW_DESKTOP_DAEMON_URL must not contain credentials");
    }
    if url.query().is_some() || url.fragment().is_some() {
        bail!("GIW_DESKTOP_DAEMON_URL must not contain a query or fragment");
    }
    if url.path() != "/" && !url.path().is_empty() {
        bail!("GIW_DESKTOP_DAEMON_URL must not contain a base path");
    }
    if url.port().is_none() {
        bail!("GIW_DESKTOP_DAEMON_URL must include an explicit port");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("GIW_DESKTOP_DAEMON_URL must include a host"))?;
    let _ = parse_literal_loopback_host(host)?;
    return Ok(url.as_str().trim_end_matches('/').to_owned());
}

fn resolve_config_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os("GIW_DESKTOP_FLAGS_CONFIG") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        bail!("GIW_DESKTOP_FLAGS_CONFIG is not a readable file");
    }
    let current = env::current_dir()?.join(".cli-flags.toml");
    if current.is_file() {
        return Ok(current);
    }
    let executable = env::current_exe()?;
    if let Some(parent) = executable.parent() {
        let adjacent = parent.join(".cli-flags.toml");
        if adjacent.is_file() {
            return Ok(adjacent);
        }
    }
    bail!("cannot locate .cli-flags.toml");
}

fn read_token() -> Result<String> {
    let path = if let Some(path) = env::var_os("GIW_DESKTOP_TOKEN_FILE") {
        PathBuf::from(path)
    } else {
        home_dir()?.join(".indiebuild/daemon/token")
    };
    validate_token_file(&path)?;
    let token = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read daemon token at {}", path.display()))?;
    let token = token.trim();
    if token.len() < 32 || token.len() > MAX_TOKEN_BYTES || token.chars().any(char::is_whitespace) {
        bail!("daemon token is malformed");
    }
    return Ok(token.to_owned());
}

fn validate_token_file(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("cannot inspect daemon token at {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        bail!("daemon token must be a regular non-symlink file");
    }
    if metadata.len() == 0 || metadata.len() > MAX_TOKEN_FILE_BYTES {
        bail!("daemon token file has an invalid size");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            bail!("daemon token file must not be accessible by group or others");
        }
    }
    return Ok(());
}

fn home_dir() -> Result<PathBuf> {
    if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(home));
    }
    if let Some(profile) = env::var_os("USERPROFILE").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(profile));
    }
    let drive = env::var_os("HOMEDRIVE").filter(|value| !value.is_empty());
    let path = env::var_os("HOMEPATH").filter(|value| !value.is_empty());
    if let (Some(drive), Some(path)) = (drive, path) {
        let mut value = PathBuf::from(drive);
        value.push(path);
        return Ok(value);
    }
    return Err(anyhow!("cannot determine user home directory"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_url_requires_literal_loopback_http() -> Result<()> {
        assert_eq!(
            validate_daemon_url("http://127.0.0.1:18440")?,
            "http://127.0.0.1:18440"
        );
        assert_eq!(
            validate_daemon_url("http://[::1]:18440")?,
            "http://[::1]:18440"
        );
        assert!(validate_daemon_url("http://localhost:18440").is_err());
        assert!(validate_daemon_url("https://127.0.0.1:18440").is_err());
        assert!(validate_daemon_url("http://192.0.2.1:18440").is_err());
        assert!(validate_daemon_url("http://user:pass@127.0.0.1:18440").is_err());
        assert!(validate_daemon_url("http://127.0.0.1:18440/base").is_err());
        return Ok(());
    }
}
