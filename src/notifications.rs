//! Notification operations. Acceptance is not evidence that the display showed the message.
use clap::Subcommand;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

#[derive(Subcommand)]
pub enum Command {
    /// Queue a notification (a device-defined upsert/queue operation).
    Send {
        #[arg(long, conflicts_with = "file", required_unless_present = "file")]
        payload: Option<String>,
        #[arg(long, conflicts_with = "payload", required_unless_present = "payload")]
        file: Option<PathBuf>,
        /// Keep the notification visible until explicitly dismissed.
        #[arg(long)]
        hold: bool,
        /// Append to the notification queue (maximum 32 queued notifications).
        #[arg(long)]
        stack: bool,
        /// Wake the display for this notification.
        #[arg(long)]
        wakeup: bool,
        /// Optional caller-defined name used only to address dismissal.
        #[arg(long)]
        name: Option<String>,
        /// Explicitly forward unknown fields without local type checks.
        #[arg(long)]
        raw: bool,
    },
    /// Dismiss the currently displayed notification.
    DeleteActive,
    /// Dismiss a queued notification by its caller-defined name.
    Delete { name: String },
}

pub fn validate(command: &Command) -> Result<(), String> {
    match command {
        Command::Send {
            payload,
            file,
            hold,
            stack,
            wakeup,
            name,
            raw,
        } => {
            let input = match (payload.as_deref(), file.as_deref()) {
                (Some(value), None) => value.to_owned(),
                (None, Some(path)) => fs::read_to_string(path)
                    .map_err(|_| "could not read payload file as UTF-8".to_owned())?,
                _ => return Err("provide exactly one of --payload or --file".into()),
            };
            prepare_payload(&input, *hold, *stack, *wakeup, name.as_deref(), *raw)?;
            Ok(())
        }
        Command::DeleteActive => Ok(()),
        Command::Delete { name } => {
            validate_name(name)?;
            if name == "active" {
                return Err("notification name 'active' is reserved".into());
            }
            Ok(())
        }
    }
}

pub fn run(command: &Command, api: &crate::ApiClient) -> crate::CliResult<Value> {
    match command {
        Command::Send {
            payload,
            file,
            hold,
            stack,
            wakeup,
            name,
            ..
        } => {
            let input = match (payload.as_deref(), file.as_deref()) {
                (Some(value), None) => value.to_owned(),
                (None, Some(path)) => fs::read_to_string(path)
                    .map_err(|_| ("ARGUMENT", "could not read payload file as UTF-8".into()))?,
                _ => {
                    return Err((
                        "ARGUMENT",
                        "provide exactly one of --payload or --file".into(),
                    ))
                }
            };
            let body = prepare_payload(
                &input,
                *hold,
                *stack,
                *wakeup,
                name.as_deref(),
                command_raw(command),
            )
            .map_err(|message| ("ARGUMENT", message))?;
            let effective_name = body.get("name").cloned().unwrap_or(Value::Null);
            api.mutate(reqwest::Method::POST, "/api/v1/notifications", &body).map_err(|(code, message)| {
                if code == "HTTP_507" { ("HTTP_507", "device returned HTTP 507 (insufficient storage); notification acceptance is not guaranteed and no retry was attempted".into()) }
                else if matches!(code, "TIMEOUT" | "TRANSPORT") { ("OUTCOME_UNKNOWN", "notification may have been accepted; outcome is unknown and no retry was attempted".into()) }
                else { (code, message) }
            })?;
            Ok(
                json!({"accepted":true,"visibility":"unknown","name":effective_name,"queue":"device-managed","runtime_success_guaranteed":false}),
            )
        }
        Command::DeleteActive => {
            api.mutate(
                reqwest::Method::DELETE,
                "/api/v1/notifications/active",
                &Value::Null,
            )?;
            Ok(json!({"accepted":true,"operation":"dismiss-active"}))
        }
        Command::Delete { name } => {
            api.mutate(
                reqwest::Method::DELETE,
                &format!("/api/v1/notifications/{name}"),
                &Value::Null,
            )?;
            Ok(json!({"accepted":true,"operation":"dismiss-named","name":name}))
        }
    }
}

fn command_raw(command: &Command) -> bool {
    matches!(command, Command::Send { raw: true, .. })
}

