//! Raw Berry source operations. The conditional route is atomic; raw PUT is explicitly force-only.
use clap::Subcommand;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use std::{fs, path::PathBuf};

#[derive(Subcommand)]
pub enum Command {
    /// List apps and the currently observed script state, including Berry errors.
    State,
    /// Verify a script over a bounded observation window (not a proof of correctness).
    Verify {
        name: String,
        #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..=3600))]
        duration_secs: u64,
        #[arg(long, default_value_t = 500, value_parser = clap::value_parser!(u64).range(1..=60000))]
        interval_ms: u64,
        #[arg(long, default_value_t = 0)]
        after: u64,
        #[arg(long)]
        capture: Option<PathBuf>,
    },
    /// Read exact source bytes from the device to stdout.
    Get { name: String },
    /// Deploy a raw Berry source file.
    Deploy {
        name: String,
        #[arg(long, conflicts_with = "file")]
        source: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
        /// The original remote source used for an atomic update; null only via --create.
        #[arg(long, conflicts_with_all = ["create", "force"])]
        expected_source: Option<String>,
        /// Create only if the script name is absent.
        #[arg(long, conflicts_with_all = ["expected_source", "force"])]
        create: bool,
        /// Replace unconditionally using raw PUT; no concurrency guarantee.
        #[arg(long, conflicts_with_all = ["create", "expected_source"])]
        force: bool,
        /// After saving, run verification for this many seconds (1..3600).
        #[arg(long, value_parser = clap::value_parser!(u64).range(1..=3600))]
        verify_secs: Option<u64>,
    },
    /// Enable a script without changing other apps.
    Enable { name: String },
    /// Disable a script without changing other apps.
    Disable { name: String },
    /// Delete a script and its persisted store.
    Delete { name: String },
    /// Read declared, user-changeable @config settings.
    ConfigGet { name: String },
    /// Patch declared @config settings (restarts init/setup); {} is a valid no-op object per OpenAPI.
    ConfigPut {
        name: String,
        #[arg(long, help = "JSON object of setting keys and values")]
        values: String,
    },
    /// Read data persisted by store.set().
    Data { name: String },
}

