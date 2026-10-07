//! Temporary pushed applications, separate from persistent Berry scripts.
use clap::Subcommand;
use serde_json::{json, Value};

const MAX_NAME: usize = 32;
// Local safety contract: bound client request memory independently of device-side 413 handling.
const MAX_PAYLOAD_BYTES: usize = 8192;

#[derive(Subcommand)]
pub enum Command {
    /// Create a pushed app, or replace it when it already exists.
    Create {
        name: String,
        #[arg(long)]
        payload: String,
        #[arg(
            long,
            help = "Allow untyped device-specific fields in this JSON object"
        )]
        raw: bool,
    },
    /// Replace an existing pushed app.
    Update {
        name: String,
        #[arg(long)]
        payload: String,
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
        Command::Create { name, payload, raw } | Command::Update { name, payload, raw } => {
            let body = parse(name, payload, *raw).map_err(|message| ("ARGUMENT", message))?;
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
        Command::Create { name, payload, raw } | Command::Update { name, payload, raw } => {
            parse(name, payload, *raw).map(|_| ())
        }
        Command::Delete { name } => validate_name(name),
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
    if input.len() > MAX_PAYLOAD_BYTES {
        return Err(format!(
            "payload exceeds local {} byte limit",
            MAX_PAYLOAD_BYTES
        ));
    }
    let value: Value =
        serde_json::from_str(input).map_err(|_| "payload must be valid JSON".to_owned())?;
    let object = value
        .as_object()
        .filter(|map| !map.is_empty())
        .ok_or("payload must be a non-empty JSON object")?;
    if let Some((key, _)) = object.iter().find(|(key, _)| !known_field(key)) {
        if !raw {
            return Err(format!("unknown pushed-app field '{key}'; pass --raw to explicitly forward device-specific fields"));
        }
    }
    for key in ["text", "icon", "effect", "overlay"] {
        if let Some(value) = object.get(key) {
            if !value.is_string() {
                return Err(format!("{key} must be a string"));
            }
            if key == "text" && value.as_str().is_some_and(str::is_empty) {
                return Err("text must not be empty".into());
            }
        }
    }
    for key in ["durationMs", "lifetimeMs", "textBlinkMs", "textFadeMs"] {
        if let Some(value) = object.get(key) {
            value.as_i64().filter(|n| *n >= 0).ok_or(format!(
                "{key} must be a non-negative integer in milliseconds"
            ))?;
        }
    }
    for (key, value) in object {
        let valid = match key.as_str() {
            "text" => value.is_string() || value.is_array(),
            "icon" | "textCase" | "font" | "iconMode" | "lifetimeExpiry" | "effect" | "overlay" => {
                value.is_string()
            }
            "textColor" | "backgroundColor" | "chartColor" | "progressColor"
            | "progressTrackColor" => value.is_string() || value.is_number() || value.is_array(),
            "scroll" => value.is_string() || value.is_object(),
            "draw" | "barChart" | "lineChart" | "palette" | "icons" => {
                value.is_array() || value.is_string()
            }
            "durationMs" | "lifetimeMs" | "textBlinkMs" | "textFadeMs" | "textOffsetX"
            | "iconOffsetX" | "repeat" | "progress" | "paletteSpan" | "iconGap" | "textAlign"
            | "iconAlign" => value.as_i64().is_some(),
            "effectSpeed" | "paletteSpeed" => value.is_number(),
            "textCenter" | "textInFront" | "chartAutoscale" | "paletteBlend" | "rainbow"
            | "noScroll" => value.is_boolean(),
            "color" | "blink" | "fade" | "border" | "center" | "top" | "pushIcon" => {
                value.is_boolean() || value.is_string() || value.is_number()
            }
            _ => true,
        };
        if !valid {
            return Err(format!("{key} has the wrong JSON type"));
        }
        let allowed = match key.as_str() {
            "textCase" => value
                .as_str()
                .is_some_and(|v| matches!(v, "inherit" | "upper" | "asTyped")),
            "font" => value
                .as_str()
                .is_some_and(|v| matches!(v, "small" | "large")),
            "iconMode" => value
                .as_str()
                .is_some_and(|v| matches!(v, "fixed" | "pushOnce" | "push")),
            "lifetimeExpiry" => value
                .as_str()
                .is_some_and(|v| matches!(v, "remove" | "mark")),
            "durationMs" | "lifetimeMs" | "textBlinkMs" | "textFadeMs" | "repeat" => {
                value.as_i64().is_some_and(|n| n >= 0)
            }
            "barChart" | "lineChart" => value
                .as_array()
                .is_some_and(|items| items.len() <= 16 && items.iter().all(Value::is_i64)),
            "textCenter" | "textInFront" | "chartAutoscale" | "paletteBlend" => value.is_boolean(),
            _ => true,
        };
        if !allowed {
            return Err(format!(
                "{key} has an invalid value or is outside its documented range"
            ));
        }
    }
    if !object.contains_key("text")
        && !object.contains_key("icon")
        && !object.contains_key("icons")
        && !object.contains_key("draw")
    {
        return Err("payload requires at least one of text, icon, icons, or draw".into());
    }
    Ok(Value::Object(object.clone()))
}

fn known_field(key: &str) -> bool {
    matches!(
        key,
        "text"
            | "icon"
            | "icons"
            | "textColor"
            | "draw"
            | "effect"
            | "overlay"
            | "durationMs"
            | "lifetimeMs"
            | "lifetimeExpiry"
            | "textBlinkMs"
            | "textFadeMs"
            | "scroll"
            | "progress"
            | "barChart"
            | "lineChart"
            | "repeat"
            | "backgroundColor"
            | "font"
            | "textCase"
            | "textOffsetX"
            | "textCenter"
            | "textInFront"
            | "iconMode"
            | "iconOffsetX"
            | "iconGap"
            | "chartAutoscale"
            | "chartColor"
            | "progressColor"
            | "progressTrackColor"
            | "effectSpeed"
            | "palette"
            | "paletteBlend"
            | "paletteSpan"
            | "paletteSpeed"
    )
}

pub fn describe(topic: &str) -> Value {
    let action = topic.strip_prefix("apps ").unwrap_or("create");
    let parameters = if action == "delete" {
        json!({"name":"[A-Za-z0-9_-]{1,32}"})
    } else {
        json!({"name":"[A-Za-z0-9_-]{1,32}","--payload":"non-empty JSON object; text, icon, icons, draw, color/effect/overlay, durationMs/lifetimeMs and display fields; lifetimeMs expires the app", "--raw":"explicitly pass device-specific unknown fields through validation"})
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
    json!({"command":format!("apps {action}"),"parameters":parameters,"inputs":["JSON pushed-app object"],"schemas":{"input":"An object with at least one of text, icon, icons or draw; unknown keys require --raw. lifetimeMs and durationMs use non-negative milliseconds; zero lifetime means no expiry.","output":schema},"outputs":[outputs],"output_fields":output_fields,"examples":[format!("awtrix --target http://awtrix.local apps {action} build --payload '{{\"text\":\"working\",\"lifetimeMs\":60000}}'"),"awtrix --json apps delete build"],"prerequisites":["AWTRIX NG pushed app routes; device capacity and 413/422/507 validation remain authoritative"],"limitations":["At most 50 pushed apps reside at once; replacing an existing name does not consume another slot. Payload limit is 8192 bytes. HTTP acceptance does not guarantee visibility; pushed apps are lost on device restart"]})
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
        assert!(parse("build", r#"{"text":""}"#, false).is_err());
        assert!(parse("build", r#"{"text":"ok","lifetimeMs":-1}"#, false).is_err());
        assert!(parse("build", r#"{"text":"ok","lifetimeMs":0}"#, false).is_ok());
        assert!(parse("build", r#"{"text":"ok","lifetimeExpiry":"later"}"#, false).is_err());
        assert!(parse("build", r#"{"text":"ok","durationMs":1.5}"#, false).is_err());
    }
}
