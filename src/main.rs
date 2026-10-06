use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::{process::ExitCode, time::Duration};

#[derive(Parser)]
#[command(name = "awtrix", version, about = "AWTRIX NG device CLI")]
struct Cli {
    #[arg(long, global = true, env = "AWTRIX_URL")]
    target: Option<String>,
    #[arg(long, global = true, env = "AWTRIX_USERNAME")]
    username: Option<String>,
    #[arg(long, global = true, env = "AWTRIX_PASSWORD")]
    password: Option<String>,
    #[arg(long, global = true, default_value_t = 3000)]
    timeout: u64,
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true, value_delimiter = ',')]
    fields: Vec<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Device {
        #[command(subcommand)]
        action: DeviceCommand,
    },
    Describe {
        #[arg(default_value = "device")]
        topic: String,
    },
}

#[derive(Subcommand)]
enum DeviceCommand {
    Identity,
    State,
    Capabilities,
    Diagnose,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(v) => {
            println!(
                "{}",
                render(v, cli.json, &cli.fields).unwrap_or_else(|e| {
                    eprintln!("{}", e);
                    std::process::exit(2)
                })
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            let (code, status) = e;
            eprintln!(
                "{}",
                if cli.json {
                    json!({"error":{"code":code,"message":status}}).to_string()
                } else {
                    format!("{}: {}", code, status)
                }
            );
            ExitCode::from(if code == "AUTHENTICATION" {
                3
            } else if code == "TIMEOUT" {
                4
            } else if code == "HTTP" {
                5
            } else if code == "INCOMPATIBLE" {
                6
            } else {
                1
            })
        }
    }
}

fn run(cli: &Cli) -> Result<Value, (&'static str, String)> {
    if let Command::Describe { topic } = &cli.command {
        if topic != "device" {
            return Err(("ARGUMENT", format!("unknown description topic {topic}")));
        }
        return Ok(
            json!({"command":"device","parameters":{"--target":"HTTP base URL (required)","--username":"HTTP Basic username","--password":"HTTP Basic password","--timeout":"request timeout in milliseconds","--json":"compact JSON output","--fields":"comma-separated top-level fields"},"inputs":["AWTRIX NG HTTP device"],"outputs":["identity","state","capabilities","diagnosis"],"examples":["awtrix --target http://awtrix.local device diagnose"],"prerequisites":["explicit HTTP target"],"offline_reference_variant":"ESP32"}),
        );
    }
    let target = cli.target.as_deref().ok_or((
        "TARGET_REQUIRED",
        "provide --target URL or AWTRIX_URL".into(),
    ))?;
    let base = target.trim_end_matches('/');
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(cli.timeout.max(1)))
        .build()
        .map_err(|e| ("TRANSPORT", e.to_string()))?;
    let request = |path: &str| {
        let mut req = client.get(format!("{base}{path}"));
        if let Some(user) = &cli.username {
            req = req.basic_auth(user, cli.password.as_deref());
        }
        let response = req.send().map_err(|e| {
            if e.is_timeout() {
                ("TIMEOUT", "request timed out".to_string())
            } else {
                ("TRANSPORT", e.to_string())
            }
        })?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err((
                "AUTHENTICATION",
                "device rejected HTTP Basic credentials".into(),
            ));
        }
        if !response.status().is_success() {
            return Err((
                "HTTP",
                format!("device returned HTTP {}", response.status().as_u16()),
            ));
        }
        response
            .json::<Value>()
            .map_err(|_| ("INVALID_RESPONSE", "device returned invalid JSON".into()))
    };
    let value = match &cli.command {
        Command::Device {
            action: DeviceCommand::Identity,
        } => {
            let system = request("/api/v1/system").unwrap_or_else(|_| json!({}));
            let version = request("/api/v1/version")?;
            let state = request("/api/v1/device")?;
            let variant = detect_variant(&system, &state);
            json!({"variant":variant,"version":version.get("version").cloned().unwrap_or(Value::Null),"identity":system,"state":state})
        }
        Command::Device {
            action: DeviceCommand::State,
        } => request("/api/v1/device")?,
        Command::Device {
            action: DeviceCommand::Capabilities,
        } => request("/api/v1/capabilities")?,
        Command::Device {
            action: DeviceCommand::Diagnose,
        } => {
            let version = request("/api/v1/version")?;
            let state = request("/api/v1/device")?;
            let caps = request("/api/v1/capabilities")?;
            let system = request("/api/v1/system").unwrap_or_else(|_| json!({}));
            json!({"reachable":true,"variant":detect_variant(&system,&state),"version":version.get("version").cloned().unwrap_or(Value::Null),"state":state,"capabilities":caps})
        }
        Command::Describe { .. } => unreachable!(),
    };
    Ok(value)
}

fn detect_variant(system: &Value, state: &Value) -> &'static str {
    let text = format!("{} {}", system, state).to_lowercase();
    if text.contains("tc002") {
        "TC002"
    } else if text.contains("esp32-s3") || text.contains("esp32_s3") {
        "ESP32-S3"
    } else if text.contains("esp32") {
        "ESP32"
    } else {
        "unknown"
    }
}

fn render(mut value: Value, machine: bool, fields: &[String]) -> Result<String, String> {
    if !fields.is_empty() {
        let object = value
            .as_object_mut()
            .ok_or("field selection requires an object result")?;
        let mut selected = serde_json::Map::new();
        for field in fields {
            let item = object
                .get(field)
                .ok_or_else(|| format!("unknown field '{field}'"))?;
            selected.insert(field.clone(), item.clone());
        }
        value = Value::Object(selected);
    }
    if machine {
        serde_json::to_string(&value).map_err(|e| e.to_string())
    } else {
        serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
    }
}
