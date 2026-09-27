#![forbid(unsafe_code)]

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use reqwest::{Client, Method};
use serde_json::{Value, json};

const PROTOCOL_VERSION: u64 = 1;
const DEFAULT_DAEMON_URL: &str = "http://127.0.0.1:18440";

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

fn read_token(path: &PathBuf) -> Result<String> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read daemon token at {}", path.display()))?;
    let token = raw.trim();
    if token.len() < 32 || token.chars().any(char::is_whitespace) {
        bail!("daemon token at {} is malformed", path.display());
    }
    return Ok(token.to_string());
}

fn endpoint(base: &str, path: &str) -> String {
    return format!("{}{}", base.trim_end_matches('/'), path);
}

async fn request(
    client: &Client,
    token: &str,
    method: Method,
    url: String,
    body: Option<Value>,
) -> Result<Value> {
    let mut builder = client.request(method, &url).bearer_auth(token);
    if let Some(body) = body {
        builder = builder.json(&body);
    }

    let response = builder
        .send()
        .await
        .with_context(|| format!("failed to call GIW desktop daemon at {url}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .context("failed to read daemon response")?;
    let value: Value = serde_json::from_str(&text)
        .with_context(|| format!("daemon returned non-JSON response ({status}): {text}"))?;

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
    if !args.daemon_url.starts_with("http://127.0.0.1:")
        && !args.daemon_url.starts_with("http://localhost:")
        && !args.daemon_url.starts_with("http://[::1]:")
    {
        bail!(
            "--daemon-url must target loopback HTTP; got {:?}",
            args.daemon_url
        );
    }

    let token_file = token_path(args.token_file)?;
    let token = read_token(&token_file)?;
    let client = Client::builder()
        .build()
        .context("failed to construct HTTP client")?;

    let (method, path, body) = match args.command {
        Command::Status => (Method::GET, "/v1/status".to_string(), None),
        Command::Reconcile => (Method::POST, "/v1/reconcile".to_string(), None),
        Command::Processes => (Method::GET, "/v1/processes".to_string(), None),
        Command::Process { action, name } => {
            if name.trim().is_empty() || name.contains('/') {
                bail!("process name must be a non-empty manifest service name without '/'");
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
        endpoint(&args.daemon_url, &path),
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
    fn endpoint_join_is_stable() {
        assert_eq!(
            endpoint("http://127.0.0.1:18440/", "/v1/status"),
            "http://127.0.0.1:18440/v1/status"
        );
    }
}
