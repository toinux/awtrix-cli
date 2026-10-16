#![recursion_limit = "256"]

use clap::{error::ErrorKind, Parser, Subcommand};
use serde_json::{json, Map, Value};
use std::process::ExitCode;

mod apps;
mod headless;
mod logs;
mod minify;
mod notifications;
mod persistence;
mod profiles;
mod project;
mod release_notice;
mod resources;
mod screen;
mod scripts;
mod settings;
mod tests;
mod transport;
mod updater;

pub(crate) use transport::ApiClient;

#[derive(Parser)]
#[command(
    name = "awtrix-cli",
    version,
    about = "AWTRIX NG device CLI",
    after_help = "Discovery (no skill required):\n  awtrix-cli describe --list --json     List available documentation topics\n  awtrix-cli describe apps create --offline --json\n                                       Read semantics, schemas and examples\n  awtrix-cli apps create --help         Read exact execution syntax and flags"
)]
pub(crate) struct Cli {
    /// Device HTTP(S) URL: --target > AWTRIX_URL > explicit/env profile > project profile (project commands only) > default profile.
    #[arg(long, global = true, env = "AWTRIX_URL")]
    pub(crate) target: Option<String>,
    /// HTTP Basic username: flag > AWTRIX_USERNAME > credentials of the selected profile only.
    #[arg(long, global = true, env = "AWTRIX_USERNAME")]
    pub(crate) username: Option<String>,
    /// HTTP Basic password: flag > AWTRIX_PASSWORD > credentials of the selected profile only.
    #[arg(long, global = true, env = "AWTRIX_PASSWORD")]
    pub(crate) password: Option<String>,
    /// Per-request device timeout in milliseconds (minimum 1); update uses its own 30-second bound.
    #[arg(long, global = true, default_value_t = 3000)]
    pub(crate) timeout: u64,
    /// Emit compact JSON; logs follow emits JSONL. Without --json, source reads are raw unless projected; other results are pretty JSON.
    #[arg(long, global = true)]
    pub(crate) json: bool,
    /// Project top-level result fields AFTER execution; failure does not undo effects. Rejected for logs follow; errors are not projected.
    #[arg(long, global = true, value_delimiter = ',')]
    fields: Vec<String>,
    /// Select a named profile after --target/AWTRIX_URL; flag > AWTRIX_PROFILE > project profile (project commands only) > default.
    #[arg(long, global = true, env = "AWTRIX_PROFILE")]
    profile: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read device identity, state, capabilities, or connection diagnostics.
    Device {
        #[command(subcommand)]
        action: DeviceCommand,
    },
    /// Explain command semantics, payload schemas, outputs and examples.
    #[command(
        after_help = "Examples:\n  awtrix-cli --json describe --list\n  awtrix-cli --json describe apps create --offline\n  awtrix-cli --json describe \"script verify\"\n  awtrix-cli --target http://awtrix.local --json describe device\n\nUse --help on the execution command for exact syntax and flags. Descriptions\ndo not execute the documented operation. Without --offline, --target or\nAWTRIX_URL requests GET /api/v1/capabilities and request failures are returned.\nNamed/default profiles alone do not connect; profile topics, update and --list\nare always local. Connected capabilities are authoritative; the offline ESP32\nreference is not proof of support on your device. --json emits compact JSON;\notherwise pretty JSON."
    )]
    Describe {
        /// Documentation topic, quoted or separate words; defaults to device.
        #[arg(default_value = "device", num_args = 1..)]
        topic: Vec<String>,
        /// List supported topics and discovery guidance without contacting a device.
        #[arg(long)]
        list: bool,
        /// Use local documentation even when --target or AWTRIX_URL is set.
        #[arg(long)]
        offline: bool,
    },
    /// Manage local personal device profiles and the default selection.
    #[command(
        after_help = "Configuration: AWTRIX_CONFIG overrides the file path. Unix: $HOME/.config/awtrix-cli/config.json. Windows: %APPDATA%/awtrix-cli/config.json (fallback %USERPROFILE%/AppData/Roaming). Profile operations are local; credentials are omitted from results."
    )]
    Profile {
        #[command(subcommand)]
        action: profiles::ProfileCommand,
    },
    /// Read, deploy, manage, and observe persistent Berry scripts.
    Script {
        #[command(subcommand)]
        action: scripts::Command,
    },
    /// Minify a local Berry source file into a sibling .min.ax file.
    #[command(
        after_help = "Class/function and variable renaming are enabled by default. Member rewriting supports literal self.field only: accesses through aliases or other receivers are not rewritten. Use --no-variables locally to disable member/local renaming for such source. Returns output (path) and bytes (UTF-8 output size); leaves the input unchanged."
    )]
    Minify {
        /// UTF-8 Berry source file; output is <file stem>.min.ax beside it.
        file: std::path::PathBuf,
        /// Rename top-level classes/functions (enabled by default).
        #[arg(long, conflicts_with = "no_classes")]
        classes: bool,
        /// Disable top-level class/function renaming.
        #[arg(long = "no-classes", conflicts_with = "classes")]
        no_classes: bool,
        /// Rename locals, parameters, import aliases, and supported self fields (enabled by default).
        #[arg(long, conflicts_with = "no_variables")]
        variables: bool,
        /// Disable variable/member renaming; mitigation for fields accessed through aliases.
        #[arg(long = "no-variables", conflicts_with = "variables")]
        no_variables: bool,
        /// Replace an existing .min.ax output file.
        #[arg(long)]
        force: bool,
    },
    /// Read bounded device log history once or follow new lines.
    Logs {
        #[command(subcommand)]
        action: logs::Command,
    },
    /// Capture the device framebuffer as a local PNG artifact.
    Screen {
        #[command(subcommand)]
        action: screen::Command,
    },
    /// Manage device files and reusable Berry modules.
    Resources {
        #[command(subcommand)]
        action: resources::Command,
    },
    /// Start, stop, or inspect a local AWTRIX NG headless runtime.
    Headless {
        #[command(subcommand)]
        action: headless::Command,
    },
    /// Validate and deploy awtrix.toml projects, or reconcile device resources.
    Project {
        #[command(subcommand)]
        action: project::Command,
    },
    /// Run the declarative tests in a project manifest.
    Test {
        #[command(subcommand)]
        action: tests::Command,
    },
    /// Create, update, and remove temporary pushed apps.
    Apps {
        #[command(subcommand)]
        action: apps::Command,
    },
    /// Send or dismiss a temporary notification.
    Notify {
        #[command(subcommand)]
        action: notifications::Command,
    },
    /// Read and modify device/display settings, and request reboot.
    Settings {
        #[command(subcommand)]
        action: settings::Command,
    },
    /// Replace the current CLI executable with the latest stable checksum-verified release.
    #[command(
        after_help = "Replaces the currently running executable; restart awtrix-cli to use the new version. Returns updated (boolean) and message (string). Uses its own 30-second per-request HTTP bound; ignores device --timeout, target, profile, and Basic credentials. --force reinstalls the same version but never downgrades a newer installation."
    )]
    Update {
        /// Reinstall the same version; never downgrade a newer installation.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum DeviceCommand {
    /// Read board/SoC identity, detected variant, firmware version, and state.
    Identity,
    /// Read the device state from GET /api/v1/device.
    State,
    /// Read the connected device's advertised capabilities.
    Capabilities,
    /// Read reachability, variant, version, state, capabilities, resolved target and target_origin; request errors fail the command.
    Diagnose,
}

