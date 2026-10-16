//! Raw Berry source operations. The conditional route is atomic; raw PUT is explicitly force-only.
use clap::Subcommand;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Subcommand)]
pub enum Command {
    /// List apps and the currently observed script state, including Berry errors.
    State,
    /// Verify a script over a bounded observation window (not a proof of correctness).
    #[command(
        long_about = "Observe an existing script without deployment. Start evidence requires scriptingEnabled=true and an enabled origin=script app with explicit error=null; a missing error field is inconclusive. Poll state and bounded logs over the requested duration, including HTTP time. Optional capture uses the same deadline. JSON reports start_verified, observed_window, runtime_error, not_available (with collection phase), logs and capture metadata; no observed error proves general correctness. See describe script verify --offline for the full report contract."
    )]
    Verify {
        name: String,
        /// Total observation bound, including HTTP and optional capture (1..3600 seconds).
        #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..=3600))]
        duration_secs: u64,
        /// Poll spacing (1..60000 milliseconds).
        #[arg(long, default_value_t = 500, value_parser = clap::value_parser!(u64).range(1..=60000))]
        interval_ms: u64,
        /// Initial incremental log cursor; history is bounded and not exhaustive.
        #[arg(long, default_value_t = 0)]
        after: u64,
        /// Optional PNG destination; capture failures are reported with phase=capture.
        #[arg(long)]
        capture: Option<PathBuf>,
    },
    /// Read exact source bytes from the device to stdout.
    Get { name: String },
    /// Fetch, back up, and conditionally replace an installed script with its minified source.
    ReMinify {
        name: String,
        /// Skip the local exact-source backup.
        #[arg(long)]
        no_backup: bool,
    },
    /// Deploy Berry source using create-only or exact-source conditional protection; --force opts out.
    #[command(
        long_about = "Deploy Berry source using the selected existing protection. --capture-dir opts into --capture-count (default 1) PNGs after new log lines exactly matching '[script:<name>] [CAPTURE] <label>', optionally preceded by an HH:MM:SS timestamp. Labels are 1..64 ASCII letters, digits, '_' or '-'. The cursor is taken before deployment; after explicit start confirmation the script is selected once and observed under a shared --capture-timeout-ms budget (default 10000, 1..3600000). Capture mode cannot be combined with --verify-secs. Timeout/failure returns nonzero with source-save and partial capture_collection evidence; it does not undo deployment. --force remains unverified outside this mode; inside capture mode it receives a separate explicit post-deploy start check. A batch cannot reconstruct past frames and no exact frame synchronization or runtime-correctness guarantee is made."
    )]
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
        /// Capture named markers into an isolated directory.
        #[arg(long, conflicts_with = "verify_secs")]
        capture_dir: Option<PathBuf>,
        /// Number of PNGs to save (positive; requires --capture-dir).
        #[arg(long, requires = "capture_dir", value_parser = clap::value_parser!(u64).range(1..))]
        capture_count: Option<u64>,
        /// Maximum marker-observation window in milliseconds.
        #[arg(long, requires = "capture_dir", value_parser = clap::value_parser!(u64).range(1..=3600000), default_value_t = 10000)]
        capture_timeout_ms: u64,
        /// Minify the deployment payload; the source file and expected-source reference stay original.
        #[arg(long)]
        minify: bool,
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
    /// Partially patch persisted script data (null deletes keys; restarts the script).
    #[command(
        long_about = "Partially PATCH the script store without redeploying source. Omitted keys remain unchanged and null deletes a key; firmware rejects @config keys (422). Saving restarts the script and reruns initialization, which may change injected values. HTTP 200 with a Berry error means data was saved but restart failed; no rollback or retry occurs. Runtime success is not guaranteed."
    )]
    DataPatch {
        name: String,
        #[arg(long, help = "JSON object of store keys and values")]
        values: String,
    },
    /// Read data persisted by store.set().
    Data { name: String },
    /// Read the current temporary publications shared by scripts.
    #[command(
        long_about = "Read a point-in-time snapshot of temporary script publications. This never reads another script's private store. Publications may disappear when the device restarts or their owner is removed/re-registered; ageMs is returned as reported, without filtering or interpretation."
    )]
    Shared,
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
        Command::ReMinify { name, no_backup } => {
            validate(name)?;
            let original = api.raw_get(&format!("/api/v1/apps/script/{name}"))?;
            let payload = crate::minify::minify_source(&original)?;
            let capabilities = api.get("/api/v1/capabilities")?;
            if capabilities.get("scriptUpdates").and_then(Value::as_bool) != Some(true) {
                return Err((
                    "PROTECTION_UNAVAILABLE",
                    "scriptUpdates capability is absent; no write performed".into(),
                ));
            }
            let backup = if *no_backup {
                None
            } else {
                Some(write_script_backup(name, &original)?)
            };
            let result = api.conditional_put(name, &Value::String(original), &payload)?;
            let saved = operational_result(result, true, None)?;
            Ok(
                json!({"name":name,"backup":backup,"backup_created":backup.is_some(),"minified":true,"source_saved":saved["source_saved"],"start_verified":false,"execution_state":"unknown","guarantee":saved["guarantee"],"device_result":saved["device_result"]}),
            )
        }
        Command::Deploy {
            name,
            source,
            file,
            expected_source,
            create,
            force,
            verify_secs,
            capture_dir,
            capture_count,
            capture_timeout_ms,
            minify,
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
            let payload = if *minify {
                crate::minify::minify_source(&source)?
            } else {
                source.clone()
            };
            let capture_root = if let Some(dir) = capture_dir {
                if !dir.is_dir() {
                    return Err((
                        "ARGUMENT",
                        "--capture-dir must name an existing directory".into(),
                    ));
                }
                Some(
                    tempfile::Builder::new()
                        .prefix("attempt-")
                        .tempdir_in(dir)
                        .map_err(|e| {
                            ("FILE_WRITE", format!("cannot reserve capture attempt: {e}"))
                        })?
                        .keep(),
                )
            } else {
                None
            };
            let baseline = if capture_root.is_some() {
                Some(
                    api.get("/api/v1/logs?after=0")?
                        .get("next")
                        .and_then(Value::as_u64)
                        .ok_or((
                            "INVALID_RESPONSE",
                            "log cursor missing before deployment".into(),
                        ))?,
                )
            } else {
                None
            };
            if *force {
                let result = api.raw_put(&format!("/api/v1/apps/script/{name}"), &payload)?;
                let saved = operational_result(result, false, None)?;
                if let Some(root) = capture_root {
                    let start_evidence = verify_start_evidence(api, name);
                    capture_one_after(
                        saved,
                        api,
                        name,
                        root,
                        (*capture_timeout_ms, capture_count.unwrap_or(1)),
                        baseline.unwrap_or(0),
                        start_evidence,
                    )
                } else {
                    chain_verification(saved, api, name, *verify_secs)
                }
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
                match api.conditional_put(name, &expected, &payload) {
                    Err(("CONFLICT", message)) => Err(("CONFLICT", message)),
                    Err(e) => Err(e),
                    Ok(result) => {
                        let start_evidence = capture_root
                            .as_ref()
                            .map(|_| verify_start_evidence(api, name));
                        let verification = start_evidence
                            .as_ref()
                            .map(|e| e["confirmed"].as_bool() == Some(true))
                            .or_else(|| verify_start(api, name));
                        let saved = operational_result(result, true, verification)?;
                        if let Some(root) = capture_root {
                            capture_one_after(
                                saved,
                                api,
                                name,
                                root,
                                (*capture_timeout_ms, capture_count.unwrap_or(1)),
                                baseline.unwrap_or(0),
                                start_evidence.unwrap_or_else(|| json!({"confirmed":false})),
                            )
                        } else {
                            chain_verification(saved, api, name, *verify_secs)
                        }
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
        Command::Shared => api.get("/api/v1/scripts/shared"),
        Command::DataPatch { name, values } => {
            validate(name)?;
            let body: Value = serde_json::from_str(values)
                .map_err(|_| ("ARGUMENT", "data values must be a JSON object".into()))?;
            if !body.is_object() {
                return Err(("ARGUMENT", "data values must be a JSON object".into()));
            }
            let result = api.mutate(
                reqwest::Method::PATCH,
                &format!("/api/v1/apps/{name}/data"),
                &body,
            )?;
            if result.get("error").is_some_and(|error| !error.is_null()) {
                return Ok(
                    json!({"__berry_error":true,"name":name,"device_result":result,"message":"data was saved but script restart failed; no rollback was performed","runtime_success_guaranteed":false}),
                );
            }
            Ok(
                json!({"name":name,"accepted":true,"device_result":result,"restart":"saving data restarts the script; initialization may immediately change injected values","runtime_success_guaranteed":false}),
            )
        }
    }
}

fn write_script_backup(name: &str, source: &str) -> crate::CliResult<PathBuf> {
    use std::io::Write;
    let path = PathBuf::from(format!("{name}.bak.ax"));
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        (
            "BACKUP",
            format!("cannot stage backup {}: {error}", path.display()),
        )
    })?;
    file.write_all(source.as_bytes())
        .and_then(|()| file.as_file().sync_all())
        .map_err(|error| {
            (
                "BACKUP",
                format!("cannot write backup {}: {error}", path.display()),
            )
        })?;
    file.persist_noclobber(&path).map_err(|error| {
        let message = if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            format!(
                "backup {} already exists; existing backups are never overwritten",
                path.display()
            )
        } else {
            format!("cannot install backup {}: {}", path.display(), error.error)
        };
        ("BACKUP", message)
    })?;
    Ok(path)
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

