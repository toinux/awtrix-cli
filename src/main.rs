use clap::{error::ErrorKind, Parser, Subcommand};
use serde_json::{json, Map, Value};
use std::{process::ExitCode, time::Duration};

mod logs;
mod profiles;
mod screen;
mod scripts;

#[derive(Parser)]
#[command(name = "awtrix", version, about = "AWTRIX NG device CLI")]
pub(crate) struct Cli {
    #[arg(long, global = true, env = "AWTRIX_URL")]
    pub(crate) target: Option<String>,
    #[arg(long, global = true, env = "AWTRIX_USERNAME")]
    pub(crate) username: Option<String>,
    #[arg(long, global = true, env = "AWTRIX_PASSWORD")]
    pub(crate) password: Option<String>,
    #[arg(long, global = true, default_value_t = 3000)]
    pub(crate) timeout: u64,
    #[arg(long, global = true)]
    pub(crate) json: bool,
    #[arg(long, global = true, value_delimiter = ',')]
    fields: Vec<String>,
    #[arg(long, global = true, env = "AWTRIX_PROFILE")]
    profile: Option<String>,
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
    Profile {
        #[command(subcommand)]
        action: profiles::ProfileCommand,
    },
    Script {
        #[command(subcommand)]
        action: scripts::Command,
    },
    Logs {
        #[command(subcommand)]
        action: logs::Command,
    },
    Screen {
        #[command(subcommand)]
        action: screen::Command,
    },
}

#[derive(Subcommand)]
enum DeviceCommand {
    Identity,
    State,
    Capabilities,
    Diagnose,
}

