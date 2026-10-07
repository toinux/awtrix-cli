//! Temporary pushed applications, separate from persistent Berry scripts.
use clap::Subcommand;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

const MAX_NAME: usize = 32;
const MAX_SMALL_VARIANT_PAYLOAD_BYTES: usize = 8192;
const MAX_TC002_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;

#[derive(Subcommand)]
pub enum Command {
    /// List the device's complete app inventory, including persistent apps and pushed apps.
    List,
    /// Read which app the device reports as active (does not dismiss notifications).
    ActiveGet,
    /// Select an app without claiming it is currently visible.
    Select { name: String },
    /// Read the ordered rotation.
    OrderGet,
    /// Set rotation order and complete disabled-name set (omitted order is preserved).
    OrderSet {
        #[arg(long)]
        order: Option<String>,
        #[arg(long, default_value = "[]")]
        disabled: String,
    },
    /// Create or replace a pushed app (the device route is an upsert, not create-only).
    Create {
        name: String,
        #[arg(long)]
        payload: Option<String>,
        /// Read the JSON payload from a UTF-8 file (useful for large TC002 bodies).
        #[arg(long, conflicts_with = "payload", required_unless_present = "payload")]
        file: Option<PathBuf>,
        #[arg(
            long,
            help = "Allow untyped device-specific fields in this JSON object"
        )]
        raw: bool,
    },
    /// Create or replace a pushed app using the same upsert route; existence is not required.
    Update {
        name: String,
        #[arg(long)]
        payload: Option<String>,
        /// Read the JSON payload from a UTF-8 file (useful for large TC002 bodies).
        #[arg(long, conflicts_with = "payload", required_unless_present = "payload")]
        file: Option<PathBuf>,
        #[arg(
            long,
            help = "Allow untyped device-specific fields in this JSON object"
        )]
        raw: bool,
    },
    /// Remove a pushed app and any indexed children.
    Delete { name: String },
}

pub fn run(command: &Command, api: &crate::ApiClient) -> crate::CliResult<Value> {
    match command {
        Command::List => {
            let inventory = api.get("/api/v1/apps")?;
            let items = inventory
                .as_array()
                .ok_or(("INVALID_RESPONSE", "app inventory must be an array".into()))?;
            let apps: Vec<Value> = items.iter().map(|app| json!({"name":app.get("name"),"origin":app.get("origin"),"enabled":app.get("enabled"),"present":app.get("present"),"in_loop":app.get("inLoop"),"error":app.get("error")})).collect();
            Ok(json!({"apps":apps,"count":apps.len()}))
        }
        Command::ActiveGet => api.get("/api/v1/apps/active"),
        Command::Select { name } => {
            validate_name(name).map_err(|message| ("ARGUMENT", message))?;
            let value = api.mutate(reqwest::Method::PUT, "/api/v1/apps/active", &json!(name))?;
            Ok(
                json!({"name":name,"accepted":true,"visibility":"unknown","device_result":value,"notification_effect":"none_claimed"}),
            )
        }
        Command::OrderGet => api.get("/api/v1/apps"),
        Command::OrderSet { order, disabled } => {
            let disabled: Vec<String> = serde_json::from_str(disabled).map_err(|_| {
                (
                    "ARGUMENT",
                    "--disabled must be a JSON array of app names".into(),
                )
            })?;
            if disabled.iter().any(|name| validate_name(name).is_err()) {
                return Err(("ARGUMENT", "--disabled contains an invalid app name".into()));
            }
            let mut body = json!({"disabled":disabled});
            if let Some(order) = order {
                let order: Vec<String> = serde_json::from_str(order).map_err(|_| {
                    (
                        "ARGUMENT",
                        "--order must be a JSON array of app names".into(),
                    )
                })?;
                if order.iter().any(|name| validate_name(name).is_err()) {
                    return Err(("ARGUMENT", "--order contains an invalid app name".into()));
                }
                body["order"] = json!(order);
            }
            let result = api.mutate(reqwest::Method::PUT, "/api/v1/apps/order", &body)?;
            Ok(
                json!({"accepted":true,"order_included":body.get("order").is_some(),"disabled":body["disabled"],"device_result":result}),
            )
        }
        Command::Create {
            name,
            payload,
            file,
            raw,
        }
        | Command::Update {
            name,
            payload,
            file,
            raw,
        } => {
            let payload = load_payload(payload.as_deref(), file.as_deref())
                .map_err(|message| ("ARGUMENT", message))?;
            let body = parse(name, &payload, *raw).map_err(|message| ("ARGUMENT", message))?;
            validate_route_size(&body, api)?;
            api.mutate(
                reqwest::Method::PUT,
                &format!("/api/v1/apps/pushed/{name}"),
                &body,
            )?;
            Ok(
                json!({"name":name,"operation":if matches!(command, Command::Create{..}) {"create"} else {"update"},"accepted":true,"visibility":"unknown","persistent":false}),
            )
        }
        Command::Delete { name } => {
            validate_name(name).map_err(|message| ("ARGUMENT", message))?;
            api.mutate(
                reqwest::Method::DELETE,
                &format!("/api/v1/apps/{name}"),
                &Value::Null,
            )?;
            Ok(json!({"name":name,"operation":"delete","accepted":true}))
        }
    }
}