fn prepare_payload(
    input: &str,
    hold: bool,
    stack: bool,
    wakeup: bool,
    option_name: Option<&str>,
    raw: bool,
) -> Result<Value, String> {
    let mut value: Value =
        serde_json::from_str(input).map_err(|_| "payload must be valid JSON".to_owned())?;
    let object = value
        .as_object_mut()
        .filter(|object| !object.is_empty())
        .ok_or("payload must be a non-empty JSON object")?;
    for (key, enabled) in [("hold", hold), ("stack", stack), ("wakeup", wakeup)] {
        if let Some(payload_value) = object.get(key) {
            let payload_bool = payload_value
                .as_bool()
                .ok_or_else(|| format!("{key} must be a boolean"))?;
            if enabled && !payload_bool {
                return Err(format!("--{key} conflicts with payload {key}: false"));
            }
        } else {
            object.insert(key.into(), json!(enabled));
        }
    }
    let payload_name = object
        .get("name")
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| "name must be a string".to_owned())
        })
        .transpose()?;
    if let Some(name) = payload_name {
        validate_name(name)?;
        if name == "active" {
            return Err("notification name 'active' is reserved".into());
        }
    }
    if let Some(name) = option_name {
        validate_name(name)?;
        if name == "active" {
            return Err("notification name 'active' is reserved".into());
        }
        if payload_name.is_some_and(|payload_name| payload_name != name) {
            return Err("--name conflicts with payload name".into());
        }
        object.insert("name".into(), json!(name));
    }
    for (key, field_value) in object.iter() {
        if matches!(key.as_str(), "name" | "hold" | "stack" | "wakeup") {
            continue;
        }
        let kind = crate::apps::notification_field_kind(key);
        if kind.is_none() && !raw {
            return Err(format!(
                "unknown notification field '{key}'; pass --raw to forward it"
            ));
        }
        if kind.is_some_and(|kind| !kind.accepts(field_value)) {
            return Err(format!("{key} has an invalid JSON type or value"));
        }
    }
    Ok(value)
}

fn validate_name(name: &str) -> Result<(), String> {
    if !name.is_empty()
        && name.len() <= 32
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        Ok(())
    } else {
        Err("name must match [A-Za-z0-9_-]{1,32}".into())
    }
}

pub fn describe(topic: &str) -> Value {
    let command = if topic == "notify" {
        "send"
    } else {
        topic.strip_prefix("notify ").unwrap_or("send")
    };
    match command {
        "delete" => {
            json!({"command":"notify delete","parameters":{"name":"[A-Za-z0-9_-]{1,32}; active is reserved"},"inputs":["caller-defined notification name"],"schemas":{"output":{"accepted":"boolean","operation":"dismiss-named","name":"string"}},"outputs":["dismisses the named notification wherever it sits in the queue; 404 means no matching notification"],"output_fields":["accepted","operation","name"],"examples":["awtrix-cli --json notify delete build"],"prerequisites":["AWTRIX NG HTTP API"]})
        }
        "send" => {
            json!({"command":"notify send","parameters":{"--payload":"non-empty JSON object of pushed-app fields; notification extras hold, stack, wakeup and name are validated","--file":"UTF-8 JSON file","--hold":"set hold true; conflicts with payload hold:false","--stack":"set stack true; conflicts with payload stack:false; API stacks at most 32 notifications","--wakeup":"set wakeup true; conflicts with payload wakeup:false","--name":"optional validated name for targeted dismissal; must agree with payload name","--raw":"forward unknown device-specific fields; known fields remain validated"},"inputs":["notification content and optional hold/stack/wakeup/name"],"schemas":{"input":"typed pushed-app fields reused from apps; hold, stack and wakeup must be booleans; name must match [A-Za-z0-9_-]{1,32} and cannot be active","output":{"accepted":"boolean","visibility":"unknown","name":"string|null","queue":"device-managed","runtime_success_guaranteed":false}},"outputs":["HTTP acceptance only; visibility is not guaranteed"],"output_fields":["accepted","visibility","name","queue","runtime_success_guaranteed"],"examples":["awtrix-cli --json notify send --payload '{\"text\":\"Ready\"}' --stack --wakeup --name build"],"prerequisites":["AWTRIX NG /api/v1/notifications"],"limitations":["No deduplication guarantee for names or options","HTTP 507 remains generic insufficient storage; queue-full is not inferred","uncertain POST outcomes are not retried"]})
        }
        _ => {
            json!({"command":"notify delete-active","parameters":{},"inputs":["active notification"],"schemas":{"output":{"accepted":"boolean","operation":"dismiss-active"}},"outputs":["dismisses the current notification"],"output_fields":["accepted","operation"],"examples":["awtrix-cli --json notify delete-active"],"prerequisites":["AWTRIX NG /api/v1/notifications/active"]})
        }
    }
}
