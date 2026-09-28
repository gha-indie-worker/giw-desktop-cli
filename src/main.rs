#![forbid(unsafe_code)]

use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use reqwest::{Client, Method, StatusCode, Url};
use serde_json::{Value, json};

const PROTOCOL_VERSION: u64 = 1;
const DEFAULT_DAEMON_URL: &str = "http://127.0.0.1:18440";
const MAX_TOKEN_BYTES: usize = 16_384;
const MAX_RESPONSE_BYTES: usize = 1_048_576;
const MAX_SERVICE_NAME_CHARS: usize = 128;
const CONNECT_TIMEOUT_SECONDS: u64 = 2;
const REQUEST_TIMEOUT_SECONDS: u64 = 15;

#[derive(Debug, Parser)]
#[command(
    name = "giw-desktop",
    about = "Control the local IndieBuild desktop daemon"
)]
struct Args {
    #[arg(long, default_value = DEFAULT_DAEMON_URL, env = "GIW_DESKTOP_URL")]
    daemon_url: String,

    #[arg(long, env = "GIW_DESKTOP_TOKEN_FILE")]
    token_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status,
    Reconcile,
    Processes,
    Process {
        #[arg(value_enum)]
        action: ProcessAction,
        name: String,
    },
    Tunnel {
        #[arg(value_enum)]
        action: TunnelAction,
    },
    KeepAwake {
        #[arg(value_enum)]
        state: Toggle,
    },
    Update,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProcessAction {
    Start,
    Stop,
    Restart,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TunnelAction {
    Start,
    Stop,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Toggle {
    On,
    Off,
}

fn home_dir() -> Result<PathBuf> {
    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home));
    }
    if let Some(profile) = env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(profile));
    }
    bail!("HOME or USERPROFILE must be set");
}

fn token_path(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    return Ok(home_dir()?.join(".giw").join("desktop").join("token"));
}

fn read_token(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect daemon token at {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "daemon token path must be a regular non-symlink file: {}",
            path.display()
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            bail!(
                "daemon token file must not grant group/other permissions: {}",
                path.display()
            );
        }
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read daemon token at {}", path.display()))?;
    if raw.len() > MAX_TOKEN_BYTES + 1 {
        bail!("daemon token at {} is too large", path.display());
    }

    let token = raw.strip_suffix('\n').unwrap_or(&raw);
    if token.len() < 32
        || token.len() > MAX_TOKEN_BYTES
        || token.chars().any(char::is_whitespace)
    {
        bail!("daemon token at {} is malformed", path.display());
    }
    return Ok(token.to_string());
}

fn parse_daemon_base(raw: &str) -> Result<Url> {
    let url = Url::parse(raw).with_context(|| format!("invalid GIW desktop daemon URL {raw:?}"))?;

    if url.scheme() != "http" {
        bail!("GIW desktop daemon URL must use http on literal loopback");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("GIW desktop daemon URL must not contain URL credentials");
    }
    if !matches!(url.host_str(), Some("127.0.0.1" | "::1")) {
        bail!("GIW desktop daemon URL must use literal loopback 127.0.0.1 or ::1");
    }
    if url.port().is_none() {
        bail!("GIW desktop daemon URL must include an explicit port");
    }
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        bail!("GIW desktop daemon URL must be an origin only, without path/query/fragment");
    }

    return Ok(url);
}

fn endpoint(base: &Url, path: &str) -> Result<Url> {
    if !path.starts_with('/') {
        bail!("daemon endpoint path must be absolute");
    }
    return base
        .join(path.trim_start_matches('/'))
        .context("failed to construct local daemon endpoint");
}

fn valid_service_name(name: &str) -> bool {
    if name.is_empty() || name.chars().count() > MAX_SERVICE_NAME_CHARS {
        return false;
    }

    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }

    return chars.all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '_' | '-')
    });
}

async fn read_bounded_response(mut response: reqwest::Response) -> Result<(StatusCode, String)> {
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        bail!("daemon response exceeds {MAX_RESPONSE_BYTES} bytes");
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .context("failed to read daemon response")?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            bail!("daemon response exceeds {MAX_RESPONSE_BYTES} bytes");
        }
        bytes.extend_from_slice(&chunk);
    }

    let text = String::from_utf8(bytes).context("daemon response is not UTF-8")?;
    return Ok((status, text));
}