pub(crate) type CliResult<T> = Result<T, (&'static str, String)>;

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
    let notice = release_notice::notice();
    if let Some(notice) = notice {
        eprintln!("awtrix: {notice}");
    }
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
        Ok(value) if value.get("passed").and_then(Value::as_bool) == Some(false) => {
            let rendered = render(value, &cli.fields, cli.json).unwrap_or_else(|_| "{}".into());
            println!("{rendered}");
            eprintln!("awtrix: TEST_FAILED: one or more declarative assertions failed");
            ExitCode::from(1)
        }
        Ok(value)
            if value.pointer("/error/code").and_then(Value::as_str) == Some("REBOOT_TIMEOUT") =>
        {
            let rendered = render(value, &cli.fields, cli.json).unwrap_or_else(|_| "{}".into());
            println!("{rendered}");
            eprintln!("awtrix: REBOOT_TIMEOUT: reboot was accepted but offline-to-online recovery was not observed");
            ExitCode::from(4)
        }
        Ok(value)
            if value
                .get("runtime_error")
                .is_some_and(|error| !error.is_null())
                || value
                    .get("not_available")
                    .is_some_and(|reason| !reason.is_null())
                || value.get("verification").is_some_and(|report| {
                    report
                        .get("runtime_error")
                        .is_some_and(|error| !error.is_null())
                        || report
                            .get("not_available")
                            .is_some_and(|reason| !reason.is_null())
                }) =>
        {
            let report = value.get("verification").unwrap_or(&value);
            if report
                .get("runtime_error")
                .is_some_and(|error| !error.is_null())
                || report
                    .get("not_available")
                    .is_some_and(|reason| !reason.is_null())
            {
                let diagnostic_code = if report
                    .get("runtime_error")
                    .is_some_and(|error| !error.is_null())
                {
                    "BERRY_ERROR"
                } else {
                    "SCRIPT_NOT_RUNNING"
                };
                let rendered = render(value, &cli.fields, cli.json).unwrap_or_else(|_| "{}".into());
                println!("{rendered}");
                eprintln!("awtrix: {diagnostic_code}: verification did not establish a healthy running script");
                return ExitCode::from(1);
            }
            let diagnostic_code = if value
                .get("runtime_error")
                .is_some_and(|error| !error.is_null())
            {
                "BERRY_ERROR"
            } else {
                "SCRIPT_NOT_RUNNING"
            };
            let rendered = render(value, &cli.fields, cli.json).unwrap_or_else(|_| "{}".into());
            println!("{rendered}");
            eprintln!("awtrix: {diagnostic_code}: verification did not establish a healthy running script");
            ExitCode::from(1)
        }
        Ok(value) if value.get("__berry_error").and_then(Value::as_bool) == Some(true) => {
            println!(
                "{}",
                json!({"error":{"code":"BERRY_ERROR","message":value["message"],"device_result":value["device_result"],"name":value["name"],"runtime_success_guaranteed":false}})
            );
            eprintln!(
                "awtrix: BERRY_ERROR: {}",
                value["message"]
                    .as_str()
                    .unwrap_or("script restart failed after data was saved")
            );
            ExitCode::from(1)
        }
        Ok(mut value) if value.get("__capture_failure").and_then(Value::as_bool) == Some(true) => {
            value.as_object_mut().unwrap().remove("__capture_failure");
            let report = &value["capture_collection"];
            let diagnostic = report["error"]["message"]
                .as_str()
                .unwrap_or("capture collection did not complete")
                .to_owned();
            let code = report["error"]["code"]
                .as_str()
                .unwrap_or("CAPTURE_ERROR")
                .to_owned();
            let rendered = render(value, &cli.fields, cli.json).unwrap_or_else(|_| "{}".into());
            println!("{rendered}");
            eprintln!("awtrix: {code}: {diagnostic}");
            ExitCode::from(1)
        }
        Ok(value) if value.get("__raw_stdout").and_then(Value::as_bool) == Some(true) => {
            if cli.json {
                let mut value = value;
                value.as_object_mut().unwrap().remove("__raw_stdout");
                match render(value, &cli.fields, true) {
                    Ok(rendered) => println!("{rendered}"),
                    Err(message) => {
                        emit_error("UNKNOWN_FIELD", &message, true);
                        return ExitCode::from(2);
                    }
                }
            } else if cli.fields.is_empty() {
                print!("{}", value["text"].as_str().unwrap_or_default());
            } else {
                let mut value = value;
                value.as_object_mut().unwrap().remove("__raw_stdout");
                match render(value, &cli.fields, false) {
                    Ok(rendered) => println!("{rendered}"),
                    Err(message) => {
                        emit_error("UNKNOWN_FIELD", &message, false);
                        return ExitCode::from(2);
                    }
                }
            }
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
        if code == "PROJECT_DEPLOY_FAILED" {
            if let Some((text, report)) = message.rsplit_once("; deployment report: ") {
                if let Ok(report) = serde_json::from_str::<Value>(report) {
                    println!(
                        "{}",
                        json!({"error":{"code":code,"message":text},"report":report})
                    );
                    eprintln!("awtrix: {code}: {text}");
                    return;
                }
            }
        }
        println!("{}", json!({"error":{"code":code,"message":message}}));
        eprintln!("awtrix: {code}: {message}");
    } else {
        eprintln!("awtrix: {code}: {message}");
    }
}