pub fn validate(command: &Command) -> Result<(), String> {
    match command {
        Command::List | Command::ActiveGet | Command::OrderGet => Ok(()),
        Command::Select { name } => validate_name(name),
        Command::OrderSet { order, disabled } => {
            let disabled: Vec<String> = serde_json::from_str(disabled)
                .map_err(|_| "--disabled must be a JSON array of app names".to_owned())?;
            if disabled.iter().any(|name| validate_name(name).is_err()) {
                return Err("--disabled contains an invalid app name".into());
            }
            if let Some(order) = order {
                let order: Vec<String> = serde_json::from_str(order)
                    .map_err(|_| "--order must be a JSON array of app names".to_owned())?;
                if order.iter().any(|name| validate_name(name).is_err()) {
                    return Err("--order contains an invalid app name".into());
                }
            }
            Ok(())
        }
        Command::Create {
            name,
            payload,
            file,
            raw,
        }
        | Command::Update {
            name,
            payload,
            file,
            raw,
        } => load_payload(payload.as_deref(), file.as_deref())
            .and_then(|payload| parse(name, &payload, *raw).map(|_| ())),
        Command::Delete { name } => validate_name(name),
    }
}

fn load_payload(payload: Option<&str>, file: Option<&std::path::Path>) -> Result<String, String> {
    match (payload, file) {
        (Some(payload), None) => Ok(payload.to_owned()),
        (None, Some(path)) => {
            fs::read_to_string(path).map_err(|_| "could not read payload file as UTF-8".to_owned())
        }
        _ => Err("provide exactly one of --payload or --file".into()),
    }
}

fn validate_route_size(body: &Value, api: &crate::ApiClient) -> crate::CliResult<()> {
    let encoded = serde_json::to_vec(body)
        .map_err(|_| ("ARGUMENT", "could not encode pushed-app payload".into()))?;
    if encoded.len() > MAX_TC002_PAYLOAD_BYTES {
        return Err((
            "ARGUMENT",
            "pushed-app JSON exceeds the official TC002 2 MiB request-body limit".into(),
        ));
    }
    if encoded.len() <= MAX_SMALL_VARIANT_PAYLOAD_BYTES {
        return Ok(());
    }
    let state = api.get("/api/v1/device")?;
    match crate::detect_variant(&state) {
        "TC002" => Ok(()),
        "ESP32" | "ESP32-S3" => Err(("ARGUMENT", "pushed-app JSON exceeds this variant's official 8192-byte HTTP request-body limit".into())),
        _ => Err(("INCOMPATIBLE", "cannot safely choose the pushed-app body limit because device identity is unknown; payloads over 8192 bytes are not sent".into())),
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    if !name.is_empty()
        && name.len() <= MAX_NAME
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        Ok(())
    } else {
        Err("name must match [A-Za-z0-9_-]{1,32}".into())
    }
}

fn parse(name: &str, input: &str, raw: bool) -> Result<Value, String> {
    validate_name(name)?;
    let value: Value =
        serde_json::from_str(input).map_err(|_| "payload must be valid JSON".to_owned())?;
    let object = value
        .as_object()
        .filter(|map| !map.is_empty())
        .ok_or("payload must be a non-empty JSON object")?;
    if let Some((key, _)) = object.iter().find(|(key, _)| field_kind(key).is_none()) {
        if !raw {
            return Err(format!("unknown pushed-app field '{key}'; pass --raw to explicitly forward device-specific fields"));
        }
    }
    for (key, value) in object {
        if field_kind(key).is_some_and(|kind| !kind.accepts(value)) {
            return Err(format!("{key} has an invalid JSON type or value"));
        }
    }
    Ok(Value::Object(object.clone()))
}