pub(crate) type CliResult<T> = Result<T, (&'static str, String)>;

pub(crate) struct ApiClient {
    base: String,
    client: reqwest::blocking::Client,
    username: Option<String>,
    password: Option<String>,
    timeout: Duration,
}

impl ApiClient {
    fn new(
        cli: &Cli,
        selected_target: Option<&str>,
        username: Option<String>,
        password: Option<String>,
    ) -> CliResult<Self> {
        let target = selected_target.ok_or((
            "TARGET_REQUIRED",
            "provide --target URL or AWTRIX_URL".into(),
        ))?;
        let parsed = reqwest::Url::parse(target)
            .map_err(|_| ("ARGUMENT", "target must be an absolute HTTP URL".into()))?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
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
            username,
            password,
            timeout: Duration::from_millis(cli.timeout.max(1)),
        })
    }

    pub(crate) fn get(&self, path: &str) -> CliResult<Value> {
        self.get_with_timeout(path, self.timeout)
    }

    pub(crate) fn get_with_timeout(&self, path: &str, timeout: Duration) -> CliResult<Value> {
        let mut request = self
            .client
            .get(format!("{}{path}", self.base))
            .timeout(timeout.min(self.timeout));
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

    pub(crate) fn raw_get(&self, path: &str) -> CliResult<String> {
        let response = self
            .authorized(self.client.get(format!("{}{path}", self.base)))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    ("TIMEOUT", "request timed out".into())
                } else {
                    ("TRANSPORT", "could not reach the HTTP target".into())
                }
            })?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err((
                "AUTHENTICATION",
                "device rejected HTTP Basic credentials".into(),
            ));
        }
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(("HTTP", "script not found".into()));
        }
        if !response.status().is_success() {
            return Err((
                "HTTP",
                format!("device returned HTTP {}", response.status().as_u16()),
            ));
        }
        String::from_utf8(
            response
                .bytes()
                .map_err(|_| ("INVALID_RESPONSE", "could not read script source".into()))?
                .to_vec(),
        )
        .map_err(|_| ("INVALID_RESPONSE", "script source is not UTF-8".into()))
    }
    pub(crate) fn raw_bytes_get(&self, path: &str, max_bytes: u64) -> CliResult<Vec<u8>> {
        let response = self
            .authorized(self.client.get(format!("{}{path}", self.base)))
            .send()
            .map_err(|error| {
                if error.is_timeout() {
                    ("TIMEOUT", "request timed out".into())
                } else {
                    ("TRANSPORT", "could not reach the HTTP target".into())
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
        if response
            .content_length()
            .is_some_and(|length| length > max_bytes)
        {
            return Err((
                "RESPONSE_TOO_LARGE",
                "framebuffer response exceeds the safe size limit".into(),
            ));
        }
        use std::io::Read;
        let mut limited = response.take(max_bytes.saturating_add(1));
        let mut bytes = Vec::new();
        limited.read_to_end(&mut bytes).map_err(|_| {
            (
                "TRANSPORT",
                "failed while reading framebuffer response".into(),
            )
        })?;
        if bytes.len() as u64 > max_bytes {
            return Err((
                "RESPONSE_TOO_LARGE",
                "framebuffer response exceeds the safe size limit".into(),
            ));
        }
        Ok(bytes)
    }
    pub(crate) fn raw_put(&self, path: &str, source: &str) -> CliResult<Value> {
        let response = self
            .authorized(
                self.client
                    .put(format!("{}{path}", self.base))
                    .header(reqwest::header::CONTENT_TYPE, "text/plain")
                    .body(source.to_owned()),
            )
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    ("TIMEOUT", "request timed out".into())
                } else {
                    (
                        "TRANSPORT",
                        "write result is unknown after transport failure".into(),
                    )
                }
            })?;
        self.script_response(response)
    }
    /// Send an authenticated JSON mutation and decode the official JSON response.
    pub(crate) fn mutate(
        &self,
        method: reqwest::Method,
        path: &str,
        body: &Value,
    ) -> CliResult<Value> {
        let mut request = self.client.request(method, format!("{}{path}", self.base));
        if let Some(username) = &self.username {
            request = request.basic_auth(username, self.password.as_deref());
        }
        if !body.is_null() {
            request = request.json(body);
        }
        let response = request.send().map_err(|e| {
            if e.is_timeout() {
                ("TIMEOUT", "request timed out".into())
            } else {
                (
                    "TRANSPORT",
                    "mutation result is unknown after transport failure".into(),
                )
            }
        })?;
        self.script_response(response)
    }
    pub(crate) fn conditional_put(
        &self,
        name: &str,
        expected: &Value,
        source: &str,
    ) -> CliResult<Value> {
        let response = self
            .authorized(
                self.client
                    .put(format!("{}/api/v1/apps/script-update/{name}", self.base))
                    .json(&json!({"expected_source":expected,"source":source})),
            )
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    ("TIMEOUT", "write result is unknown after timeout".into())
                } else {
                    (
                        "TRANSPORT",
                        "write result is unknown after transport failure".into(),
                    )
                }
            })?;
        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err((
                "CONFLICT",
                "remote source differs from the supplied reference; no overwrite performed".into(),
            ));
        }
        self.script_response(response)
    }
    fn authorized(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        if let Some(username) = &self.username {
            request.basic_auth(username, self.password.as_deref())
        } else {
            request
        }
    }
    fn script_response(&self, response: reqwest::blocking::Response) -> CliResult<Value> {
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
            .json()
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
        Ok(_)
            if matches!(
                cli.command,
                Command::Logs {
                    action: logs::Command::Follow { .. }
                }
            ) =>
        {
            ExitCode::SUCCESS
        }
        Ok(value) if value.get("source").is_some() && cli.fields.is_empty() && !cli.json => {
            print!("{}", value["source"].as_str().unwrap_or_default());
            ExitCode::SUCCESS
        }
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
            let streaming_logs = matches!(
                cli.command,
                Command::Logs {
                    action: logs::Command::Follow { .. }
                }
            );
            emit_error(
                code,
                &message,
                cli.json
                    && !(streaming_logs && cli.fields.is_empty() && logs::is_stream_error(code)),
            );
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
        "INCOMPATIBLE" | "PROTECTION_UNAVAILABLE" => 6,
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
    if matches!(
        cli.command,
        Command::Logs {
            action: logs::Command::Follow { .. }
        }
    ) && !cli.fields.is_empty()
    {
        return Err((
            "ARGUMENT",
            "--fields is not supported with streaming logs follow".into(),
        ));
    }
    if let Command::Profile { action } = &cli.command {
        return profiles::run(action);
    }
    if let Command::Describe { topic } = &cli.command {
        return describe(cli, topic);
    }
    let explicit_target =
        std::env::args().any(|arg| arg == "--target" || arg.starts_with("--target="));
    let resolved = profiles::resolve(
        if explicit_target {
            cli.target.as_deref()
        } else {
            None
        },
        cli.profile.as_deref(),
        cli.username.as_deref(),
        cli.password.as_deref(),
    )?;
    let api = ApiClient::new(
        cli,
        resolved.target.as_deref(),
        resolved.username.clone(),
        resolved.password.clone(),
    )?;
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
                json!({"reachable":true,"variant":variant,"version":version.get("version").cloned().unwrap_or(Value::Null),"state":state,"capabilities":capabilities,"target":resolved.target,"target_origin":resolved.origin}),
            )
        }
        Command::Describe { .. } => unreachable!(),
        Command::Profile { .. } => unreachable!(),
        Command::Script { action } => scripts::run(action, &api),
        Command::Logs { action } => logs::run(action, &api, cli.json),
        Command::Screen { action } => screen::run(action, &api),
    }
}