async fn request(
    client: &Client,
    token: &str,
    method: Method,
    url: Url,
    body: Option<Value>,
) -> Result<Value> {
    let mut builder = client.request(method, url.clone()).bearer_auth(token);
    if let Some(body) = body {
        builder = builder.json(&body);
    }

    let response = builder
        .send()
        .await
        .with_context(|| format!("failed to call GIW desktop daemon at {url}"))?;
    let (status, text) = read_bounded_response(response).await?;
    let value: Value = serde_json::from_str(&text)
        .with_context(|| format!("daemon returned non-JSON response ({status})"))?;

    if !status.is_success() {
        bail!("daemon request failed with {status}: {value}");
    }

    if let Some(version) = value.get("protocol_version").and_then(Value::as_u64)
        && version > PROTOCOL_VERSION
    {
        bail!(
            "daemon protocol version {version} is newer than this CLI supports ({PROTOCOL_VERSION}); update giw-desktop-cli"
        );
    }

    return Ok(value);
}

async fn run(args: Args) -> Result<Value> {
    let daemon_base = parse_daemon_base(&args.daemon_url)?;
    let token_file = token_path(args.token_file)?;
    let token = read_token(&token_file)?;
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .build()
        .context("failed to construct HTTP client")?;

    let (method, path, body) = match args.command {
        Command::Status => (Method::GET, "/v1/status".to_string(), None),
        Command::Reconcile => (Method::POST, "/v1/reconcile".to_string(), None),
        Command::Processes => (Method::GET, "/v1/processes".to_string(), None),
        Command::Process { action, name } => {
            if !valid_service_name(&name) {
                bail!(
                    "process name must match the manifest service-name contract [a-z0-9][a-z0-9._-]*"
                );
            }
            let action = match action {
                ProcessAction::Start => "start",
                ProcessAction::Stop => "stop",
                ProcessAction::Restart => "restart",
            };
            (Method::POST, format!("/v1/processes/{name}/{action}"), None)
        }
        Command::Tunnel { action } => {
            let action = match action {
                TunnelAction::Start => "start",
                TunnelAction::Stop => "stop",
            };
            (Method::POST, format!("/v1/tunnel/{action}"), None)
        }
        Command::KeepAwake { state } => {
            let enabled = matches!(state, Toggle::On);
            (
                Method::POST,
                "/v1/power/keep-awake".to_string(),
                Some(json!({"enabled": enabled})),
            )
        }
        Command::Update => (Method::POST, "/v1/updates/apply".to_string(), None),
    };

    return request(
        &client,
        &token,
        method,
        endpoint(&daemon_base, &path)?,
        body,
    )
    .await;
}

#[tokio::main]
async fn main() -> Result<()> {
    let value = run(Args::parse()).await?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_url_requires_literal_loopback_origin() {
        assert!(parse_daemon_base("http://127.0.0.1:18440").is_ok());
        assert!(parse_daemon_base("http://[::1]:18440").is_ok());
        assert!(parse_daemon_base("http://localhost:18440").is_err());
        assert!(parse_daemon_base("https://127.0.0.1:18440").is_err());
        assert!(parse_daemon_base("http://127.0.0.1:18440/path").is_err());
        assert!(parse_daemon_base("http://127.0.0.1:18440/?q=1").is_err());
        assert!(parse_daemon_base("http://127.0.0.1:18440@evil.example").is_err());
    }

    #[test]
    fn endpoint_join_is_stable() {
        let base = parse_daemon_base("http://127.0.0.1:18440").expect("loopback base");
        assert_eq!(
            endpoint(&base, "/v1/status")
                .expect("endpoint")
                .as_str(),
            "http://127.0.0.1:18440/v1/status"
        );
    }

    #[test]
    fn service_name_is_one_bounded_url_safe_segment() {
        assert!(valid_service_name("build-server"));
        assert!(valid_service_name("worker_1"));
        assert!(!valid_service_name(""));
        assert!(!valid_service_name("../worker"));
        assert!(!valid_service_name("worker?admin=true"));
        assert!(!valid_service_name("Worker"));
    }
}