#[derive(Clone, Copy)]
pub(crate) enum FieldKind {
    Text,
    String,
    TextCase,
    TextAlign,
    IconMode,
    LifetimeExpiry,
    Font,
    Color,
    Integer,
    NonNegativeInteger,
    IconGap,
    Float(f64, f64),
    Boolean,
    Scroll,
    Icons,
    Charts,
    Palette,
    Draw,
}

pub(crate) fn notification_field_kind(key: &str) -> Option<FieldKind> {
    field_kind(key)
}

impl FieldKind {
    pub(crate) fn accepts(self, value: &Value) -> bool {
        match self {
            Self::Text => value.is_string() || value.is_array(),
            Self::String | Self::Font => value.is_string(),
            Self::TextCase => value
                .as_str()
                .is_some_and(|v| matches!(v, "inherit" | "upper" | "asTyped")),
            Self::TextAlign => value
                .as_str()
                .is_some_and(|v| matches!(v, "start" | "center" | "end")),
            Self::IconMode => value
                .as_str()
                .is_some_and(|v| matches!(v, "fixed" | "pushOnce" | "push")),
            Self::LifetimeExpiry => value
                .as_str()
                .is_some_and(|v| matches!(v, "remove" | "mark")),
            Self::Color => value.is_string() || value.as_i64().is_some() || value.is_array(),
            Self::Integer => value.as_i64().is_some(),
            Self::NonNegativeInteger => value.as_i64().is_some_and(|v| v >= 0),
            Self::IconGap => value.as_i64().is_some_and(|v| (0..=128).contains(&v)),
            Self::Float(min, max) => value.as_f64().is_some_and(|v| (min..=max).contains(&v)),
            Self::Boolean => value.is_boolean(),
            Self::Scroll => accepts_scroll(value),
            Self::Icons => accepts_icons(value),
            Self::Charts => value
                .as_array()
                .is_some_and(|a| a.len() <= 16 && a.iter().all(Value::is_i64)),
            Self::Palette => value.is_string() || value.as_array().is_some_and(|a| a.len() <= 16),
            Self::Draw => value.as_array().is_some_and(|a| !a.is_empty()),
        }
    }
}

/// The complete 35-key pushed-app schema shared by ESP32, ESP32-S3 and TC002.
/// Unknown keys have no kind and therefore require explicit `--raw` forwarding.
fn field_kind(key: &str) -> Option<FieldKind> {
    use FieldKind::*;
    Some(match key {
        "text" => Text,
        "icon" | "effect" | "overlay" => String,
        "textCase" => TextCase,
        "textAlign" => TextAlign,
        "iconMode" => IconMode,
        "lifetimeExpiry" => LifetimeExpiry,
        "font" => Font,
        "textColor" | "backgroundColor" | "chartColor" | "progressColor" | "progressTrackColor" => {
            Color
        }
        "textBlinkMs" | "textFadeMs" => NonNegativeInteger,
        "textOffsetX" | "iconOffsetX" | "durationMs" | "lifetimeMs" | "repeat" | "progress"
        | "paletteSpan" => Integer,
        "iconGap" => IconGap,
        "effectSpeed" => Float(0.1, 10.0),
        "paletteSpeed" => Float(0.0, 10.0),
        "textInFront" | "chartAutoscale" | "paletteBlend" => Boolean,
        "scroll" => Scroll,
        "icons" => Icons,
        "barChart" | "lineChart" => Charts,
        "palette" => Palette,
        "draw" => Draw,
        _ => return None,
    })
}

fn accepts_scroll(value: &Value) -> bool {
    if value.is_string() {
        return true;
    }
    let Some(object) = value.as_object() else {
        return false;
    };
    object.iter().all(|(key, value)| match key.as_str() {
        "mode" => value
            .as_str()
            .is_some_and(|v| matches!(v, "static" | "wrap" | "loop" | "bounce")),
        "direction" => value
            .as_str()
            .is_some_and(|v| matches!(v, "left" | "right")),
        "entry" => value
            .as_str()
            .is_some_and(|v| matches!(v, "inline" | "offscreen")),
        "whenFits" => value
            .as_str()
            .is_some_and(|v| matches!(v, "static" | "scroll")),
        "speed" | "gap" | "holdMs" => value.as_i64().is_some_and(|n| n >= 0),
        _ => false,
    })
}