fn describe(cli: &Cli, topic: &str) -> CliResult<Value> {
    if topic == "profile" || topic.starts_with("profile ") {
        return profiles::describe(
            topic
                .strip_prefix("profile ")
                .filter(|_| topic != "profile"),
        );
    }
    let mut result = match topic {
        "device" => {
            json!({"command":"device","parameters":{"--target":"HTTP base URL; required for device commands, optional for describe","--username":"HTTP Basic username","--password":"HTTP Basic password","--timeout":"bounded request timeout in milliseconds (default 3000)","--json":"emit compact JSON independent of terminal","--fields":"comma-separated top-level result fields"},"inputs":["AWTRIX NG HTTP device"],"outputs":["identity: variant, version, identity, state","state: /api/v1/device JSON","capabilities: /api/v1/capabilities JSON","diagnose: reachability, variant, version, state and capabilities"],"examples":["awtrix --target http://awtrix.local device diagnose","awtrix --json --target http://awtrix.local device identity"],"prerequisites":["HTTP(S) AWTRIX NG endpoint; Basic credentials when configured"],"offline_reference_variant":"ESP32"})
        }
        "profiles" => profiles::describe(None)?,
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
        "script" | "scripts" => {
            json!({"command":"script","parameters":{"name":"[A-Za-z0-9_-]{1,32}","--source":"raw Berry source","--file":"UTF-8 Berry source file","--expected-source":"exact original remote source for atomic update","--create":"create only when absent","--force":"explicit unconditional raw PUT; no conflict protection"},"inputs":["raw Berry source"],"outputs":["get: raw source stdout or JSON source field","deploy: source_saved plus independently verified start status; otherwise execution_state unknown"],"examples":["awtrix script get demo","awtrix --json script get demo","awtrix script deploy demo --file main.be --expected-source OLD","awtrix script deploy demo --file main.be --create","awtrix script deploy demo --file main.be --force"],"prerequisites":["AWTRIX NG script route; atomic update when scriptUpdates capability is present; start confirmation requires system/app state"],"offline_reference_variant":"ESP32"})
        }
        "script state" | "script enable" | "script disable" | "script delete"
        | "script config-get" | "script config-put" | "script data" => {
            let action = topic.strip_prefix("script ").unwrap_or("state");
            let (route, output, example) = match action {
                "state" => ("GET /api/v1/apps", "scripts with observed enabled/inLoop/present and available error message/line/hook", "awtrix script state"),
                "enable" => ("PUT /api/v1/apps/{name}/enabled (bare JSON true)", "name, enabled, accepted, device_result, runtime_success_guaranteed=false", "awtrix script enable demo"),
                "disable" => ("PUT /api/v1/apps/{name}/enabled (bare JSON false)", "name, enabled, accepted, device_result, runtime_success_guaranteed=false", "awtrix script disable demo"),
                "delete" => ("DELETE /api/v1/apps/{name}", "name, deleted, device_result", "awtrix script delete demo"),
                "config-get" => ("GET /api/v1/apps/{name}/config", "declared settings fields and warnings", "awtrix script config-get demo"),
                "config-put" => ("PATCH /api/v1/apps/{name}/config", "accepted, device_result; saves restart init()/setup() and do not promise future runtime success", "awtrix script config-put demo --values '{\"rate\":2}'"),
                _ => ("GET /api/v1/apps/{name}/data", "persisted store values", "awtrix script data demo"),
            };
            command_description(&format!("script {action}"), route, output, example)
        }
        "logs" | "logs follow" | "logs read" => logs::describe(topic)?,
        "screen" => screen::describe(),
        _ => return Err(("ARGUMENT", format!("unknown description topic '{topic}'"))),
    };
    if let Some(target) = &cli.target {
        // Descriptions are usable offline; a supplied target explicitly requests live capability refinement.
        let resolved = profiles::resolve(
            Some(target),
            cli.profile.as_deref(),
            cli.username.as_deref(),
            cli.password.as_deref(),
        )?;
        let capabilities = ApiClient::new(cli, Some(target), resolved.username, resolved.password)?
            .get("/api/v1/capabilities")?;
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