fn run(cli: &Cli) -> CliResult<Value> {
    if let Command::Update { force } = cli.command {
        updater::run(force).map(|(updated, message)| json!({"updated":updated,"message":message}))
    } else if let Command::Minify {
        file,
        classes,
        no_classes,
        variables,
        no_variables,
        force,
    } = &cli.command
    {
        minify::run(
            file,
            *classes || !*no_classes,
            *variables || !*no_variables,
            *force,
        )
    } else {
        run_command(cli)
    }
}

fn run_command(cli: &Cli) -> CliResult<Value> {
    if matches!(
        &cli.command,
        Command::Resources {
            action: resources::Command::Files {
                action: resources::FileCommand::Download { .. }
            }
        }
    ) {
        return resources::unsupported_download();
    }
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
    if let Command::Describe {
        topic,
        list,
        offline,
    } = &cli.command
    {
        if *list {
            return Ok(description_catalog());
        }
        let topic = topic.join(" ");
        let topic = topic.split_whitespace().collect::<Vec<_>>().join(" ");
        return describe(cli, &topic, *offline);
    }
    if let Command::Headless { action } = &cli.command {
        return headless::run(action);
    }
    if let Command::Project { action } = &cli.command {
        return project::run(action, cli);
    }
    if let Command::Test { action } = &cli.command {
        return tests::run(action, cli);
    }
    if let Command::Apps { action } = &cli.command {
        apps::validate(action).map_err(|message| ("ARGUMENT", message))?;
    }
    if let Command::Notify { action } = &cli.command {
        notifications::validate(action).map_err(|message| ("ARGUMENT", message))?;
    }
    if let Command::Settings { action } = &cli.command {
        settings::validate(action).map_err(|message| ("ARGUMENT", message))?;
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
        Command::Apps { action } => apps::run(action, &api),
        Command::Notify { action } => notifications::run(action, &api),
        Command::Settings { action } => settings::run(action, &api, cli.timeout),
        Command::Update { .. } => unreachable!(),
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
        Command::Minify { .. } => unreachable!(),
        Command::Logs { action } => logs::run(action, &api, cli.json),
        Command::Screen { action } => screen::run(action, &api),
        Command::Resources { action } => resources::run(action, &api),
        Command::Headless { .. } => unreachable!(),
        Command::Project { .. } => unreachable!(),
        Command::Test { .. } => unreachable!(),
    }
}

