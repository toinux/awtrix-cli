use clap::{error::ErrorKind, Parser, Subcommand};
use serde_json::{json, Map, Value};
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

type CliResult<T> = Result<T, (&'static str, String)>;

struct ApiClient {
    base: String,
    client: reqwest::blocking::Client,
    username: Option<String>,
    password: Option<String>,
}

impl ApiClient {
    fn new(cli: &Cli) -> CliResult<Self> {
        let target = cli.target.as_deref().ok_or((
            "TARGET_REQUIRED",
            "provide --target URL or AWTRIX_URL".into(),
        ))?;
        let parsed = reqwest::Url::parse(target)
            .map_err(|_| ("ARGUMENT", "target must be an absolute HTTP URL".into()))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(("ARGUMENT", "target must be an absolute HTTP URL".into()));
        }
        let base = target.trim_end_matches('/').to_string();
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(cli.timeout.max(1)))
            .build()
            .map_err(|_| ("TRANSPORT", "could not create HTTP client".into()))?;
        Ok(Self {
            base,
            client,
            username: cli.username.clone(),
            password: cli.password.clone(),
        })
    }

    fn get(&self, path: &str) -> CliResult<Value> {
        let mut request = self.client.get(format!("{}{path}", self.base));
        if let Some(username) = &self.username {
            request = request.basic_auth(username, self.password.as_deref());
        }
        let response = request.send().map_err(|error| {
            if error.is_timeout() {
                ("TIMEOUT", "request timed out".to_string())
            } else {
                ("TRANSPORT", "could not reach the HTTP target".to_string())
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
    }
}

fn main() -> ExitCode {
    let machine_requested = std::env::args().any(|argument| argument == "--json");
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if machine_requested
                && error.kind() != ErrorKind::DisplayHelp
                && error.kind() != ErrorKind::DisplayVersion
            {
                eprintln!("{}", error);
                println!(
                    "{}",
                    json!({"error":{"code":"ARGUMENT","message":"invalid command-line arguments"}})
                );
                return ExitCode::from(2);
            }
            let _ = error.print();
            return if error.kind() == ErrorKind::DisplayHelp
                || error.kind() == ErrorKind::DisplayVersion
            {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            };
        }
    };
    match run(&cli) {
        Ok(value) => match render(value, &cli.fields, cli.json) {
            Ok(value) => {
                println!("{value}");
                ExitCode::SUCCESS
            }
            Err(message) => {
                emit_error("UNKNOWN_FIELD", &message, cli.json);
                ExitCode::from(2)
            }
        },
        Err((code, message)) => {
            emit_error(code, &message, cli.json);
            ExitCode::from(exit_code(code))
        }
    }
}

fn exit_code(code: &str) -> u8 {
    match code {
        "ARGUMENT" | "TARGET_REQUIRED" | "UNKNOWN_FIELD" => 2,
        "AUTHENTICATION" => 3,
        "TIMEOUT" => 4,
        "HTTP" => 5,
        "INCOMPATIBLE" => 6,
        _ => 1,
    }
}

fn emit_error(code: &str, message: &str, machine: bool) {
    if machine {
        println!("{}", json!({"error":{"code":code,"message":message}}));
        eprintln!("awtrix: {code}: {message}");
    } else {
        eprintln!("awtrix: {code}: {message}");
    }
}

fn run(cli: &Cli) -> CliResult<Value> {
    if let Command::Describe { topic } = &cli.command {
        return describe(cli, topic);
    }
    let api = ApiClient::new(cli)?;
    match &cli.command {
        Command::Device {
            action: DeviceCommand::State,
        } => api.get("/api/v1/device"),
        Command::Device {
            action: DeviceCommand::Capabilities,
        } => api.get("/api/v1/capabilities"),
        Command::Device {
            action: DeviceCommand::Identity,
        } => {
            let state = api.get("/api/v1/device")?;
            let variant = detect_variant(&state);
            let version = api.get("/api/v1/version")?;
            let identity = json!({"boardType":state.get("boardType"),"soc":state.get("soc")});
            Ok(
                json!({"variant":variant,"version":version.get("version").cloned().unwrap_or(Value::Null),"identity":identity,"state":state}),
            )
        }
        Command::Device {
            action: DeviceCommand::Diagnose,
        } => {
            let state = api.get("/api/v1/device")?;
            let variant = detect_variant(&state);
            let version = api.get("/api/v1/version")?;
            let capabilities = api.get("/api/v1/capabilities")?;
            Ok(
                json!({"reachable":true,"variant":variant,"version":version.get("version").cloned().unwrap_or(Value::Null),"state":state,"capabilities":capabilities}),
            )
        }
        Command::Describe { .. } => unreachable!(),
    }
}