fn valid_name(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn run(command: &Command, api: &crate::ApiClient) -> crate::CliResult<Value> {
    match command {
        Command::State => {
            let apps = api.get("/api/v1/apps")?;
            let Some(apps) = apps.as_array() else {
                return Err(("INVALID_RESPONSE", "app inventory must be an array".into()));
            };
            let scripts: Vec<Value> = apps.iter().filter(|app| app.get("origin").and_then(Value::as_str) == Some("script")).map(|app| {
                let error = app.get("error").filter(|v| !v.is_null());
                json!({"name":app.get("name"),"enabled":app.get("enabled"),"in_loop":app.get("inLoop"),"present":app.get("present"),"error":error,"error_message":error.and_then(|e|e.get("message")),"error_line":error.and_then(|e|e.get("line")),"error_hook":error.and_then(|e|e.get("hook"))})
            }).collect();
            Ok(json!({"scripts":scripts,"observed":true,"runtime_success_guaranteed":false}))
        }
        Command::Verify {
            name,
            duration_secs,
            interval_ms,
            after,
            capture,
        } => {
            validate(name)?;
            verify(
                api,
                name,
                *duration_secs,
                *interval_ms,
                *after,
                capture.as_ref(),
            )
        }
        Command::Get { name } => {
            validate(name)?;
            let response = api.raw_get(&format!("/api/v1/apps/script/{name}"))?;
            // The output layer emits this raw unless --json was explicitly requested.
            Ok(json!({"source":response}))
        }
        Command::Deploy {
            name,
            source,
            file,
            expected_source,
            create,
            force,
            verify_secs,
        } => {
            validate(name)?;
            let source = match (source, file) {
                (Some(s), None) => s.clone(),
                (None, Some(path)) => fs::read_to_string(path)
                    .map_err(|_| ("ARGUMENT", "could not read source file as UTF-8".into()))?,
                _ => {
                    return Err((
                        "ARGUMENT",
                        "provide exactly one of --source or --file".into(),
                    ))
                }
            };
            if source.is_empty() {
                return Err(("ARGUMENT", "Berry source must not be empty".into()));
            }
            if *force {
                let result = api.raw_put(&format!("/api/v1/apps/script/{name}"), &source)?;
                let saved = operational_result(result, false, None)?;
                chain_verification(saved, api, name, *verify_secs)
            } else {
                let capabilities = api.get("/api/v1/capabilities")?;
                if capabilities.get("scriptUpdates").and_then(Value::as_bool) != Some(true) {
                    return Err(("PROTECTION_UNAVAILABLE", "scriptUpdates capability is absent; no write performed. Use --force for an unprotected overwrite".into()));
                }
                let expected = if *create {
                    Value::Null
                } else {
                    Value::String(expected_source.clone().ok_or(("ARGUMENT", "conditional update requires --expected-source or --create; source was not reread".into()))?)
                };
                match api.conditional_put(name, &expected, &source) {
                    Err(("CONFLICT", message)) => Err(("CONFLICT", message)),
                    Err(e) => Err(e),
                    Ok(result) => {
                        let verification = verify_start(api, name);
                        let saved = operational_result(result, true, verification)?;
                        chain_verification(saved, api, name, *verify_secs)
                    }
                }
            }
        }
        Command::Enable { name } | Command::Disable { name } => {
            validate(name)?;
            let enabled = matches!(command, Command::Enable { .. });
            let value = api.mutate(
                reqwest::Method::PUT,
                &format!("/api/v1/apps/{name}/enabled"),
                &json!(enabled),
            ).map_err(|(code, message)| if code == "HTTP_507" {
                ("APPLIED_NOT_SAVED", format!("{} was applied but not persisted; free device storage and repeat before rebooting", if enabled { "enable" } else { "disable" }))
            } else { (code, message) })?;
            Ok(
                json!({"name":name,"enabled":enabled,"accepted":true,"device_result":value,"runtime_success_guaranteed":false}),
            )
        }
        Command::Delete { name } => {
            validate(name)?;
            let value = api.mutate(
                reqwest::Method::DELETE,
                &format!("/api/v1/apps/{name}"),
                &Value::Null,
            )?;
            Ok(json!({"name":name,"deleted":true,"device_result":value}))
        }
        Command::ConfigGet { name } => {
            validate(name)?;
            api.get(&format!("/api/v1/apps/{name}/config"))
        }
        Command::ConfigPut { name, values } => {
            validate(name)?;
            let body: Value = serde_json::from_str(values)
                .map_err(|_| ("ARGUMENT", "config values must be a JSON object".into()))?;
            if !body.is_object() {
                return Err(("ARGUMENT", "config values must be a JSON object".into()));
            }
            let result = api.mutate(
                reqwest::Method::PATCH,
                &format!("/api/v1/apps/{name}/config"),
                &body,
            )?;
            Ok(
                json!({"name":name,"accepted":true,"device_result":result,"restart":"saving config restarts the script; init() and setup() run again","runtime_success_guaranteed":false}),
            )
        }
        Command::Data { name } => {
            validate(name)?;
            api.get(&format!("/api/v1/apps/{name}/data"))
        }
    }
}

fn chain_verification(
    mut saved: Value,
    api: &crate::ApiClient,
    name: &str,
    duration: Option<u64>,
) -> crate::CliResult<Value> {
    if let Some(duration) = duration {
        let report = verify(api, name, duration, 500, 0, None)?;
        if let Some(object) = saved.as_object_mut() {
            object.insert("verification".into(), report);
        }
    }
    Ok(saved)
}

fn verify(
    api: &crate::ApiClient,
    name: &str,
    duration_secs: u64,
    interval_ms: u64,
    after: u64,
    capture: Option<&PathBuf>,
) -> crate::CliResult<Value> {
    let started = Instant::now();
    let deadline = Duration::from_secs(duration_secs);
    let system = api.get_with_timeout("/api/v1/system", deadline)?;
    let apps = api.get_with_timeout("/api/v1/apps", deadline.saturating_sub(started.elapsed()))?;
    let app = apps.as_array().and_then(|items| {
        items.iter().find(|app| {
            app.get("name").and_then(Value::as_str) == Some(name)
                && app.get("origin").and_then(Value::as_str) == Some("script")
        })
    });
    let scripting = system.get("scriptingEnabled").and_then(Value::as_bool);
    let enabled = app.and_then(|a| a.get("enabled")).and_then(Value::as_bool) == Some(true);
    let mut cursor = after;
    let mut lines = Vec::new();
    let mut collection_error = None;
    let mut runtime_error = app
        .and_then(|a| a.get("error"))
        .filter(|e| !e.is_null())
        .cloned();
    let start_verified =
        scripting == Some(true) && enabled && app.is_some() && runtime_error.is_none();
    if scripting != Some(true) || app.is_none() || !enabled || !start_verified {
        return Ok(
            json!({"source_saved":"not_requested","start_verified":false,"observed_window":{"complete":false,"duration_secs":started.elapsed().as_secs_f64()},"not_available":if scripting != Some(true) {"scripting_disabled"} else if app.is_none() {"script_absent"} else {"script_disabled_or_start_error"},"runtime_error":runtime_error,"logs":{"after":after,"next":cursor,"lines":lines,"history_limit":34,"exhaustive":false},"capture":null,"runtime_success_guaranteed":false}),
        );
    }
    let mut final_state = app.cloned();
    // Reserve a bounded tail of the same deadline for optional framebuffer I/O and local PNG write.
    let capture_reserve = if capture.is_some() {
        (deadline / 4).min(Duration::from_secs(2))
    } else {
        Duration::ZERO
    };
    let observation_deadline = deadline.saturating_sub(capture_reserve);
    let poll_deadline = observation_deadline.saturating_sub(Duration::from_millis(100));
    while started.elapsed() < poll_deadline {
        let remaining = observation_deadline.saturating_sub(started.elapsed());
        match api.get_with_timeout(&format!("/api/v1/logs?after={cursor}"), remaining) {
            Ok(logs) => {
                cursor = logs
                    .get("next")
                    .and_then(Value::as_u64)
                    .unwrap_or(cursor)
                    .max(cursor);
                if let Some(batch) = logs.get("lines").and_then(Value::as_array) {
                    lines.extend(batch.iter().filter_map(Value::as_str).map(str::to_owned));
                }
            }
            Err((code, message)) => {
                collection_error = Some(json!({"code":code,"message":message}));
                break;
            }
        }
        let remaining = observation_deadline.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            break;
        }
        match api.get_with_timeout("/api/v1/apps", remaining) {
            Ok(items) => {
                final_state = items
                    .as_array()
                    .and_then(|apps| {
                        apps.iter()
                            .find(|a| a.get("name").and_then(Value::as_str) == Some(name))
                    })
                    .cloned();
                runtime_error = final_state
                    .as_ref()
                    .and_then(|a| a.get("error"))
                    .filter(|e| !e.is_null())
                    .cloned();
                if runtime_error.is_some() {
                    break;
                }
            }
            Err((code, message)) => {
                collection_error =
                    Some(json!({"code":code,"message":message,"phase":"state_observation"}));
                break;
            }
        }
        if started.elapsed() >= observation_deadline {
            break;
        }
        std::thread::sleep(
            Duration::from_millis(interval_ms)
                .min(observation_deadline.saturating_sub(started.elapsed())),
        );
    }
    let artifact = if let Some(path) = capture {
        let remaining = deadline.saturating_sub(started.elapsed());
        let result = if remaining.is_zero() {
            Err((
                "TIMEOUT",
                "verification deadline left no time for capture".into(),
            ))
        } else {
            crate::screen::run_with_timeout(
                &crate::screen::Command::Capture {
                    output: path.clone(),
                },
                api,
                remaining,
            )
        };
        match result {
            Ok(value) => Some(value),
            Err((code, message)) => {
                if collection_error.is_none() {
                    collection_error = Some(json!({"code":code,"message":message}));
                }
                None
            }
        }
    } else {
        None
    };
    Ok(
        json!({"source_saved":"not_requested","start_verified":true,"observed_window":{"complete":collection_error.is_none() && (started.elapsed() >= poll_deadline || runtime_error.is_some()),"duration_secs":started.elapsed().as_secs_f64(),"note":"No observed error is not proof of general correctness"},"not_available":collection_error,"runtime_error":runtime_error,"runtime_state":final_state,"logs":{"after":after,"next":cursor,"lines":lines,"history_limit":34,"exhaustive":false},"capture":artifact,"runtime_success_guaranteed":false}),
    )
}
fn validate(name: &str) -> crate::CliResult<()> {
    if valid_name(name) {
        Ok(())
    } else {
        Err((
            "ARGUMENT",
            "script name must match [A-Za-z0-9_-]{1,32}".into(),
        ))
    }
}
fn verify_start(api: &crate::ApiClient, name: &str) -> Option<bool> {
    let system = api.get("/api/v1/system").ok()?;
    if system.get("scriptingEnabled").and_then(Value::as_bool) != Some(true) {
        return Some(false);
    }
    let apps = api.get("/api/v1/apps").ok()?;
    let app = apps
        .as_array()?
        .iter()
        .find(|app| app.get("name").and_then(Value::as_str) == Some(name))?;
    Some(
        app.get("origin").and_then(Value::as_str) == Some("script")
            && app.get("enabled").and_then(Value::as_bool) == Some(true)
            && app.get("error").is_some_and(Value::is_null),
    )
}

fn operational_result(
    result: Value,
    atomic: bool,
    verified: Option<bool>,
) -> crate::CliResult<Value> {
    if result.get("error").is_some_and(|e| !e.is_null()) {
        return Err((
            "BERRY_ERROR",
            "source saved, but Berry compilation/setup returned an error".into(),
        ));
    }
    let start_verified = atomic && verified == Some(true);
    Ok(
        json!({"source_saved":true,"start_verified":start_verified,"execution_state":if start_verified {"verified"} else {"unknown"},"guarantee":if atomic {"atomic conditional update; restore on compile/setup failure only; no runtime guarantee"} else {"unconditional raw PUT; compile/start behavior is not verified"},"device_result":result}),
    )
}