const DESCRIPTION_TOPICS: &[&str] = &[
    "device",
    "device identity",
    "device state",
    "device capabilities",
    "device diagnose",
    "profile",
    "profile add",
    "profile update",
    "profile list",
    "profile show",
    "profile set-default",
    "profile delete",
    "script",
    "script get",
    "script deploy",
    "script re-minify",
    "script verify",
    "script state",
    "script enable",
    "script disable",
    "script delete",
    "script config-get",
    "script config-put",
    "script data",
    "script data-patch",
    "script shared",
    "minify",
    "logs",
    "logs read",
    "logs follow",
    "screen",
    "screen capture",
    "resources",
    "resources files",
    "resources files list",
    "resources files upload",
    "resources files delete",
    "resources files download",
    "resources modules",
    "resources modules list",
    "resources modules get",
    "resources modules deploy",
    "resources modules delete",
    "headless",
    "headless start",
    "headless stop",
    "headless status",
    "apps",
    "apps create",
    "apps update",
    "apps delete",
    "apps list",
    "apps active-get",
    "apps select",
    "apps order-get",
    "apps order-set",
    "notify",
    "notify send",
    "notify delete",
    "notify delete-active",
    "settings",
    "settings get",
    "settings patch",
    "settings display-get",
    "settings display-patch",
    "settings brightness",
    "settings power",
    "settings system-get",
    "settings reboot",
    "project",
    "project init",
    "project validate",
    "project deploy",
    "project prune",
    "project reconcile",
    "test",
    "test project",
    "update",
];

