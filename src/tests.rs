//! Declarative project checks. The runner deliberately accepts no shell commands.
use clap::{Args, Subcommand};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

const MAX_IMAGE_PIXELS: u64 = 4_194_304;

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Deploy a project, then evaluate its declared assertions.
    #[command(
        long_about = "Deploy a project and evaluate its assertions over a bounded window. Visual assertions use manifest-relative 8-bit RGB/RGBA PNG references (RGBA alpha is ignored) and explicit max_channel_diff and max_different_pixels tolerances. They may select a known app before capture, but capture is not synchronized to an exact animation frame. Clock, animations and network activity can vary. Framebuffer output does not include physical brightness, LED color correction, or other physical display effects; headless visual checks are not physical validation."
    )]
    Project(ArgsProject),
}

#[derive(Args)]
pub(crate) struct ArgsProject {
    #[arg(long, default_value = "awtrix.toml")]
    manifest: PathBuf,
    /// Caller-provided AWTRIX Linux executable used for an isolated local run.
    #[arg(long, env = "AWTRIX_LINUX_BIN")]
    binary: Option<PathBuf>,
    #[arg(long, requires = "binary")]
    webui: Option<PathBuf>,
    /// Bound isolated AWTRIX readiness wait (1..300 seconds).
    #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u64).range(1..=300))]
    ready_timeout_secs: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestPlan {
    #[serde(default = "default_window")]
    pub(crate) window_secs: u64,
    #[serde(default)]
    pub(crate) assertion: Vec<Assertion>,
}
fn default_window() -> u64 {
    10
}
pub(crate) fn validate_plan(plan: &TestPlan) -> crate::CliResult<()> {
    if !(1..=3600).contains(&plan.window_secs) || plan.assertion.is_empty() {
        return Err((
            "PROJECT_INVALID",
            "tests.window_secs must be 1..3600 and at least one [[tests.assertion]] is required"
                .into(),
        ));
    }
    let mut names = std::collections::HashSet::new();
    for assertion in &plan.assertion {
        if assertion.name.trim().is_empty()
            || !assertion
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            || !names.insert(assertion.name.as_str())
        {
            return Err((
                "PROJECT_INVALID",
                "assertion names must be non-empty and unique".into(),
            ));
        }
        match &assertion.kind {
            AssertionKind::Active { script_active } => validate_script_name(script_active)?,
            AssertionKind::State { script, path, .. } => {
                validate_script_name(script)?;
                if path.is_empty() || path.split('.').any(str::is_empty) {
                    return Err((
                        "PROJECT_INVALID",
                        "state assertion path must contain non-empty dot-separated keys".into(),
                    ));
                }
            }
            AssertionKind::Log { log_contains } if log_contains.is_empty() => {
                return Err(("PROJECT_INVALID", "log_contains must not be empty".into()))
            }
            AssertionKind::Log { .. } => {}
            AssertionKind::Visual {
                reference,
                max_channel_diff: _,
                max_different_pixels,
                select_app,
            } => {
                if reference.as_os_str().is_empty() || *max_different_pixels > MAX_IMAGE_PIXELS {
                    return Err((
                        "PROJECT_INVALID",
                        "visual reference/tolerance is invalid or exceeds the supported image size"
                            .into(),
                    ));
                }
                if let Some(name) = select_app {
                    validate_script_name(name)?;
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_scripts_declared(
    plan: &TestPlan,
    scripts: &[String],
) -> crate::CliResult<()> {
    for assertion in &plan.assertion {
        let name = match &assertion.kind {
            AssertionKind::Active { script_active } => Some(script_active.as_str()),
            AssertionKind::State { script, .. } => Some(script.as_str()),
            AssertionKind::Log { .. } | AssertionKind::Visual { .. } => None,
        };
        if name.is_some_and(|name| !scripts.iter().any(|script| script == name)) {
            return Err((
                "PROJECT_INVALID",
                "test assertion references a script not declared by this project".into(),
            ));
        }
    }
    Ok(())
}

fn validate_script_name(name: &str) -> crate::CliResult<()> {
    if (1..=32).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        Ok(())
    } else {
        Err((
            "PROJECT_INVALID",
            "test script names must match [A-Za-z0-9_-]{1,32}".into(),
        ))
    }
}
#[derive(Deserialize)]
pub(crate) struct Assertion {
    pub(crate) name: String,
    #[serde(flatten)]
    pub(crate) kind: AssertionKind,
}
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum AssertionKind {
    Active {
        script_active: String,
    },
    State {
        script: String,
        path: String,
        equals: Value,
    },
    Log {
        log_contains: String,
    },
    Visual {
        reference: PathBuf,
        #[serde(default)]
        max_channel_diff: u8,
        #[serde(default)]
        max_different_pixels: u64,
        #[serde(default)]
        select_app: Option<String>,
    },
}

fn decode_png(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let decoder = png::Decoder::new(
        fs::File::open(path).map_err(|_| format!("cannot read PNG {}", path.display()))?,
    );
    let mut reader = decoder
        .read_info()
        .map_err(|_| format!("invalid PNG {}", path.display()))?;
    let info = reader.info();
    let (width, height, color_type, bit_depth) =
        (info.width, info.height, info.color_type, info.bit_depth);
    let count = u64::from(width)
        .checked_mul(u64::from(height))
        .filter(|n| *n > 0 && *n <= MAX_IMAGE_PIXELS)
        .ok_or_else(|| "PNG dimensions exceed supported pixel limit".to_owned())?;
    if !matches!(color_type, png::ColorType::Rgb | png::ColorType::Rgba)
        || bit_depth != png::BitDepth::Eight
    {
        return Err("visual PNG must be 8-bit RGB or RGBA".into());
    }
    let expected_size = usize::try_from(count)
        .map_err(|_| "PNG dimensions overflow".to_owned())?
        .checked_mul(if color_type == png::ColorType::Rgba {
            4
        } else {
            3
        })
        .ok_or_else(|| "PNG decoded size overflow".to_owned())?;
    if reader.output_buffer_size() > expected_size {
        return Err("PNG decoded buffer exceeds dimensions".into());
    }
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|_| "invalid PNG pixel data".to_owned())?;
    if info.width != width || info.height != height {
        return Err("invalid PNG dimensions".into());
    }
    let pixels = match info.color_type {
        #[expect(clippy::chunks_exact_to_as_chunks)]
        png::ColorType::Rgba => buffer[..info.buffer_size()]
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
        png::ColorType::Rgb => buffer[..info.buffer_size()].to_vec(),
        _ => return Err("visual PNG must be RGB or RGBA".into()),
    };
    if pixels.len() as u64 != count * 3 {
        return Err("invalid PNG pixel buffer".into());
    }
    Ok((width, height, pixels))
}

fn compare_pixels(reference: &[u8], actual: &[u8], tolerance: u8) -> (u64, u8) {
    let mut different = 0;
    let mut maximum = 0;
    #[expect(clippy::chunks_exact_to_as_chunks)]
    for (expected, observed) in reference.chunks_exact(3).zip(actual.chunks_exact(3)) {
        let delta = expected
            .iter()
            .zip(observed)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        maximum = maximum.max(delta);
        if delta > tolerance {
            different += 1;
        }
    }
    (different, maximum)
}

fn artifact_path(reference: &Path, name: &str) -> PathBuf {
    let stem = reference
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("reference");
    reference.with_file_name(format!("{stem}.{name}.capture.png"))
}

fn write_difference_png(
    path: &Path,
    width: u32,
    height: u32,
    expected: &[u8],
    actual: &[u8],
) -> crate::CliResult<()> {
    let difference: Vec<u8> = expected
        .iter()
        .zip(actual)
        .map(|(a, b)| a.abs_diff(*b))
        .collect();
    let mut encoded = Vec::new();
    let mut encoder = png::Encoder::new(&mut encoded, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|_| ("FILE_WRITE", "cannot encode visual diff PNG header".into()))?;
    writer
        .write_image_data(&difference)
        .map_err(|_| ("FILE_WRITE", "cannot encode visual diff PNG pixels".into()))?;
    drop(writer);
    crate::screen::persist_sibling(&path.to_path_buf(), &encoded)
}

pub(crate) fn run(command: &Command, cli: &crate::Cli) -> crate::CliResult<Value> {
    let Command::Project(args) = command;
    let explicit_target = std::env::args().any(|a| a == "--target" || a.starts_with("--target="));
    let target = explicit_target.then_some(cli.target.as_deref()).flatten();
    let explicit_binary = std::env::args().any(|a| a == "--binary" || a.starts_with("--binary="));
    if explicit_binary && target.is_some() {
        return Err((
            "ARGUMENT",
            "choose --binary for isolated headless tests or --target for external reuse".into(),
        ));
    }
    let use_isolated = !explicit_target;
    if use_isolated && args.binary.is_none() {
        return Err((
            "TARGET_REQUIRED",
            "provide AWTRIX_LINUX_BIN/--binary for isolated tests or explicit --target for external reuse; a default profile is never used".into(),
        ));
    }
    let bytes =
        fs::read(&args.manifest).map_err(|_| ("PROJECT_INVALID", "cannot read manifest".into()))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| ("PROJECT_INVALID", "manifest is not UTF-8".into()))?;
    let document: toml::Value =
        toml::from_str(text).map_err(|_| ("PROJECT_INVALID", "manifest TOML is invalid".into()))?;
    validate_assertion_fields(&document)?;
    let project_scripts = document
        .get("scripts")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|script| {
            script
                .get("name")
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    let mut plan: TestPlan = document
        .get("tests")
        .cloned()
        .ok_or((
            "PROJECT_INVALID",
            "manifest requires [tests] and [[tests.assertion]]".into(),
        ))?
        .try_into()
        .map_err(|error: toml::de::Error| {
            (
                "PROJECT_INVALID",
                format!("test assertions are invalid: {error}"),
            )
        })?;
    validate_plan(&plan)?;
    let base = args.manifest.parent().unwrap_or_else(|| Path::new("."));
    let mut reference_paths = Vec::new();
    for assertion in &mut plan.assertion {
        if let AssertionKind::Visual { reference, .. } = &mut assertion.kind {
            if reference.is_absolute() {
                return Err((
                    "PROJECT_INVALID",
                    "visual references must be relative to manifest".into(),
                ));
            }
            *reference = base.join(&*reference);
            decode_png(reference).map_err(|e| ("IMAGE_INVALID", e))?;
            reference_paths.push(fs::canonicalize(&*reference).map_err(|_| {
                (
                    "IMAGE_INVALID",
                    format!("cannot resolve reference PNG {}", reference.display()),
                )
            })?);
        }
    }
    for assertion in &plan.assertion {
        if let AssertionKind::Visual { reference, .. } = &assertion.kind {
            for artifact in [
                artifact_path(reference, &assertion.name),
                artifact_path(reference, &assertion.name).with_extension("diff.png"),
            ] {
                let parent = artifact.parent().unwrap_or_else(|| Path::new("."));
                let canonical_artifact = fs::canonicalize(parent)
                    .map_err(|_| {
                        (
                            "FILE_WRITE",
                            "cannot resolve visual artifact directory".into(),
                        )
                    })?
                    .join(artifact.file_name().unwrap_or_default());
                if reference_paths.contains(&canonical_artifact) {
                    return Err((
                        "PROJECT_INVALID",
                        "visual artifact path would overwrite a reference PNG".into(),
                    ));
                }
            }
        }
    }
    let interrupted = Arc::new(AtomicBool::new(false));
    let signal = interrupted.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::SeqCst)).map_err(|e| {
        (
            "SIGNAL",
            format!("cannot install test interrupt handler: {e}"),
        )
    })?;
    if let Some(binary) = args.binary.as_ref().filter(|_| use_isolated) {
        let temp = tempfile::tempdir()
            .map_err(|_| ("TEMP_DIR", "cannot create isolated test directory".into()))?;
        let data = temp.path().join("data");
        fs::create_dir(&data).map_err(|_| {
            (
                "TEMP_DIR",
                "cannot create isolated headless data directory".into(),
            )
        })?;
        let tracking = temp.path().join("tracking");
        fs::create_dir(&tracking).map_err(|_| {
            (
                "TEMP_DIR",
                "cannot create isolated project tracking directory".into(),
            )
        })?;
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .map_err(|_| ("PORT", "cannot reserve an isolated loopback port".into()))?;
        let port = listener
            .local_addr()
            .map_err(|_| ("PORT", "cannot determine isolated loopback port".into()))?
            .port();
        drop(listener);
        let _session = crate::headless::start_isolated(
            binary.clone(),
            args.webui.clone(),
            data,
            port,
            args.ready_timeout_secs,
            interrupted.clone(),
        )?;
        let target = _session.target.as_str();
        let data_path = _session.data.clone();
        let resolved = crate::profiles::resolve(
            Some(target),
            cli.profile.as_deref(),
            cli.username.as_deref(),
            cli.password.as_deref(),
        )?;
        let api = crate::ApiClient::new(cli, Some(target), resolved.username, resolved.password)?;
        let deploy = crate::project::deploy_test_project(&args.manifest, cli, target, &tracking)?;
        if interrupted.load(Ordering::SeqCst) {
            return Err(("INTERRUPTED", "declarative test interrupted after deployment; owned headless child will be stopped".into()));
        }
        let mut result = evaluate(&api, &plan, &project_scripts, &interrupted)?;
        result["target"] = json!(target);
        result["target_mode"] = json!("isolated-headless");
        result["deployment"] = deploy;
        result["isolation"] = json!({"data_path":data_path,"project_tracking_path":tracking,"fresh_per_run":true,"removed_after_command":true});
        return Ok(result);
    }
    let target = target.ok_or(("TARGET_REQUIRED", "an explicit target is required".into()))?;
    let resolved = crate::profiles::resolve(
        Some(target),
        cli.profile.as_deref(),
        cli.username.as_deref(),
        cli.password.as_deref(),
    )?;
    let api = crate::ApiClient::new(cli, Some(target), resolved.username, resolved.password)?;
    let deploy = crate::project::deploy_test_project(
        &args.manifest,
        cli,
        target,
        args.manifest
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
    )?;
    if interrupted.load(Ordering::SeqCst) {
        return Err((
            "INTERRUPTED",
            "declarative test interrupted after deployment".into(),
        ));
    }
    let mut result = evaluate(&api, &plan, &project_scripts, &interrupted)?;
    result["target"] = json!(target);
    result["target_mode"] = json!("external-reuse");
    result["deployment"] = deploy;
    Ok(result)
}

fn validate_assertion_fields(document: &toml::Value) -> crate::CliResult<()> {
    let entries = document
        .get("tests")
        .and_then(|tests| tests.get("assertion"))
        .and_then(toml::Value::as_array)
        .ok_or((
            "PROJECT_INVALID",
            "tests.assertion must be a non-empty array of assertion tables".into(),
        ))?;
    let allowed = [
        "name",
        "script_active",
        "script",
        "path",
        "equals",
        "log_contains",
        "reference",
        "max_channel_diff",
        "max_different_pixels",
        "select_app",
    ];
    for entry in entries {
        let table = entry.as_table().ok_or((
            "PROJECT_INVALID",
            "each test assertion must be a TOML table".into(),
        ))?;
        if table.keys().any(|key| !allowed.contains(&key.as_str())) {
            return Err(("PROJECT_INVALID", "test assertions accept only name, script_active, script/path/equals, or log_contains; commands and unknown keys are not supported".into()));
        }
        let has_active = table.contains_key("script_active");
        let has_log = table.contains_key("log_contains");
        let state_count = ["script", "path", "equals"]
            .iter()
            .filter(|key| table.contains_key(**key))
            .count();
        let valid = table.contains_key("name")
            && ((has_active && !has_log && state_count == 0)
                || (has_log && !has_active && state_count == 0)
                || (!has_active && !has_log && state_count == 3)
                || (!has_active
                    && !has_log
                    && state_count == 0
                    && table.contains_key("reference")
                    && table.keys().all(|key| {
                        [
                            "name",
                            "reference",
                            "max_channel_diff",
                            "max_different_pixels",
                            "select_app",
                        ]
                        .contains(&key.as_str())
                    })));
        if !valid {
            return Err((
                "PROJECT_INVALID",
                "each assertion must declare exactly one supported assertion form".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_test_fields(text: &str) -> crate::CliResult<()> {
    let document: toml::Value =
        toml::from_str(text).map_err(|_| ("PROJECT_INVALID", "manifest TOML is invalid".into()))?;
    if document.get("tests").is_some() {
        validate_assertion_fields(&document)?;
    }
    Ok(())
}

fn evaluate(
    api: &crate::ApiClient,
    plan: &TestPlan,
    project_scripts: &[String],
    interrupted: &AtomicBool,
) -> crate::CliResult<Value> {
    let started = Instant::now();
    let deadline = Duration::from_secs(plan.window_secs);
    let mut results: Vec<Value> = plan
        .assertion
        .iter()
        .map(|a| json!({"name":a.name,"passed":false,"observed":null}))
        .collect();
    let mut errors: Vec<Value> = Vec::new();
    let mut active_failed = vec![false; plan.assertion.len()];
    while started.elapsed() < deadline {
        if interrupted.load(Ordering::SeqCst) {
            return Err((
                "INTERRUPTED",
                "declarative test interrupted; owned headless child will be stopped".into(),
            ));
        }
        let apps_inventory = if !project_scripts.is_empty() {
            match api.get_with_timeout("/api/v1/apps", deadline.saturating_sub(started.elapsed())) {
                Ok(apps) => {
                    for script in project_scripts {
                        let app = apps.as_array().and_then(|all| {
                            all.iter().find(|app| {
                                app.get("name").and_then(Value::as_str) == Some(script)
                                    && app.get("origin").and_then(Value::as_str) == Some("script")
                            })
                        });
                        if let Some(runtime_error) = app
                            .and_then(|app| app.get("error"))
                            .filter(|error| !error.is_null())
                        {
                            let error = json!({"script":script,"code":"BERRY_ERROR","runtime_error":runtime_error});
                            if !errors.contains(&error) {
                                errors.push(error);
                            }
                        }
                    }
                    Some(apps)
                }
                Err((code, message)) => {
                    let error =
                        json!({"code":code,"message":message,"phase":"runtime_error_observation"});
                    if !errors.contains(&error) {
                        errors.push(error);
                    }
                    None
                }
            }
        } else {
            None
        };
        for (index, assertion) in plan.assertion.iter().enumerate() {
            if interrupted.load(Ordering::SeqCst) {
                return Err((
                    "INTERRUPTED",
                    "declarative test interrupted; owned headless child will be stopped".into(),
                ));
            }
            if started.elapsed() >= deadline {
                break;
            }
            let remaining = deadline.saturating_sub(started.elapsed());
            let outcome: Result<(bool, Value), (&str, String)> = match &assertion.kind {
                AssertionKind::Active { script_active } => (|| {
                    let system = api.get_with_timeout("/api/v1/system", remaining)?;
                    let apps = apps_inventory.as_ref().ok_or(("INVALID_RESPONSE", "project app inventory unavailable".into()))?;
                    let app = apps.as_array().and_then(|all| all.iter().find(|app| app.get("name").and_then(Value::as_str) == Some(script_active) && app.get("origin").and_then(Value::as_str) == Some("script")));
                    let error_value = app.and_then(|app| app.get("error"));
                    let berry_error = error_value.filter(|e| !e.is_null()).cloned();
                    let error_is_null = error_value.is_some_and(Value::is_null);
                    let active = system.get("scriptingEnabled").and_then(Value::as_bool) == Some(true) && app.is_some_and(|app| app.get("enabled").and_then(Value::as_bool) == Some(true)) && error_is_null;
                    if !active { active_failed[index] = true; }
                    Ok((active, json!({"active":active,"runtime_error":berry_error,"error_field_present_and_null":error_is_null,"definition":"active means scriptingEnabled and an enabled origin=script app with explicit error=null at every poll"})))
                })(),
                AssertionKind::State { script, path, equals } => api.get_with_timeout(&format!("/api/v1/apps/{script}/data"), remaining).map(|data| {
                    if results[index]["passed"] == true {
                        return (true, results[index]["observed"].clone());
                    }
                    let actual = path.split('.').try_fold(&data, |v, key| v.get(key)).cloned();
                    let now_passed = actual.as_ref() == Some(equals);
                    (results[index]["passed"] == true || now_passed, json!({"actual":actual,"expected":equals,"matched":now_passed}))
                }),
                AssertionKind::Log { log_contains } => api.get_with_timeout("/api/v1/logs?after=0", remaining).map(|logs| {
                    let lines = logs.get("lines").and_then(Value::as_array).cloned().unwrap_or_default();
                    let found = lines.iter().any(|line| line.as_str().is_some_and(|line| line.contains(log_contains)));
                    let mut evidence = results[index]["observed"]["observed_lines"].as_array().cloned().unwrap_or_default();
                    for line in lines.into_iter().filter(|line| line.as_str().is_some_and(|line| line.contains(log_contains))) {
                        if !evidence.contains(&line) { evidence.push(line); }
                    }
                    if evidence.len() > 34 { evidence.drain(..evidence.len() - 34); }
                    let matched = results[index]["passed"] == true || found;
                    (matched, json!({"contains":log_contains,"matched":matched,"observed_lines":evidence,"history_limit":34,"exhaustive":false}))
                }),
                AssertionKind::Visual { reference, max_channel_diff, max_different_pixels, select_app } => (|| {
                    if results[index]["passed"] == true {
                        return Ok((true, results[index]["observed"].clone()));
                    }
                    if let Some(name) = select_app {
                        validate_script_name(name)?;
                        let left = deadline.saturating_sub(started.elapsed());
                        if left.is_zero() { return Err(("TIMEOUT", "visual assertion deadline expired before app selection".into())); }
                        // AWTRIX NG OpenAPI: PUT /api/v1/apps/active accepts {name, fast}.
                        api.mutate_with_timeout(reqwest::Method::PUT, "/api/v1/apps/active", &json!({"name":name,"fast":true}), left)?;
                    }
                    let output = artifact_path(reference, &assertion.name);
                    if output == *reference { return Err(("PROJECT_INVALID", "visual capture must not overwrite its reference image".into())); }
                    let left = deadline.saturating_sub(started.elapsed());
                    if left.is_zero() { return Err(("TIMEOUT", "visual assertion deadline expired before capture".into())); }
                    let capture = crate::screen::capture_png(api, &output, left)?;
                    let actual = decode_png(&output).map_err(|e| ("IMAGE_INVALID", e))?;
                    let expected = decode_png(reference).map_err(|e| ("IMAGE_INVALID", e))?;
                    if (actual.0, actual.1) != (expected.0, expected.1) {
                        return Err(("IMAGE_DIMENSIONS", format!("capture dimensions {}x{} differ from reference {}x{}", actual.0, actual.1, expected.0, expected.1)));
                    }
                    let (different, maximum) = compare_pixels(&expected.2, &actual.2, *max_channel_diff);
                    let diff_path = output.with_extension("diff.png");
                    write_difference_png(&diff_path, actual.0, actual.1, &expected.2, &actual.2)?;
                    let passed = different <= *max_different_pixels;
                    Ok((passed, json!({"capture":capture,"diff":{"path":diff_path,"format":"png"},"reference":reference,"different_pixels":different,"max_channel_difference":maximum,"allowed_different_pixels":max_different_pixels,"allowed_channel_difference":max_channel_diff,"dimensions":{"width":actual.0,"height":actual.1}})))
                })(),
            };
            match outcome {
                Ok((passed, observed)) => {
                    let total = passed && !active_failed[index];
                    if let Some(runtime_error) = observed
                        .get("runtime_error")
                        .filter(|value| !value.is_null())
                    {
                        let error = json!({"assertion":assertion.name,"code":"BERRY_ERROR","runtime_error":runtime_error});
                        if !errors.contains(&error) {
                            errors.push(error);
                        }
                    }
                    results[index]["passed"] = json!(total);
                    results[index]["observed"] = observed;
                }
                Err((code, message)) => {
                    let error = json!({"assertion":assertion.name,"code":code,"message":message});
                    if !errors.contains(&error) {
                        errors.push(error);
                    }
                    results[index]["passed"] = json!(false);
                    results[index]["observed"] = json!({"collection_error":code});
                }
            }
        }
        if started.elapsed() < deadline {
            thread::sleep(
                Duration::from_millis(200).min(deadline.saturating_sub(started.elapsed())),
            );
        }
    }
    Ok(
        json!({"passed":errors.is_empty() && results.iter().all(|r| r["passed"] == true),"assertions":results,"errors":errors,"observed_window":{"duration_secs":started.elapsed().as_secs_f64(),"requested_secs":plan.window_secs,"complete":started.elapsed() >= deadline},"logs":{"history_limit":34,"exhaustive":false},"runtime_success_guaranteed":false}),
    )
}

#[allow(unreachable_code)]
pub(crate) fn describe() -> Value {
    // Kept here so offline structured discovery has the same visual contract as the runner.
    return json!({"command":"test project","parameters":{"--manifest":"project TOML path","--target":"explicit external target, never stopped","--binary":"caller-supplied AWTRIX Linux executable","--webui":"optional Web UI asset path","--ready-timeout-secs":"1..300 seconds"},"outputs":{"passed":"overall boolean","assertions":[{"name":"safe unique assertion name","passed":"boolean","observed":"metrics plus capture and diff artifact paths; no pixel arrays"}],"errors":[{"code":"IMAGE_INVALID, IMAGE_DIMENSIONS, or collection error; distinct from render mismatch"}]},"schema":{"[tests]":{"window_secs":"integer 1..3600; bounds all observation requests including app selection and capture"},"[[tests.assertion]] visual":{"name":"unique alphanumeric/underscore/hyphen name","reference":"required 8-bit RGB/RGBA PNG path relative to manifest; RGBA alpha is ignored","max_channel_diff":"0..255, default 0; pixel differs if any RGB channel absolute difference is greater than this","max_different_pixels":"0..4,194,304, default 0; maximum differing pixel count","select_app":"optional app name; PUT /api/v1/apps/active with {name,fast:true} before capture"},"artifact":"<reference stem>.<assertion name>.capture.png plus same stem .diff.png beside reference; reference is never overwritten"},"examples":["[[tests.assertion]]\nname='berry-display'\nreference='visual/berry.png'\nmax_channel_diff=2\nmax_different_pixels=1\nselect_app='main'"],"limitations":["No exact-frame synchronization is promised. Clock, animation and network activity affect captures. Framebuffer PNGs do not model physical brightness or LED corrections."]});
    json!({"command":"test project","parameters":{"--manifest":"project TOML path (default awtrix.toml)","--target":"explicit external HTTP target; required in reuse mode; never stopped","--binary":"caller-provided AWTRIX Linux executable for isolated tests (or AWTRIX_LINUX_BIN)","--webui":"optional AWTRIX web UI asset path; only valid with --binary","--ready-timeout-secs":"isolated readiness bound 1..300 seconds (default 15)","--username":"HTTP Basic username, or AWTRIX_USERNAME","--password":"HTTP Basic password, or AWTRIX_PASSWORD","--timeout":"maximum HTTP request time in milliseconds (default 3000)","--json":"compact JSON result","--fields":"comma-separated output fields","--profile":"not used to select external test target"},"inputs":["project TOML and relative script/module/resource/config files","bounded AWTRIX NG HTTP state, app inventory and log routes"],"outputs":{"passed":"boolean","assertions":[{"name":"string","passed":"boolean","observed":"assertion-specific evidence"}],"errors":[{"assertion":"name","code":"Berry or collection error","runtime_error":"reported device diagnostic when available"}],"deployment":"ordinary additive project deployment report","isolation":{"data_path":"fresh temporary data directory (isolated mode)","project_tracking_path":"temporary project tracking state (isolated mode)","removed_after_command":true},"observed_window":{"requested_secs":"integer","duration_secs":"number","complete":"boolean"},"logs":{"history_limit":34,"exhaustive":false},"runtime_success_guaranteed":false},"schema":{"[tests]":{"window_secs":"integer 1..3600, default 10"},"[[tests.assertion]]":{"name":"string","script_active":"script name; must remain enabled with origin=script and explicit error=null at every poll","script + path + equals":"script data value equals JSON value at least once during the window","log_contains":"literal substring observed in current bounded log buffer during the window"}},"examples":["awtrix --json test project --manifest awtrix.toml --binary ./awtrix-linux","awtrix --json --target http://192.0.2.1 test project --manifest awtrix.toml"],"prerequisites":["test plan with at least one assertion","caller-provided AWTRIX Linux executable for isolated mode; or explicit --target URL for external reuse"],"limitations":["log ring buffer retains at most 34 lines; historical completeness is not guaranteed","active means no error observed at each poll, not proof of general correctness","headless does not validate sensors, audio, or physical ESP32 resource budgets"]})
}