fn capture_one_after(
    mut saved: Value,
    api: &crate::ApiClient,
    name: &str,
    root: PathBuf,
    (timeout_ms, expected): (u64, u64),
    mut cursor: u64,
    start_evidence: Value,
) -> crate::CliResult<Value> {
    let started = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let start_verified = start_evidence["confirmed"] == true;
    let mut report = json!({"attempt":root.file_name().and_then(|x|x.to_str()),"directory":root,"start_verified":start_verified,"start_evidence":start_evidence,"expected":expected,"obtained":0,"captures":[],"stop_reason":"observation_timeout","error":null,"runtime_error":null,"runtime_success_guaranteed":false});
    if !start_verified {
        report["stop_reason"] = json!("error");
        let code = if report["start_evidence"]["scripting_enabled"] == false {
            "SCRIPTING_DISABLED"
        } else if report["start_evidence"]
            .get("runtime_error")
            .is_some_and(|error| !error.is_null())
        {
            "BERRY_ERROR"
        } else if report["start_evidence"]["app_present"] == false {
            "SCRIPT_ABSENT"
        } else if report["start_evidence"]["enabled"] != true {
            "SCRIPT_NOT_RUNNING"
        } else if report["start_evidence"]["error_explicit_null"] != true {
            "ERROR_STATE_UNAVAILABLE"
        } else {
            "START_UNCONFIRMED"
        };
        report["error"] = json!({"code":code,"message":"script start was not confirmed; capture was not attempted","phase":"start_confirmation"});
        saved["capture_collection"] = report;
        saved["__capture_failure"] = json!(true);
        return Ok(saved);
    }
    let deadline = started + timeout;
    let selected = api.mutate_with_timeout(
        reqwest::Method::PUT,
        "/api/v1/apps/active",
        &json!({"name":name,"fast":true}),
        timeout,
    );
    match selected {
        Err((code, message)) => {
            report["stop_reason"] = json!(if code == "TIMEOUT" && Instant::now() >= deadline {
                "observation_timeout"
            } else {
                "error"
            });
            report["error"] = json!({"code":code,"message":message,"phase":"selection"});
            saved["capture_collection"] = report;
            saved["__capture_failure"] = json!(true);
            return Ok(saved);
        }
        Ok(result) if result.get("error").is_some_and(|error| !error.is_null()) => {
            report["stop_reason"] = json!("error");
            report["error"] = json!({"code":"SELECTION_FAILED","message":"device rejected script selection","phase":"selection"});
            report["device_result"] = result;
            saved["capture_collection"] = report;
            saved["__capture_failure"] = json!(true);
            return Ok(saved);
        }
        Ok(_) => {}
    }
    let mut warnings: usize = 0;
    let mut received = std::collections::VecDeque::new();
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let logs = match api.get_with_timeout(&format!("/api/v1/logs?after={cursor}"), remaining) {
            Ok(x) => x,
            Err((c, m)) => {
                if c == "TIMEOUT" && Instant::now() >= deadline {
                    report["stop_reason"] = json!("observation_timeout");
                    report["error"] = json!({"code":c,"message":m,"phase":"log_collection"});
                    break;
                }
                report["stop_reason"] = json!("error");
                report["error"] = json!({"code":c,"message":m,"phase":"log_collection"});
                break;
            }
        };
        let Some(next) = logs.get("next").and_then(Value::as_u64) else {
            report["stop_reason"] = json!("error");
            report["error"] = json!({"code":"INVALID_RESPONSE","message":"log cursor is missing or invalid","phase":"log_collection"});
            break;
        };
        let Some(lines) = logs.get("lines").and_then(Value::as_array) else {
            report["stop_reason"] = json!("error");
            report["error"] = json!({"code":"INVALID_RESPONSE","message":"log lines are missing or invalid","phase":"log_collection"});
            break;
        };
        cursor = next.max(cursor);
        let mut found = Vec::new();
        {
            for line in lines.iter().filter_map(Value::as_str) {
                if received.len() == 34 {
                    received.pop_front();
                }
                received.push_back(line.to_owned());
                match parse_capture_marker(line, name) {
                    Some(label) => {
                        found.push((label, started.elapsed()));
                    }
                    None if line.contains(&format!("[script:{name}]"))
                        && line.contains("[CAPTURE]") =>
                    {
                        warnings = warnings.saturating_add(1)
                    }
                    None => {}
                }
            }
        }
        if Instant::now() >= deadline {
            break;
        }
        match api.get_with_timeout(
            "/api/v1/apps",
            deadline.saturating_duration_since(Instant::now()),
        ) {
            Ok(apps) => {
                let app = apps.as_array().and_then(|items| {
                    items.iter().find(|app| {
                        app.get("name").and_then(Value::as_str) == Some(name)
                            && app.get("origin").and_then(Value::as_str) == Some("script")
                    })
                });
                if let Some(error) = app
                    .and_then(|app| app.get("error"))
                    .filter(|error| !error.is_null())
                {
                    report["stop_reason"] = json!("error");
                    report["runtime_error"] = error.clone();
                    report["error"] = json!({"code":"BERRY_ERROR","message":"script reported a runtime error during capture observation","phase":"state_observation"});
                    break;
                }
                if !app.is_some_and(|app| {
                    app.get("enabled").and_then(Value::as_bool) == Some(true)
                        && app.get("error").is_some_and(Value::is_null)
                }) {
                    report["stop_reason"] = json!("error");
                    report["error"] = json!({"code":"SCRIPT_NOT_RUNNING","message":"script is no longer explicitly enabled and error-free","phase":"state_observation"});
                    break;
                }
            }
            Err((code, message)) => {
                if code == "TIMEOUT" && Instant::now() >= deadline {
                    report["stop_reason"] = json!("observation_timeout");
                    report["error"] =
                        json!({"code":code,"message":message,"phase":"state_observation"});
                    break;
                }
                report["stop_reason"] = json!("error");
                report["error"] =
                    json!({"code":code,"message":message,"phase":"state_observation"});
                break;
            }
        }
        for (label, marker_at) in found {
            if Instant::now() >= deadline || report["obtained"].as_u64() == Some(expected) {
                break;
            }
            let number = report["obtained"].as_u64().unwrap_or(0) + 1;
            let path = root.join(format!("{label}-{number}.png"));
            let capture_started = started.elapsed();
            let result = crate::screen::run_with_timeout(
                &crate::screen::Command::Capture {
                    format: crate::screen::Format::Png,
                    output: Some(path.clone()),
                },
                api,
                deadline.saturating_duration_since(Instant::now()),
            );
            match result {
                Ok(image) => {
                    report["obtained"] = json!(number);
                    report["captures"].as_array_mut().unwrap().push(json!({"name":label,"number":number,"path":path,"marker_received_ms":marker_at.as_millis(),"capture_started_ms":capture_started.as_millis(),"capture_finished_ms":started.elapsed().as_millis(),"capture":image}));
                    if number == expected {
                        report["stop_reason"] = json!("capture_limit");
                        break;
                    }
                }
                Err((c, m)) => {
                    report["stop_reason"] =
                        json!(if c == "TIMEOUT" && Instant::now() >= deadline {
                            "observation_timeout"
                        } else {
                            "error"
                        });
                    report["error"] = json!({"code":c,"message":m,"phase":"capture"});
                    saved["__capture_failure"] = json!(true);
                    break;
                }
            }
        }
        if report["obtained"].as_u64() == Some(expected) || report["stop_reason"] == "error" {
            break;
        }
        std::thread::sleep(
            Duration::from_millis(250).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    report["elapsed_ms"] = json!(started.elapsed().as_millis());
    report["warning_count"] = json!(warnings.min(34));
    if report["obtained"].as_u64() != Some(expected) {
        if report["stop_reason"] == "observation_timeout"
            && !report.get("error").is_some_and(|error| !error.is_null())
        {
            report["error"] = json!({"code":"TIMEOUT","message":"no valid capture marker was observed before the capture deadline","phase":"capture_observation"});
        }
        report["diagnostic_logs"] = json!(received);
        saved["__capture_failure"] = json!(true);
    }
    saved["capture_collection"] = report;
    Ok(saved)
}

fn parse_capture_marker(line: &str, target: &str) -> Option<String> {
    let rest = if line.as_bytes().get(2) == Some(&b':') {
        let (time, rest) = line.split_once(' ')?;
        let mut parts = time.split(':');
        let h = parts.next()?.parse::<u8>().ok()?;
        let m = parts.next()?.parse::<u8>().ok()?;
        let s = parts.next()?.parse::<u8>().ok()?;
        if parts.next().is_some() || h > 23 || m > 59 || s > 59 || time.len() != 8 {
            return None;
        }
        rest
    } else {
        line
    };
    let prefix = format!("[script:{target}] [CAPTURE] ");
    let marker = rest.strip_prefix(&prefix)?;
    if marker.is_empty()
        || marker.len() > 64
        || !marker
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return None;
    }
    Some(marker.to_owned())
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
    let system = match api.get_with_timeout("/api/v1/system", deadline) {
        Ok(value) => value,
        Err((code, message)) => {
            return Ok(initial_failure(after, started, code, message, "system"))
        }
    };
    if system.get("scriptingEnabled").and_then(Value::as_bool) != Some(true) {
        return Ok(
            json!({"source_saved":"not_requested","start_verified":false,"observed_window":{"complete":false,"duration_secs":started.elapsed().as_secs_f64(),"elapsed_ms":started.elapsed().as_millis(),"early_termination_reason":"scripting_disabled"},"not_available":"scripting_disabled","runtime_error":null,"runtime_state":null,"logs":{"after":after,"next":after,"lines":[],"history_limit":34,"exhaustive":false},"capture":null,"runtime_success_guaranteed":false}),
        );
    }
    let remaining = deadline.saturating_sub(started.elapsed());
    let apps = match api.get_with_timeout("/api/v1/apps", remaining) {
        Ok(value) => value,
        Err((code, message)) => return Ok(initial_failure(after, started, code, message, "apps")),
    };
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
    let error_is_null = app.and_then(|a| a.get("error")).is_some_and(Value::is_null);
    let start_verified = scripting == Some(true) && enabled && error_is_null;
    if scripting != Some(true) || app.is_none() || !enabled || !start_verified {
        let reason = if scripting != Some(true) {
            "scripting_disabled"
        } else if app.is_none() {
            "script_absent"
        } else if runtime_error.is_some() {
            "berry_error"
        } else if enabled && !error_is_null {
            "error_state_unavailable"
        } else {
            "script_disabled"
        };
        return Ok(
            json!({"source_saved":"not_requested","start_verified":false,"observed_window":{"complete":false,"duration_secs":started.elapsed().as_secs_f64(),"elapsed_ms":started.elapsed().as_millis(),"early_termination_reason":reason},"not_available":reason,"runtime_error":runtime_error,"logs":{"after":after,"next":cursor,"lines":lines,"history_limit":34,"exhaustive":false},"capture":null,"runtime_success_guaranteed":false}),
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
                collection_error =
                    Some(json!({"code":code,"message":message,"phase":"log_collection"}));
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
                let still_running = final_state.as_ref().is_some_and(|app| {
                    app.get("origin").and_then(Value::as_str) == Some("script")
                        && app.get("enabled").and_then(Value::as_bool) == Some(true)
                });
                if !still_running {
                    collection_error = Some(
                        json!({"code":"SCRIPT_NOT_RUNNING","message":"script disappeared, changed origin, or became disabled during verification","phase":"state_observation"}),
                    );
                    break;
                }
                if !final_state
                    .as_ref()
                    .and_then(|app| app.get("error"))
                    .is_some_and(Value::is_null)
                {
                    collection_error = Some(
                        json!({"code":"INVALID_RESPONSE","message":"script error field is missing; explicit error=null is required for verification","phase":"state_observation"}),
                    );
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
                    format: crate::screen::Format::Png,
                    output: Some(path.clone()),
                },
                api,
                remaining,
            )
        };
        match result {
            Ok(value) => Some(value),
            Err((code, message)) => {
                if collection_error.is_none() {
                    collection_error =
                        Some(json!({"code":code,"message":message,"phase":"capture"}));
                }
                None
            }
        }
    } else {
        None
    };
    if collection_error.is_none() && runtime_error.is_none() && started.elapsed() < deadline {
        std::thread::sleep(deadline.saturating_sub(started.elapsed()));
    }
    let early_reason = if runtime_error.is_some() {
        Some("berry_error")
    } else if collection_error.as_ref().is_some_and(|error| {
        error.get("code").and_then(Value::as_str) == Some("SCRIPT_NOT_RUNNING")
    }) {
        Some("script_not_running")
    } else if collection_error.is_some() {
        Some("collection_error")
    } else {
        None
    };
    Ok(
        json!({"source_saved":"not_requested","start_verified":true,"observed_window":{"complete":started.elapsed() >= deadline && early_reason.is_none(),"duration_secs":started.elapsed().as_secs_f64(),"elapsed_ms":started.elapsed().as_millis(),"early_termination_reason":early_reason,"note":"No observed error is not proof of general correctness"},"not_available":collection_error,"runtime_error":runtime_error,"runtime_state":final_state,"logs":{"after":after,"next":cursor,"lines":lines,"history_limit":34,"exhaustive":false},"capture":artifact,"runtime_success_guaranteed":false}),
    )
}