fn description_catalog() -> Value {
    json!({
        "command": "describe",
        "topics": DESCRIPTION_TOPICS,
        "default_topic": "device",
        "usage": "awtrix-cli [--json] describe <topic words...> [--offline]",
        "purpose": "Read command semantics, payload schemas, outputs, prerequisites and examples without executing the documented operation. Documentation coverage varies by topic; some operations share a family description.",
        "syntax_help": "Use awtrix-cli <command words...> --help for exact execution syntax and flags.",
        "connection": {
            "default": "Local ESP32 reference unless --target or AWTRIX_URL is supplied. Named/default profiles alone do not connect.",
            "connected": "GET /api/v1/capabilities; connected capabilities are authoritative and request failures propagate. This does not execute the documented operation or validate its payload.",
            "offline": "--offline ignores --target and AWTRIX_URL for descriptions. Offline documentation is not proof of device support.",
            "always_local": ["--list", "profile topics", "update"]
        },
        "output": "JSON object on stdout (pretty by default, compact with --json). --fields selects top-level fields of this description, not of the documented operation's result.",
        "examples": [
            "awtrix-cli --json describe --list",
            "awtrix-cli --json describe apps create --offline",
            "awtrix-cli --json describe \"script verify\" --offline",
            "awtrix-cli --target http://awtrix.local --json describe device",
            "awtrix-cli apps create --help"
        ]
    })
}

