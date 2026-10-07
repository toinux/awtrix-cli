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
    /// Set device brightness level (0..=255), and optionally auto-brightness.
    Brightness {
        level: u8,
        #[arg(long)]
        auto: Option<bool>,
    },
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
            ("power", "boolean"),
            ("overlay", "overlay"),
            ("overlaySettings", "object"),
        ]
    } else {
        &[("brightness", "integer"), ("autoBrightness", "boolean")]
    };
    let fields = value
        .as_object()
        .ok_or_else(|| "patch values must be a JSON object".to_owned())?;
    for (key, field) in fields {
        let kind = schema
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, kind)| *kind)
            .ok_or_else(|| format!("unsupported setting field '{key}'"))?;
        let valid = match kind {
            "integer" => field.as_i64().is_some_and(|n| (0..=255).contains(&n)),
            "boolean" => field.is_boolean(),
            "object" => field.is_object(),
            "overlay" => field.is_null() || field.as_str().is_some(),
            _ => false,
        };
        if !valid {
            return Err(format!("setting '{key}' must be {kind}"));
        }
        if key == "overlaySettings" {
            let Some(tuning) = field.as_object() else {
                return Err("overlaySettings must be an object".into());
            };
            for (name, setting) in tuning {
                let valid = match name.as_str() {
                    "speed" => setting.as_f64().is_some_and(|n| (0.1..=10.0).contains(&n)),
                    "palette" => setting.is_null() || setting.is_string(),
                    "blend" => setting.is_boolean(),
                    _ => false,
                };
                if !valid {
                    return Err(format!(
                        "overlaySettings.{name} has an unsupported field or value"
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn run(command: &Command, api: &crate::ApiClient, timeout_ms: u64) -> crate::CliResult<Value> {
    match command {
        Command::Get => api.get("/api/v1/settings"),
        Command::SystemGet => api.get("/api/v1/system").map(|mut v| {
            redact(&mut v);
            v
        }),
        Command::Patch { values } => mutate_patch(api, "/api/v1/settings", values, false),
        Command::DisplayGet => api.get("/api/v1/display"),
        Command::DisplayPatch { values } => mutate_patch(api, "/api/v1/display", values, true),
        Command::Brightness { level, auto } => {
            let mut body = json!({"brightness":level});
            if let Some(auto) = auto {
                body["autoBrightness"] = json!(auto);
            }
            api.mutate(reqwest::Method::PATCH, "/api/v1/settings", &body)
                .map_err(map_storage_state)
        }
        Command::Power { state } => api
            .mutate(
                reqwest::Method::PATCH,
                "/api/v1/display",
                &json!({"power":matches!(state, PowerState::On)}),
            )
            .map_err(map_storage_state),
        Command::Reboot { wait_secs } => {
            let deadline = Instant::now() + Duration::from_secs(*wait_secs);
            let request_timeout = Duration::from_millis(timeout_ms.max(1));
            api.get("/api/v1/device")?;
            // One POST only: if its response is lost, the outcome is ambiguous and not retried.
            api.mutate(reqwest::Method::POST, "/api/v1/device/reboot", &Value::Null)
                .map_err(|(code, message)| {
                    if code == "TIMEOUT"
                        || code == "TRANSPORT"
                        || (code == "HTTP" && message.contains("HTTP 5"))
                    {
                        (
                            "OUTCOME_UNKNOWN",
                            format!(
                                "reboot request outcome is unknown; no retry was made ({message})"
                            ),
                        )
                    } else {
                        (code, message)
                    }
                })?;
            if *wait_secs == 0 {
                return Ok(json!({"accepted":true,"online_observed":false,"waited":false}));
            }
            let mut offline_observed = false;
            while Instant::now() < deadline {
                let remaining = deadline.saturating_duration_since(Instant::now());
                match api.get_with_timeout("/api/v1/device", remaining.min(request_timeout)) {
                    Ok(_after) if offline_observed => {
                        return Ok(
                            json!({"accepted":true,"online_observed":true,"waited":true,"offline_observed":offline_observed}),
                        );
                    }
                    Ok(_) => {}
                    Err(("TIMEOUT" | "TRANSPORT", _)) => offline_observed = true,
                    Err(("HTTP", message)) if message.contains("HTTP 503") => {
                        offline_observed = true
                    }
                    Err(_) => {}
                }
                thread::sleep(
                    Duration::from_millis(200)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Ok(
                json!({"accepted":true,"online_observed":false,"offline_observed":offline_observed,"waited":true,"error":{"code":"REBOOT_TIMEOUT","message":"reboot was accepted but a post-reboot device state was not observed before the wait deadline"}}),
            )
        }
    }
}

fn mutate_patch(
    api: &crate::ApiClient,
    path: &str,
    text: &str,
    display: bool,
) -> crate::CliResult<Value> {
    let body: Value = serde_json::from_str(text)
        .map_err(|_| ("ARGUMENT", "--values must be valid JSON".into()))?;
    if display {
        if let Some(overlay) = body.get("overlay").and_then(Value::as_str) {
            let capabilities = api.get("/api/v1/capabilities")?;
            let overlays = capabilities
                .get("overlays")
                .and_then(Value::as_array)
                .ok_or((
                "INCOMPATIBLE",
                "device capabilities do not advertise an overlays list; no display patch was sent"
                    .into(),
            ))?;
            if !overlays.iter().any(|item| {
                item.as_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case(overlay))
            }) {
                return Err(("INCOMPATIBLE", format!("overlay '{overlay}' is not advertised by this device; no display patch was sent")));
            }
        }
        if let Some(palette) = body
            .get("overlaySettings")
            .and_then(|settings| settings.get("palette"))
            .and_then(Value::as_str)
        {
            let capabilities = api.get("/api/v1/capabilities")?;
            let palettes = capabilities
                .get("palettes")
                .and_then(Value::as_array)
                .ok_or((
                "INCOMPATIBLE",
                "device capabilities do not advertise a palettes list; no display patch was sent"
                    .into(),
            ))?;
            if !palettes.iter().any(|item| {
                item.as_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case(palette))
            }) {
                return Err(("INCOMPATIBLE", format!("palette '{palette}' is not advertised by this device; no display patch was sent")));
            }
        }
    }
    api.mutate(reqwest::Method::PATCH, path, &body)
        .map_err(map_storage_state)
}

fn map_storage_state((code, message): (&'static str, String)) -> (&'static str, String) {
    if code == "HTTP_507" {
        (
            "APPLIED_STATE_UNKNOWN",
            "HTTP 507: the device may have applied the change in memory but did not persist it; resulting state is unknown".into(),
        )
    } else {
        (code, message)
    }
}

fn redact(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                let lower = key.to_ascii_lowercase();
                let compact = lower
                    .chars()
                    .filter(char::is_ascii_alphanumeric)
                    .collect::<String>();
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
                    || (compact.ends_with("pass") && compact != "compass")
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
            "awtrix-cli settings patch --values '{\"brightness\":80,\"autoBrightness\":false}'",
        ),
        "settings display-get" | "settings display" => {
            ("GET /api/v1/display", "awtrix-cli settings display-get")
        }
        "settings display-patch" => (
            "PATCH /api/v1/display",
            "awtrix-cli settings display-patch --values '{\"power\":true,\"overlay\":\"rain\"}'",
        ),
        "settings brightness" => (
            "PATCH /api/v1/settings with brightness and optional autoBrightness",
            "awtrix-cli settings brightness 80 --auto false",
        ),
        "settings power" => (
            "PATCH /api/v1/display with power boolean",
            "awtrix-cli settings power off",
        ),
        "settings system-get" => ("GET /api/v1/system", "awtrix-cli settings system-get"),
        "settings reboot" => (
            "POST /api/v1/device/reboot once",
            "awtrix-cli settings reboot --wait-secs 30",
        ),
        _ => ("GET /api/v1/settings", "awtrix-cli settings get"),
    };
    let fields = match topic {
        "settings get" => vec!["settings"],
        "settings patch" => vec!["resulting settings"],
        "settings display" | "settings display-get" => vec![
            "power",
            "overlay",
            "overlaySettings",
            "display runtime state",
        ],
        "settings display-patch" | "settings power" => vec!["accepted", "device_result"],
        "settings brightness" => vec!["resulting settings"],
        "settings system-get" => vec!["redacted system configuration"],
        "settings reboot" => vec![
            "accepted",
            "offline_observed",
            "online_observed",
            "waited",
            "error",
        ],
        _ => vec!["device result"],
    };
    let schemas = match topic {
        "settings patch" => json!({"brightness":"integer 0..255","autoBrightness":"boolean"}),
        "settings brightness" => json!({"level":"integer 0..255","--auto":"optional boolean"}),
        "settings display-patch" => {
            json!({"power":"boolean","overlay":"string|null; string must be in capabilities.overlays","overlaySettings":{"type":"object","properties":{"speed":{"type":"number","minimum":0.1,"maximum":10.0},"palette":{"type":"string|null","capability":"capabilities.palettes"},"blend":{"type":"boolean"}}}})
        }
        "settings power" => json!({"state":"on|off mapped to power:boolean"}),
        "settings reboot" => {
            json!({"--wait-secs":"integer 0..300; success requires observing offline then online"})
        }
        _ => json!({}),
    };
    json!({"command":topic,"parameters":{"--target":"HTTP base URL","--username":"HTTP Basic user; AWTRIX_USERNAME","--password":"HTTP Basic password; AWTRIX_PASSWORD","--timeout":"request timeout milliseconds (default 3000)","--json":"compact JSON output","--fields":"comma-separated top-level result fields","--values":"non-empty JSON object; settings fields brightness integer 0..255 and autoBrightness boolean; display fields power boolean, overlay string|null, overlaySettings object","--auto":"optional boolean for brightness","level":"brightness integer 0..255","state":"on|off","--wait-secs":"optional offline-to-online observation deadline, 0..300"},"inputs":[route],"schemas":schemas,"outputs":fields,"output_fields":fields,"examples":[example],"prerequisites":["AWTRIX NG route; display overlay names are checked against GET /api/v1/capabilities overlays"],"limitations":["A 507 reports application/persistence as unknown unless the specific route contract establishes more","reboot online observation requires an offline response followed by a successful post-reboot response"],"offline_reference_variant":"ESP32"})
}