fn initial_failure(
    after: u64,
    started: Instant,
    code: &str,
    message: String,
    phase: &str,
) -> Value {
    json!({"source_saved":"not_requested","start_verified":false,"observed_window":{"complete":false,"duration_secs":started.elapsed().as_secs_f64(),"elapsed_ms":started.elapsed().as_millis(),"early_termination_reason":"collection_error"},"not_available":{"code":code,"message":message,"phase":phase},"runtime_error":null,"runtime_state":null,"logs":{"after":after,"next":after,"lines":[],"history_limit":34,"exhaustive":false},"capture":null,"runtime_success_guaranteed":false})
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

/// Project deployment preserves the ticket-07 conditional update/create contract.
pub(crate) fn deploy_project(
    api: &crate::ApiClient,
    name: &str,
    source: &str,
    expected: Option<&str>,
    create: bool,
    force: bool,
) -> crate::CliResult<Value> {
    validate(name)?;
    if force {
        return operational_result(
            api.raw_put(&format!("/api/v1/apps/script/{name}"), source)?,
            false,
            None,
        );
    }
    let caps = api.get("/api/v1/capabilities")?;
    if caps.get("scriptUpdates").and_then(Value::as_bool) != Some(true) {
        return Err((
            "PROTECTION_UNAVAILABLE",
            "scriptUpdates capability absent; no script write performed".into(),
        ));
    }
    let expected = if create {
        Value::Null
    } else {
        Value::String(
            expected
                .ok_or((
                    "PROJECT_INVALID",
                    "script lacks original source reference".into(),
                ))?
                .to_owned(),
        )
    };
    let response = api.conditional_put(name, &expected, source)?;
    operational_result(response, true, verify_start(api, name))
}
fn verify_start(api: &crate::ApiClient, name: &str) -> Option<bool> {
    let evidence = verify_start_evidence(api, name);
    if evidence.get("error").is_some() {
        None
    } else {
        evidence["confirmed"].as_bool()
    }
}

fn verify_start_evidence(api: &crate::ApiClient, name: &str) -> Value {
    let system = match api.get("/api/v1/system") {
        Ok(value) => value,
        Err((code, message)) => {
            return json!({"confirmed":false,"phase":"system","error":{"code":code,"message":message}})
        }
    };
    let scripting_enabled = system.get("scriptingEnabled").and_then(Value::as_bool);
    if scripting_enabled != Some(true) {
        return json!({"confirmed":false,"scripting_enabled":scripting_enabled,"phase":"system"});
    }
    let apps = match api.get("/api/v1/apps") {
        Ok(value) => value,
        Err((code, message)) => {
            return json!({"confirmed":false,"scripting_enabled":scripting_enabled,"phase":"apps","error":{"code":code,"message":message}})
        }
    };
    let app = apps.as_array().and_then(|items| {
        items
            .iter()
            .find(|app| app.get("name").and_then(Value::as_str) == Some(name))
    });
    let origin = app
        .and_then(|app| app.get("origin"))
        .and_then(Value::as_str);
    let enabled = app
        .and_then(|app| app.get("enabled"))
        .and_then(Value::as_bool);
    let error_explicit_null = app
        .and_then(|app| app.get("error"))
        .is_some_and(Value::is_null);
    let runtime_error = app
        .and_then(|app| app.get("error"))
        .filter(|error| !error.is_null());
    json!({"confirmed":scripting_enabled == Some(true) && origin == Some("script") && enabled == Some(true) && error_explicit_null,"scripting_enabled":scripting_enabled,"app_present":app.is_some(),"origin":origin,"enabled":enabled,"error_explicit_null":error_explicit_null,"runtime_error":runtime_error})
}

fn operational_result(
    result: Value,
    atomic: bool,
    verified: Option<bool>,
) -> crate::CliResult<Value> {
    if result.get("error").is_some_and(|e| !e.is_null()) {
        return Err((
            "BERRY_ERROR",
            if atomic {
                "conditional update failed Berry compilation/setup; the old script is restored when replacing an existing script; new source was not confirmed saved".into()
            } else {
                "source saved, but Berry compilation/setup returned an error".into()
            },
        ));
    }
    let start_verified = atomic && verified == Some(true);
    Ok(
        json!({"source_saved":true,"start_verified":start_verified,"execution_state":if start_verified {"verified"} else {"unknown"},"guarantee":if atomic {"atomic conditional update; restore on compile/setup failure only; no runtime guarantee"} else {"unconditional raw PUT; compile/start behavior is not verified"},"device_result":result}),
    )
}

#[cfg(test)]
mod capture_tests {
    use super::parse_capture_marker;

    #[test]
    fn capture_marker_requires_exact_timestamp_script_and_token_prefix() {
        assert_eq!(
            parse_capture_marker("01:02:03 [script:demo] [CAPTURE] frame_1", "demo"),
            Some("frame_1".into())
        );
        assert_eq!(
            parse_capture_marker("[script:demo] [CAPTURE] frame-2", "demo"),
            Some("frame-2".into())
        );
        for line in [
            "01:02:03 [script:demographic] [CAPTURE] frame",
            "01:02:03 [script:other] [CAPTURE] frame",
            "garbage [CAPTURE] frame",
            "25:02:03 [script:demo] [CAPTURE] frame",
            "01:02:03 [script:demo] note [CAPTURE] frame",
            "01:02:03 [script:demo] [CAPTURE] ../escape",
            "01:02:03 [script:demo] [CAPTURE] frame extra",
        ] {
            assert_eq!(parse_capture_marker(line, "demo"), None, "{line}");
        }
    }
}