fn describe(cli: &Cli, topic: &str, offline: bool) -> CliResult<Value> {
    let always_local = topic == "update"
        || topic == "profiles"
        || topic == "profile"
        || topic.starts_with("profile ");
    let mut result = if topic == "profile" || topic.starts_with("profile ") {
        profiles::describe(
            topic
                .strip_prefix("profile ")
                .filter(|_| topic != "profile"),
        )
        .map_err(|(code, message)| {
            (
                code,
                format!("{message}; use 'awtrix-cli describe --list' to list supported topics"),
            )
        })?
    } else {
        match topic {
        "update" => json!({
            "command":"update",
            "parameters":{"--force":"reinstall the same version; never downgrade a newer installation"},
            "inputs":["latest stable GitHub release metadata, host executable asset, SHA256SUMS"],
            "outputs":{"updated":"boolean: whether the executable was replaced","message":"status string; restart after replacement"},
            "output_fields":["updated","message"],
            "examples":["awtrix-cli --json update","awtrix-cli update --force"],
            "prerequisites":["supported host release asset; writable current executable location; release network access"],
            "semantics":["Replaces the currently running CLI executable after checksum validation", "Uses its own 30-second per-request HTTP bound; ignores device --timeout, target, profile and Basic credentials", "--fields projection happens after replacement; a projection error does not undo it", "describe update is always local and never invokes the updater"]
        }),
        "device" => {
            let mut description = command_description("device", "AWTRIX NG HTTP device", "identity: variant, version, identity, state; state: /api/v1/device JSON; capabilities: /api/v1/capabilities JSON; diagnose: reachable, variant, version, state, capabilities, target, target_origin", "awtrix-cli --target http://awtrix.local device diagnose");
            description["outputs"] = json!(["identity: variant, version, identity, state", "state: /api/v1/device JSON", "capabilities: /api/v1/capabilities JSON", "diagnose: reachable, variant, version, state, capabilities, target, target_origin"]);
            description["examples"] = json!(["awtrix-cli --target http://awtrix.local device diagnose", "awtrix-cli --json --target http://awtrix.local device identity"]);
            description["errors"] = json!("Request failures return an error and nonzero exit, not reachable=false. Diagnose reports reachable=true only after all three reads succeed; no credentials are returned.");
            description
        }
        "profiles" => profiles::describe(None)?,
        "identity" | "device identity" => command_description(
            "device identity",
            "GET /api/v1/device and /api/v1/version",
            "variant, firmware version, identity and state",
            "awtrix-cli --target http://awtrix.local device identity",
        ),
        "state" | "device state" => command_description(
            "device state",
            "GET /api/v1/device",
            "device state JSON",
            "awtrix-cli --target http://awtrix.local device state",
        ),
        "capabilities" | "device capabilities" => command_description(
            "device capabilities",
            "GET /api/v1/capabilities",
            "device capability JSON",
            "awtrix-cli --target http://awtrix.local device capabilities",
        ),
        "diagnose" | "device diagnose" => command_description(
            "device diagnose",
            "GET /api/v1/device, /api/v1/version and /api/v1/capabilities",
            "reachable=true, variant, version, state, capabilities, target, target_origin (command-line, environment, profile, or default-profile); request errors fail with nonzero exit rather than returning reachable=false",
            "awtrix-cli --target http://awtrix.local device diagnose",
        ),
        "script" | "scripts" | "script get" | "script deploy" | "script re-minify" => {
            json!({"command":"script","parameters":{"name":"[A-Za-z0-9_-]{1,32}","--source":"raw Berry source","--file":"UTF-8 Berry source file (.ax for scripts; .be for modules)","--expected-source":"exact original remote source for atomic update","--create":"create only when absent","--force":"explicit unconditional raw PUT; no concurrency guarantee","--minify":"opt-in minification of deployed payload; source file/reference remain unchanged","--no-backup":"script re-minify only: explicitly skip local <name>.bak.ax backup","--verify-secs":"optional bounded observation after successful deploy, 1..3600","--capture-dir":"opt-in named captures triggered by new exact target-script markers; cannot combine with --verify-secs","--capture-count":"positive PNG count (default 1), requires --capture-dir","--capture-timeout-ms":"capture observation bound 1..3600000 milliseconds (default 10000), requires --capture-dir"},"inputs":["raw Berry source"],"outputs":["get: raw source stdout or JSON source field","deploy: source_saved plus independently verified start status; optional verification or capture_collection {attempt,directory,start_verified,start_evidence,expected,obtained,captures,stop_reason,error}; otherwise execution_state unknown","re-minify: exact-source backup path and conditional update outcome; runtime state unknown"],"examples":["awtrix-cli script get demo","awtrix-cli --json script get demo","awtrix-cli script re-minify demo","awtrix-cli script re-minify demo --no-backup","awtrix-cli script deploy demo --file main.ax --expected-source OLD --minify","awtrix-cli script deploy demo --file main.ax --create","awtrix-cli script deploy demo --file main.ax --force","awtrix-cli script deploy demo --file main.ax --expected-source OLD --capture-dir ./captures","awtrix-cli script deploy demo --file main.ax --expected-source OLD --capture-dir ./captures --capture-count 3"],"prerequisites":["AWTRIX NG script route; atomic update when scriptUpdates capability is present; explicit start check requires system/app state; capture also requires active-app, log and framebuffer routes"],"limitations":["A shared observation deadline bounds selection, logs and captures; partial PNGs remain in the report","Marker must be [script:<name>] [CAPTURE] <label>, optionally preceded by HH:MM:SS; label is 1..64 ASCII letters, digits, underscore or hyphen","Timeout and collection errors return nonzero while preserving deployment evidence and partial capture paths; log batches cannot reconstruct missing states","Selection changes the active app once; no rotation/notification restoration is attempted","Logs and framebuffer are not synchronized; marker attribution, lost-log detection and visual correctness are not guaranteed"],"offline_reference_variant":"ESP32"})
        }
        "script verify" => {
            json!({"command":"script verify","parameters":{"name":"script name [A-Za-z0-9_-]{1,32}","--duration-secs":"bounded observation window, 1..3600 (default 10)","--interval-ms":"log polling interval, 1..60000 (default 500)","--after":"initial log cursor (default 0)","--capture":"optional PNG output path"},"inputs":["AWTRIX NG system, app inventory, cursor logs and optional framebuffer"],"outputs":{"source_saved":"not_requested for existing-script verification","start_verified":"boolean","observed_window":{"complete":"true only if the full requested duration elapsed without early termination","elapsed_ms":"integer","early_termination_reason":"null or scripting_disabled, script_absent, script_disabled, error_state_unavailable, berry_error, script_not_running, collection_error"},"not_available":"null, reason string, or diagnostic object with code/message/phase","runtime_error":"null or reported Berry error","runtime_state":"last observed app state or null","logs":{"after":"integer","next":"integer","lines":"string array","history_limit":34,"exhaustive":false},"capture":"optional screen artifact summary","runtime_success_guaranteed":false},"examples":["awtrix-cli --json script verify demo --duration-secs 30 --interval-ms 500","awtrix-cli script verify demo --capture observed.png"],"prerequisites":["existing enabled AWTRIX NG script","bounded device log history"],"limitations":["No error observed is not proof of general correctness","log history is bounded to 34 lines"]})
        }
        "script data-patch" => json!({"command":"script data-patch","route":"PATCH /api/v1/apps/{name}/data","parameters":{"NAME":"[A-Za-z0-9_-]{1,32}","--values":"required JSON object; {} is valid"},"inputs":["one PATCH JSON object; nested objects/arrays and null values are passed unchanged"],"outputs":{"name":"string","accepted":"boolean","device_result":"firmware response JSON","restart":"script restarts and initialization may alter injected values","runtime_success_guaranteed":false},"schema":{"values":"JSON object; omitted keys are unchanged; null deletes a key; firmware rejects @config keys"},"examples":["awtrix-cli --json script data-patch demo --values '{\"count\":8,\"remove\":null}'"],"prerequisites":["AWTRIX NG PATCH /api/v1/apps/{name}/data endpoint"],"limitations":["partial patch, not replacement/reset","script restart reruns initialization; runtime success is not guaranteed","HTTP 200 Berry error means data was saved but restart failed; no rollback or retry","a lost response leaves write outcome unknown"]}),
        "script shared" => json!({"command":"script shared","route":"GET /api/v1/scripts/shared","inputs":["current shared script publications"],"outputs":{"publications":[{"owner":"script name","key":"publication key","type":"firmware type","value":"unchanged JSON value","ageMs":"firmware-reported age in milliseconds"}]},"schema":{"publications":"array; empty arrays are valid; values retain their JSON scalar types"},"examples":["awtrix-cli script shared","awtrix-cli --json --fields publications script shared"],"prerequisites":["AWTRIX NG GET /api/v1/scripts/shared endpoint"],"limitations":["read-only point-in-time view, not a private store","publications are temporary and can disappear when the device restarts or their owner is removed/re-registered","ageMs is reported as-is; no filtering, sorting, or stream is provided"]}),
        "script state" | "script enable" | "script disable" | "script delete"
        | "script config-get" | "script config-put" | "script data" => {
            let action = topic.strip_prefix("script ").unwrap_or("state");
            let (route, output, example) = match action {
                "state" => ("GET /api/v1/apps", "scripts with observed enabled/inLoop/present and available error message/line/hook", "awtrix-cli script state"),
                "enable" => ("PUT /api/v1/apps/{name}/enabled (bare JSON true)", "name, enabled, accepted, device_result, runtime_success_guaranteed=false", "awtrix-cli script enable demo"),
                "disable" => ("PUT /api/v1/apps/{name}/enabled (bare JSON false)", "name, enabled, accepted, device_result, runtime_success_guaranteed=false", "awtrix-cli script disable demo"),
                "delete" => ("DELETE /api/v1/apps/{name}", "name, deleted, device_result", "awtrix-cli script delete demo"),
                "config-get" => ("GET /api/v1/apps/{name}/config", "declared settings fields and warnings", "awtrix-cli script config-get demo"),
                "config-put" => ("PATCH /api/v1/apps/{name}/config", "accepted, device_result; saves restart init()/setup() and do not promise future runtime success", "awtrix-cli script config-put demo --values '{\"rate\":2}'"),
                "data-patch" => ("PATCH /api/v1/apps/{name}/data", "name, accepted, device_result, restart, runtime_success_guaranteed=false; HTTP 200 Berry error returns BERRY_ERROR with the saved device_result", "awtrix-cli --json script data-patch demo --values '{\"count\":8,\"remove\":null}'"),
                "data" => ("GET /api/v1/apps/{name}/data", "persisted store values", "awtrix-cli script data demo"),
                _ => ("PATCH /api/v1/apps/{name}/data", "partial store patch; null deletes keys; restarts script; no runtime guarantee; @config keys are rejected by firmware", "awtrix-cli --json script data-patch demo --values '{\"count\":8,\"remove\":null}'"),
            };
            command_description(&format!("script {action}"), route, output, example)
        }
        "minify" => {
            json!({"command":"minify","parameters":{"<file>":"UTF-8 Berry source file","--classes":"rename top-level classes/functions (enabled by default)","--no-classes":"disable class/function renaming","--variables":"rename locals, parameters, import aliases and supported self fields (enabled by default)","--no-variables":"disable variable renaming","--force":"overwrite an existing .min.ax output"},"inputs":["local Berry source"],"outputs":["sibling .min.ax file; original source is unchanged"],"examples":["awtrix-cli minify main.ax","awtrix-cli minify main.ax --no-classes","awtrix-cli minify main.ax --force"],"prerequisites":["UTF-8 Berry source file"]})
        }
        "logs" | "logs follow" | "logs read" => logs::describe(topic)?,
        "screen" | "screen capture" => screen::describe(),
        "resources" | "resources files" | "resources modules" => resources::describe(None),
        "headless" | "headless start" | "headless stop" | "headless status" => {
            headless::describe(topic)
        }
        "apps" | "apps create" | "apps update" | "apps delete" | "apps list"
        | "apps active-get" | "apps select" | "apps order-get" | "apps order-set" => {
            apps::describe(topic)
        }
        "notify" | "notify send" | "notify delete" | "notify delete-active" => {
            notifications::describe(topic)
        }
        "settings"
        | "settings get"
        | "settings patch"
        | "settings display"
        | "settings display-get"
        | "settings display-patch"
        | "settings brightness"
        | "settings power"
        | "settings system-get"
        | "settings reboot" => settings::describe(topic),
        "project" | "project init" | "project validate" | "project deploy" | "project prune"
        | "project reconcile" => project::describe(topic),
        "test" | "test project" => tests::describe(),
        "resources files list"
        | "resources files upload"
        | "resources files delete"
        | "resources files download"
        | "resources modules list"
        | "resources modules get"
        | "resources modules deploy"
        | "resources modules delete" => resources::describe(Some(topic)),
        _ => return Err(("ARGUMENT", format!("unknown description topic '{topic}'; use 'awtrix-cli describe --list' to list supported topics, or 'awtrix-cli <command> --help' for execution syntax"))),
        }
    };
    if matches!(
        topic,
        "script" | "scripts" | "script get" | "script deploy" | "script re-minify" | "minify"
    ) {
        result["limitations"] = json!(["Member rewriting supports literal self.field only; field access through aliases or other receivers is not rewritten and can change behavior when members are renamed", "Local minify --no-variables disables variable/member renaming as a mitigation; script deploy --minify and script re-minify use default renamings and do not expose that switch. Deploy locally minified source without --minify when needed"]);
    }
    if topic == "minify" {
        result["outputs"] = json!([
            "output: sibling <file stem>.min.ax path; original source is unchanged",
            "bytes: UTF-8 byte count of the written minified output"
        ]);
        result["output_fields"] = json!(["output", "bytes"]);
    }
    result["requested_topic"] = json!(topic);
    let help_topic = if topic == "settings display" {
        "settings display-get"
    } else if DESCRIPTION_TOPICS.contains(&topic) {
        topic
    } else {
        result["command"].as_str().unwrap_or(topic)
    };
    result["syntax_help"] = json!(format!("awtrix-cli {help_topic} --help"));
    if let Some(target) = cli.target.as_ref().filter(|_| !offline && !always_local) {
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
    } else if !always_local {
        result["capability_source"] = json!("offline-ESP32-reference");
    }
    Ok(result)
}

fn command_description(name: &str, inputs: &str, outputs: &str, example: &str) -> Value {
    json!({"command":name,"parameters":{"--target":"HTTP(S) URL: --target > AWTRIX_URL > explicit/env profile > project profile (project commands only) > default profile; optional for describe","--profile":"named profile: flag > AWTRIX_PROFILE; used only when no target flag/environment URL wins","--username":"Basic username: flag > AWTRIX_USERNAME > selected profile only","--password":"Basic password: flag > AWTRIX_PASSWORD > selected profile only","--timeout":"per-request device timeout in milliseconds (default 3000, minimum 1)","--json":"compact JSON result","--fields":"comma-separated top-level projection AFTER execution; failure does not undo effects; error envelopes are not projected"},"inputs":[inputs],"outputs":[outputs],"examples":[example],"prerequisites":["AWTRIX NG HTTP endpoint resolved from target, environment or profile","Basic credentials if authentication is enabled"],"offline_reference_variant":"ESP32"})
}

// AWTRIX NG's device-state schema exposes boardType and soc; inspect only these identity fields.
pub(crate) fn detect_variant(state: &Value) -> &'static str {
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