fn describe(cli: &Cli, topic: &str) -> CliResult<Value> {
    let mut result = match topic {
        "device" => {
            json!({"command":"device","parameters":{"--target":"HTTP base URL; required for device commands, optional for describe","--username":"HTTP Basic username","--password":"HTTP Basic password","--timeout":"bounded request timeout in milliseconds (default 3000)","--json":"emit compact JSON independent of terminal","--fields":"comma-separated top-level result fields"},"inputs":["AWTRIX NG HTTP device"],"outputs":["identity: variant, version, identity, state","state: /api/v1/device JSON","capabilities: /api/v1/capabilities JSON","diagnose: reachability, variant, version, state and capabilities"],"examples":["awtrix --target http://awtrix.local device diagnose","awtrix --json --target http://awtrix.local device identity"],"prerequisites":["HTTP(S) AWTRIX NG endpoint; Basic credentials when configured"],"offline_reference_variant":"ESP32"})
        }
        "identity" | "device identity" => command_description(
            "device identity",
            "GET /api/v1/device and /api/v1/version",
            "variant, firmware version, identity and state",
            "awtrix --target http://awtrix.local device identity",
        ),
        "state" | "device state" => command_description(
            "device state",
            "GET /api/v1/device",
            "device state JSON",
            "awtrix --target http://awtrix.local device state",
        ),
        "capabilities" | "device capabilities" => command_description(
            "device capabilities",
            "GET /api/v1/capabilities",
            "device capability names",
            "awtrix --target http://awtrix.local device capabilities",
        ),
        "diagnose" | "device diagnose" => command_description(
            "device diagnose",
            "GET /api/v1/device, /api/v1/version and /api/v1/capabilities",
            "reachability, variant, version, state, capabilities",
            "awtrix --target http://awtrix.local device diagnose",
        ),
        _ => return Err(("ARGUMENT", format!("unknown description topic '{topic}'"))),
    };
    if let Some(target) = &cli.target {
        // Descriptions are usable offline; a supplied target explicitly requests live capability refinement.
        let capabilities = ApiClient::new(cli)?.get("/api/v1/capabilities")?;
        result["target"] = json!(target);
        result["connected_capabilities"] = capabilities;
        result["capability_source"] = json!("connected-device");
    } else {
        result["capability_source"] = json!("offline-ESP32-reference");
    }
    Ok(result)
}

fn command_description(name: &str, inputs: &str, outputs: &str, example: &str) -> Value {
    json!({"command":name,"parameters":{"--target":"HTTP base URL (required for execution; optional for describe)","--username":"HTTP Basic username","--password":"HTTP Basic password","--timeout":"request timeout in milliseconds","--json":"compact JSON result","--fields":"comma-separated top-level fields"},"inputs":[inputs],"outputs":[outputs],"examples":[example],"prerequisites":["AWTRIX NG HTTP endpoint","Basic credentials if authentication is enabled"],"offline_reference_variant":"ESP32"})
}

// AWTRIX NG's device-state schema exposes boardType and soc; inspect only these identity fields.
fn detect_variant(state: &Value) -> &'static str {
    let board_type = state
        .get("boardType")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let soc = state
        .get("soc")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if board_type == "tc002" {
        "TC002"
    } else if board_type == "awtrixng" && soc == "esp32s3" {
        "ESP32-S3"
    } else if board_type == "awtrixng" && soc == "esp32" {
        "ESP32"
    } else {
        "unknown"
    }
}

fn render(mut value: Value, fields: &[String], machine: bool) -> Result<String, String> {
    if !fields.is_empty() {
        let object = value
            .as_object_mut()
            .ok_or("field selection requires an object result")?;
        let mut selected = Map::new();
        for field in fields {
            let item = object
                .get(field)
                .ok_or_else(|| format!("unknown field '{field}'"))?;
            selected.insert(field.clone(), item.clone());
        }
        value = Value::Object(selected);
    }
    if machine {
        serde_json::to_string(&value).map_err(|error| error.to_string())
    } else {
        serde_json::to_string_pretty(&value).map_err(|error| error.to_string())
    }
}