fn accepts_icons(value: &Value) -> bool {
    value.as_array().is_some_and(|icons| {
        icons.len() <= 4
            && icons.iter().all(|icon| {
                let Some(icon) = icon.as_object() else {
                    return false;
                };
                icon.keys()
                    .all(|key| matches!(key.as_str(), "icon" | "x" | "y"))
                    && icon.get("icon").is_some_and(Value::is_string)
                    && ["x", "y"].into_iter().all(|key| {
                        icon.get(key).is_none_or(|v| {
                            v.as_i64().is_some_and(|n| (-65_535..=65_535).contains(&n))
                        })
                    })
            })
    })
}

pub fn describe(topic: &str) -> Value {
    let action = topic.strip_prefix("apps ").unwrap_or("create");
    if matches!(
        action,
        "list" | "active-get" | "select" | "order-get" | "order-set"
    ) {
        let (route, parameters, output, example) = match action {
            "list" => ("GET /api/v1/apps", json!({}), "apps: name, origin, enabled, present, in_loop, error (absent device fields are null)", "awtrix --json apps list"),
            "active-get" => ("GET /api/v1/apps/active", json!({}), "device-reported active app", "awtrix apps active-get"),
            "select" => ("PUT /api/v1/apps/active (JSON string name)", json!({"name":"[A-Za-z0-9_-]{1,32}"}), "accepted, visibility=unknown; does not remove notifications", "awtrix apps select demo"),
            "order-get" => ("GET /api/v1/apps", json!({}), "full ordered app inventory", "awtrix apps order-get"),
            _ => ("PUT /api/v1/apps/order", json!({"--order":"optional JSON string array; repeated names are allowed", "--disabled":"JSON string array, complete set of switched-off app names (default [])"}), "accepted, order_included, disabled, device_result", "awtrix apps order-set --order '[\"clock\",\"demo\"]' --disabled '[\"weather\"]'"),
        };
        return json!({"command":format!("apps {action}"),"route":route,"parameters":parameters,"inputs":[],"schemas":{"output":output},"outputs":[output],"output_fields":["apps","count","name","origin","enabled","present","in_loop","error","accepted","visibility","order_included","disabled","device_result"],"examples":[example],"prerequisites":["AWTRIX NG app API; device routes remain authoritative"],"limitations":["select acceptance does not guarantee current visibility or dismiss notifications","order-set disabled is the complete off-set; omitted order preserves existing order"]});
    }
    let parameters = if action == "delete" {
        json!({"name":"[A-Za-z0-9_-]{1,32}"})
    } else {
        json!({"name":"[A-Za-z0-9_-]{1,32}","--payload":"JSON object using pushed-app fields; durationMs controls dwell time, lifetimeMs/lifetimeExpiry control expiration", "--file":"UTF-8 JSON payload file; enables large TC002 request bodies", "--raw":"explicitly pass unknown device-specific fields without typed validation", "route_semantics":"create and update both use the device's PUT upsert; neither requires presence/absence"})
    };
    let (outputs, schema, output_fields) = if action == "delete" {
        (
            "accepted deletion; no visibility claim",
            json!({"name":"string","operation":"delete","accepted":"boolean"}),
            json!(["name", "operation", "accepted"]),
        )
    } else {
        (
            "accepted app mutation; visibility is unknown",
            json!({"name":"string","operation":"create|update","accepted":"boolean","visibility":"unknown: HTTP acceptance does not prove when or whether the app is displayed","persistent":false}),
            json!(["name", "operation", "accepted", "visibility", "persistent"]),
        )
    };
    json!({"command":format!("apps {action}"),"parameters":parameters,"inputs":["non-empty JSON pushed-app object"],"schemas":{"input":"All 35 official pushed-app keys have local top-level type checks; unknown keys require --raw. Time fields are integer milliseconds. durationMs <= 0 uses device app duration; lifetimeMs <= 0 disables expiry; lifetimeExpiry is remove|mark.","output":schema},"outputs":[outputs],"output_fields":output_fields,"examples":[format!("awtrix --target http://awtrix.local apps {action} build --payload '{{\"text\":\"working\",\"lifetimeMs\":60000}}'"),"awtrix --json apps delete build"],"prerequisites":["AWTRIX NG pushed app routes; device capacity and 413/422/507 validation remain authoritative"],"limitations":["At most 50 pushed apps reside at once; replacing an existing name does not consume another slot. HTTP request body limits are 8192 bytes on ESP32/ESP32-S3 and 2 MiB on TC002; identity is queried before sending payloads above 8192 bytes. HTTP acceptance does not guarantee visibility; pushed apps are lost on device restart"]})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_fields_without_explicit_raw_mode() {
        assert!(parse("build", r#"{"text":"ok","vendor":1}"#, false).is_err());
        assert!(parse("build", r#"{"text":"ok","vendor":1}"#, true).is_ok());
    }
    #[test]
    fn rejects_empty_and_out_of_range_expiration() {
        assert!(parse("build", "{}", false).is_err());
        assert!(parse("build", r#"{"text":"ok","lifetimeMs":-1}"#, false).is_ok());
        assert!(parse("build", r#"{"text":"ok","lifetimeMs":0}"#, false).is_ok());
        assert!(parse("build", r#"{"text":"ok","lifetimeExpiry":"later"}"#, false).is_err());
        assert!(parse("build", r#"{"text":"ok","durationMs":1.5}"#, false).is_err());
    }

    #[test]
    fn official_pushed_app_fields_accept_their_documented_top_level_types() {
        let fields = [
            ("text", json!("ok")),
            ("icon", json!("weather")),
            ("textCase", json!("upper")),
            ("font", json!("small")),
            ("textColor", json!("#ff0000")),
            ("textBlinkMs", json!(0)),
            ("textFadeMs", json!(50)),
            ("textAlign", json!("center")),
            ("scroll", json!({"mode":"bounce","speed":10})),
            ("textOffsetX", json!(-1)),
            ("textInFront", json!(true)),
            ("iconMode", json!("push")),
            ("iconOffsetX", json!(2)),
            ("iconGap", json!(128)),
            ("icons", json!([{"icon":"mail","x":0,"y":0}])),
            ("durationMs", json!(-1)),
            ("lifetimeMs", json!(0)),
            ("lifetimeExpiry", json!("mark")),
            ("repeat", json!(0)),
            ("backgroundColor", json!(16711680)),
            ("barChart", json!([1, 2])),
            ("lineChart", json!([1, 2])),
            ("chartAutoscale", json!(false)),
            ("chartColor", json!(["HSV", 0, 100, 100])),
            ("progress", json!(-1)),
            ("progressColor", json!("palette")),
            ("progressTrackColor", json!("#000000")),
            ("effect", json!("rain")),
            ("effectSpeed", json!(0.1)),
            ("palette", json!(["#000000", "#ffffff"])),
            ("paletteBlend", json!(true)),
            ("paletteSpan", json!(0)),
            ("paletteSpeed", json!(10.0)),
            ("overlay", json!("snow")),
            ("draw", json!([["pixel", 0, 0, "#ffffff"]])),
        ];
        assert_eq!(fields.len(), 35);
        for (field, value) in fields {
            assert!(
                field_kind(field).is_some(),
                "{field} must be in the typed official-field table"
            );
            let payload = json!({(field):value}).to_string();
            assert!(
                parse("sample", &payload, false).is_ok(),
                "{field}: {payload}"
            );
        }
    }

    #[test]
    fn invalid_typed_fields_and_nested_scroll_values_fail_locally() {
        for (field, value) in [
            ("text", json!(true)),
            ("icon", json!(1)),
            ("textCase", json!("lower")),
            ("font", json!(1)),
            ("textColor", json!(true)),
            ("textFadeMs", json!(false)),
            ("textBlinkMs", json!(1.5)),
            ("textAlign", json!("middle")),
            ("textOffsetX", json!(false)),
            ("textInFront", json!("yes")),
            ("iconMode", json!("slide")),
            ("iconOffsetX", json!(false)),
            ("lifetimeExpiry", json!("forever")),
            ("iconGap", json!(129)),
            ("icons", json!([{"icon":"a","unexpected":1}])),
            ("durationMs", json!(false)),
            ("lifetimeMs", json!(false)),
            ("repeat", json!(false)),
            ("backgroundColor", json!(true)),
            ("barChart", json!([1, "x"])),
            ("lineChart", json!([1, "x"])),
            ("chartAutoscale", json!("yes")),
            ("chartColor", json!(true)),
            ("progress", json!(false)),
            ("progressColor", json!(true)),
            ("progressTrackColor", json!(true)),
            ("effect", json!(1)),
            ("effectSpeed", json!(11)),
            ("palette", json!({})),
            ("paletteBlend", json!("yes")),
            ("paletteSpan", json!(false)),
            ("paletteSpeed", json!(-1)),
            ("overlay", json!(1)),
            ("draw", json!({})),
            ("scroll", json!({"speed":-1})),
        ] {
            let payload = json!({(field):value}).to_string();
            assert!(
                parse("sample", &payload, false).is_err(),
                "{field}: {payload}"
            );
        }
    }
}
