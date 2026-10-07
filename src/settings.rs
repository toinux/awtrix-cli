//! Device settings administration. Mutations are validated locally before HTTP.
use clap::Subcommand;
use serde_json::{json, Map, Value};
use std::{
    thread,
    time::{Duration, Instant},
};

#[derive(Subcommand)]
pub enum Command {
    /// Read system settings (secret-like keys are redacted recursively).
    Get,
    /// Patch a JSON object of known, typed fields.
    Patch {
        #[arg(long)]
        values: String,
    },
    /// Read current display configuration.
    DisplayGet,
    /// Patch display configuration.
    DisplayPatch {
        #[arg(long)]
        values: String,
    },
    /// Set the device brightness level (0..=255).
    Brightness { level: u8 },
    /// Turn display power on or off.
    Power { state: PowerState },
    /// Read system configuration with secret redaction.
    SystemGet,
    /// Request reboot once and optionally wait for HTTP to return.
    Reboot {
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u64).range(0..=300))]
        wait_secs: u64,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum PowerState {
    On,
    Off,
}

pub fn validate(command: &Command) -> Result<(), String> {
    match command {
        Command::Patch { values } | Command::DisplayPatch { values } => {
            let value: Value = serde_json::from_str(values)
                .map_err(|_| "--values must be valid JSON".to_owned())?;
            if !value.is_object() || value.as_object().is_some_and(Map::is_empty) {
                return Err("--values must be a non-empty JSON object".into());
            }
            validate_patch(&value, matches!(command, Command::DisplayPatch { .. }))
        }
        _ => Ok(()),
    }
}

fn validate_patch(value: &Value, display: bool) -> Result<(), String> {
    // Fields remain allow-listed to avoid forwarding mistyped or secret/network writes.
    let schema: &[(&str, &str)] = if display {
        &[
            ("brightness", "number"),
            ("display", "boolean"),
            ("flip", "boolean"),
            ("rotation", "number"),
            ("effect", "string"),
        ]
    } else {
        &[
            ("brightness", "number"),
            ("display", "boolean"),
            ("flip", "boolean"),
            ("rotation", "number"),
            ("effect", "string"),
            ("sleep", "number"),
            ("screensaver", "boolean"),
            ("timezone", "string"),
            ("language", "string"),
        ]
    };
    for (key, field) in value.as_object().expect("object checked above") {
        let kind = schema
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, kind)| *kind)
            .ok_or_else(|| format!("unsupported setting field '{key}'"))?;
        let valid = match kind {
            "number" => field.as_number().is_some(),
            "boolean" => field.is_boolean(),
            _ => field.is_string(),
        };
        if !valid {
            return Err(format!("setting '{key}' must be {kind}"));
        }
        if key == "brightness" && field.as_f64().is_none_or(|v| !(0.0..=255.0).contains(&v)) {
            return Err("brightness must be between 0 and 255".into());
        }
    }
    Ok(())
}

pub fn run(command: &Command, api: &crate::ApiClient, _timeout_ms: u64) -> crate::CliResult<Value> {
    match command {
        Command::Get => api.get("/api/v1/settings"),
        Command::SystemGet => api.get("/api/v1/system").map(|mut v| {
            redact(&mut v);
            v
        }),
        Command::Patch { values } => mutate_patch(api, "/api/v1/settings", values, false),
        Command::DisplayGet => api.get("/api/v1/display"),
        Command::DisplayPatch { values } => mutate_patch(api, "/api/v1/display", values, true),
        Command::Brightness { level } => api.mutate(
            reqwest::Method::PATCH,
            "/api/v1/display",
            &json!({"brightness":level}),
        ),
        Command::Power { state } => api.mutate(
            reqwest::Method::PATCH,
            "/api/v1/display",
            &json!({"display":matches!(state, PowerState::On)}),
        ),
        Command::Reboot { wait_secs } => {
            // One POST only: if its response is lost, the outcome is ambiguous and not retried.
            api.mutate(reqwest::Method::POST, "/api/v1/reboot", &Value::Null)?;
            if *wait_secs == 0 {
                return Ok(json!({"accepted":true,"online_observed":false,"waited":false}));
            }
            let deadline = Instant::now() + Duration::from_secs(*wait_secs);
            while Instant::now() < deadline {
                if api.get("/api/v1/device").is_ok() {
                    return Ok(json!({"accepted":true,"online_observed":true,"waited":true}));
                }
                thread::sleep(
                    Duration::from_millis(200)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Ok(
                json!({"accepted":true,"online_observed":false,"waited":true,"error":{"code":"REBOOT_TIMEOUT","message":"reboot was accepted but device did not return before the wait deadline"}}),
            )
        }
    }
}

fn mutate_patch(
    api: &crate::ApiClient,
    path: &str,
    text: &str,
    _display: bool,
) -> crate::CliResult<Value> {
    let body: Value = serde_json::from_str(text)
        .map_err(|_| ("ARGUMENT", "--values must be valid JSON".into()))?;
    api.mutate(reqwest::Method::PATCH, path, &body)
}

fn redact(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                let lower = key.to_ascii_lowercase();
                if [
                    "password",
                    "passwd",
                    "secret",
                    "token",
                    "credential",
                    "api_key",
                    "apikey",
                    "private_key",
                ]
                .iter()
                .any(|needle| lower.contains(needle))
                {
                    *item = Value::String("[REDACTED]".into());
                } else {
                    redact(item);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact),
        _ => {}
    }
}

pub fn describe(topic: &str) -> Value {
    let (route, example) = match topic {
        "settings patch" => (
            "PATCH /api/v1/settings",
            "awtrix settings patch --values '{\"brightness\":80}'",
        ),
        "settings display-get" | "settings display" => {
            ("GET /api/v1/display", "awtrix settings display-get")
        }
        "settings display-patch" => (
            "PATCH /api/v1/display",
            "awtrix settings display-patch --values '{\"brightness\":80}'",
        ),
        "settings brightness" => (
            "PATCH /api/v1/display with brightness",
            "awtrix settings brightness 80",
        ),
        "settings power" => (
            "PATCH /api/v1/display with display boolean",
            "awtrix settings power off",
        ),
        "settings system-get" => ("GET /api/v1/system", "awtrix settings system-get"),
        "settings reboot" => (
            "POST /api/v1/reboot once",
            "awtrix settings reboot --wait-secs 30",
        ),
        _ => ("GET /api/v1/settings", "awtrix settings get"),
    };
    json!({"command":topic,"parameters":{"--target":"HTTP base URL","--json":"compact JSON output","--fields":"comma-separated result fields","--values":"non-empty JSON object with typed allow-listed settings fields","level":"brightness integer 0..255","state":"on|off","--wait-secs":"optional online-observation deadline, 0..300"},"inputs":[route],"outputs":["device JSON; system-get recursively redacts secret-like keys","reboot: accepted, online_observed, waited; acceptance is not observed availability"],"examples":[example],"prerequisites":["AWTRIX NG route and advertised device capability when the route is variant-specific"],"offline_reference_variant":"ESP32"})
}
