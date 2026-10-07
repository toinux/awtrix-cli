//! TOML project loading, complete local preflight, and additive device deployment.
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub type Result<T> = crate::CliResult<T>;

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a minimal valid Berry project.
    Init {
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
    },
    /// Validate manifest and every local dependency without contacting a device.
    Validate {
        #[arg(long, default_value = "awtrix.toml")]
        manifest: PathBuf,
    },
    /// Deploy declared content in dependency order; never deletes undeclared remote content.
    Deploy {
        #[arg(long, default_value = "awtrix.toml")]
        manifest: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Explicitly delete obsolete entries recorded by a successful project deployment.
    Prune {
        #[arg(long, default_value = "awtrix.toml")]
        manifest: PathBuf,
        /// Show planned deletions without changing the device or tracking state.
        #[arg(long)]
        dry_run: bool,
    },
    /// Explicitly release local ownership of items whose remote effect is uncertain.
    Reconcile {
        #[arg(long, default_value = "awtrix.toml")]
        manifest: PathBuf,
        /// Forget uncertain ownership without claiming or changing remote state.
        #[arg(long, required = true)]
        forget_uncertain: bool,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    project: Identity,
    #[serde(default)]
    target: Target,
    #[serde(default)]
    scripts: Vec<Script>,
    #[serde(default)]
    modules: Vec<Module>,
    #[serde(default)]
    resources: Vec<Resource>,
    #[serde(default)]
    config: Vec<Config>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    name: String,
    version: String,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    profile: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Script {
    name: String,
    file: PathBuf,
    #[serde(default)]
    create: bool,
    expected_source_file: Option<PathBuf>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Module {
    name: String,
    file: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resource {
    path: String,
    file: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    script: String,
    file: PathBuf,
}

struct Loaded {
    manifest: Manifest,
    sources: Vec<String>,
    expected: Vec<Option<String>>,
    modules: Vec<String>,
    resources: Vec<Vec<u8>>,
    configs: Vec<Value>,
}

// Tracking is project-local (never beside the profile/credential config), versioned,
// and keyed by manifest identity plus the effective normalized HTTP endpoint.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tracking {
    version: u32,
    project: String,
    target: String,
    #[serde(default)]
    device_id: Option<String>,
    entries: Vec<String>,
    #[serde(default)]
    uncertain: Vec<String>,
}

fn tracking_path(manifest: &Path, project: &str, target: &str) -> PathBuf {
    // FNV is only a filename discriminator, not a security primitive; the full identity
    // is independently checked inside the state file, so collisions fail closed.
    let key = format!("{project}\0{target}");
    let hash = key.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    manifest.with_file_name(format!(".awtrix-tracking-{hash:016x}.json"))
}

fn read_tracking(
    path: &Path,
    project: &str,
    target: &str,
    device_id: Option<&str>,
) -> Result<Tracking> {
    let bytes = fs::read(path).map_err(|_| {
        (
            "TRACKING_INVALID",
            "tracking state is missing or unreadable; refusing deletion".into(),
        )
    })?;
    let state: Tracking = serde_json::from_slice(&bytes).map_err(|_| {
        (
            "TRACKING_INVALID",
            "tracking state is corrupt; refusing deletion".into(),
        )
    })?;
    if state.version != 2
        || state.project != project
        || state.target != target
        || device_id.is_some_and(|expected| state.device_id.as_deref() != Some(expected))
        || state.entries.iter().any(|e| !valid_tracked_entry(e))
        || state.uncertain.iter().any(|e| !valid_uncertain_entry(e))
    {
        return Err((
            "TRACKING_INVALID",
            "tracking identity or entries do not match; refusing deletion".into(),
        ));
    }
    Ok(state)
}

fn valid_tracked_entry(entry: &str) -> bool {
    if let Some(name) = entry
        .strip_prefix("script:")
        .or_else(|| entry.strip_prefix("module:"))
    {
        return valid_name(name).is_ok();
    }
    entry.strip_prefix("resource:").is_some_and(|path| {
        path.starts_with('/')
            && !path.split('/').any(|p| p == "..")
            && path.rsplit('/').next().is_some_and(|p| !p.is_empty())
    })
}

fn valid_uncertain_entry(entry: &str) -> bool {
    valid_tracked_entry(entry)
        || entry
            .strip_prefix("config:")
            .is_some_and(|name| valid_name(name).is_ok())
}

fn save_tracking(path: &Path, state: &Tracking) -> Result<()> {
    let bytes = serde_json::to_vec(state)
        .map_err(|_| ("TRACKING_WRITE", "cannot encode tracking state".into()))?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|_| {
        (
            "TRACKING_WRITE",
            "cannot create private tracking state temporary file".into(),
        )
    })?;
    tmp.write_all(&bytes)
        .map_err(|_| ("TRACKING_WRITE", "cannot write tracking state".into()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o600))
            .map_err(|_| ("TRACKING_WRITE", "cannot secure tracking state".into()))?;
    }
    tmp.persist(path).map(|_| ()).map_err(|_| {
        (
            "TRACKING_WRITE",
            "cannot atomically replace tracking state".into(),
        )
    })
}

fn device_id(api: &crate::ApiClient) -> Result<Option<String>> {
    let value = api.get("/api/v1/device")?;
    Ok(["uid", "deviceId", "serial"].iter().find_map(|key| {
        value
            .get(key)
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
    }))
}

fn load(path: &Path) -> Result<Loaded> {
    let bytes = fs::read(path).map_err(|_| ("PROJECT_INVALID", "cannot read manifest".into()))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| ("PROJECT_INVALID", "manifest is not UTF-8".into()))?;
    let manifest: Manifest = toml::from_str(text).map_err(|_| {
        (
            "PROJECT_INVALID",
            "manifest TOML or schema is invalid".into(),
        )
    })?;
    if manifest.project.name.trim().is_empty() || manifest.project.version.trim().is_empty() {
        return Err((
            "PROJECT_INVALID",
            "project name and version are required".into(),
        ));
    }
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut names = HashSet::new();
    let mut sources = Vec::new();
    let mut expected = Vec::new();
    for s in &manifest.scripts {
        valid_name(&s.name)?;
        if !names.insert(s.name.as_str()) {
            return Err(("PROJECT_INVALID", "duplicate script/module name".into()));
        }
        let source = read_text(base, &s.file)?;
        if source.trim().is_empty() {
            return Err(("PROJECT_INVALID", "script source must not be empty".into()));
        }
        let reference = s
            .expected_source_file
            .as_deref()
            .map(|p| read_text(base, p))
            .transpose()?;
        if s.create && reference.is_some() {
            return Err((
                "PROJECT_INVALID",
                "create=true cannot be combined with expected_source_file".into(),
            ));
        }
        if !s.create && reference.is_none() {
            return Err((
                "PROJECT_INVALID",
                "script needs create=true or expected_source_file".into(),
            ));
        }
        sources.push(source);
        expected.push(reference);
    }
    let mut modules = Vec::new();
    for m in &manifest.modules {
        valid_name(&m.name)?;
        if !names.insert(m.name.as_str()) {
            return Err(("PROJECT_INVALID", "duplicate script/module name".into()));
        }
        let source = read_text(base, &m.file)?;
        if !crate::resources::declares_module(&source) {
            return Err((
                "PROJECT_INVALID",
                "module source must begin with # @module".into(),
            ));
        }
        modules.push(source);
    }
    let mut resource_paths = HashSet::new();
    let resources = manifest
        .resources
        .iter()
        .map(|r| {
            if !r.path.starts_with('/')
                || r.path.split('/').any(|part| part == "..")
                || r.path.rsplit('/').next().is_none_or(str::is_empty)
                || !resource_paths.insert(r.path.as_str())
            {
                return Err((
                    "PROJECT_INVALID",
                    "resource path must be unique, absolute, traversal-free, and include a filename".into(),
                ));
            }
            let bytes = read_bytes(base, &r.file, "cannot read declared resource")?;
            let device_dir = r.path.rsplit_once('/').map(|(dir, _)| if dir.is_empty() { "/" } else { dir }).unwrap_or("/");
            crate::resources::validate_icon(device_dir, Path::new(&r.path), &bytes)?;
            Ok(bytes)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut config_targets = HashSet::new();
    let configs = manifest
        .config
        .iter()
        .map(|c| {
            valid_name(&c.script)?;
            if !manifest.scripts.iter().any(|s| s.name == c.script) {
                return Err((
                    "PROJECT_INVALID",
                    "configuration references an undeclared script".into(),
                ));
            }
            if !config_targets.insert(c.script.as_str()) {
                return Err((
                    "PROJECT_INVALID",
                    "each script may have at most one config entry".into(),
                ));
            }
            let data = read_bytes(base, &c.file, "cannot read declared config")?;
            let value: Value = serde_json::from_slice(&data)
                .map_err(|_| ("PROJECT_INVALID", "config must be valid JSON".into()))?;
            if !value.is_object() {
                return Err(("PROJECT_INVALID", "config must be a JSON object".into()));
            }
            Ok(value)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Loaded {
        manifest,
        sources,
        expected,
        modules,
        resources,
        configs,
    })
}

fn read_text(base: &Path, path: &Path) -> Result<String> {
    if path.is_absolute() {
        return Err(("PROJECT_INVALID", "manifest paths must be relative".into()));
    }
    fs::read_to_string(base.join(path))
        .map_err(|_| ("PROJECT_INVALID", "cannot read declared UTF-8 file".into()))
}
fn read_bytes(base: &Path, path: &Path, message: &'static str) -> Result<Vec<u8>> {
    if path.is_absolute() {
        return Err(("PROJECT_INVALID", "manifest paths must be relative".into()));
    }
    fs::read(base.join(path)).map_err(|_| ("PROJECT_INVALID", message.into()))
}
fn valid_name(name: &str) -> Result<()> {
    if (1..=32).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        Ok(())
    } else {
        Err((
            "PROJECT_INVALID",
            "name must match [A-Za-z0-9_-]{1,32}".into(),
        ))
    }
}

pub fn run(command: &Command, cli: &crate::Cli) -> Result<Value> {
    match command {
        Command::Init { path, name } => init(path, name.as_deref()),
        Command::Validate { manifest } => {
            let p = load(manifest)?;
            Ok(
                json!({"valid":true,"project":p.manifest.project.name,"scripts":p.sources.len(),"modules":p.modules.len(),"resources":p.resources.len(),"config":p.configs.len()}),
            )
        }
        Command::Deploy { manifest, force } => deploy(load(manifest)?, manifest, cli, *force),
        Command::Prune { manifest, dry_run } => prune(load(manifest)?, manifest, cli, *dry_run),
        Command::Reconcile {
            manifest,
            forget_uncertain,
        } => reconcile(load(manifest)?, manifest, cli, *forget_uncertain),
    }
}

fn init(path: &Path, requested: Option<&str>) -> Result<Value> {
    if path.join("awtrix.toml").exists() || path.join("src/main.be").exists() {
        return Err((
            "PROJECT_EXISTS",
            "refusing to overwrite an existing project manifest or starter script".into(),
        ));
    }
    fs::create_dir_all(path)
        .map_err(|_| ("FILE_WRITE", "cannot create project directory".into()))?;
    let name = requested
        .map(str::to_owned)
        .or_else(|| path.file_name().and_then(|s| s.to_str()).map(str::to_owned))
        .unwrap_or_else(|| "awtrix-project".into());
    let manifest = format!("[project]\nname = {name:?}\nversion = \"0.1.0\"\n\n[target]\n# profile = \"desk\"\n\n[[scripts]]\nname = \"main\"\nfile = \"src/main.be\"\ncreate = true\n");
    fs::create_dir_all(path.join("src"))
        .map_err(|_| ("FILE_WRITE", "cannot create source directory".into()))?;
    fs::write(path.join("awtrix.toml"), manifest)
        .map_err(|_| ("FILE_WRITE", "cannot write manifest".into()))?;
    fs::write(path.join("src/main.be"), "# @name main\nclass ProjectApp\n  def init()\n    print(\"project ready\")\n  end\n  def draw()\n  end\n  def loop()\n    return true\n  end\nend\nreturn ProjectApp()\n").map_err(|_| ("FILE_WRITE", "cannot write Berry script".into()))?;
    load(&path.join("awtrix.toml"))?;
    Ok(
        json!({"path":path,"manifest":path.join("awtrix.toml"),"script":path.join("src/main.be"),"valid":true}),
    )
}

fn deploy(p: Loaded, manifest_path: &Path, cli: &crate::Cli, force: bool) -> Result<Value> {
    // The entire project has been loaded and validated before target resolution or the first mutation.
    let explicit = std::env::args().any(|a| a == "--target" || a.starts_with("--target="));
    let selected = crate::profiles::resolve_project(
        if explicit {
            cli.target.as_deref()
        } else {
            None
        },
        cli.profile.as_deref(),
        p.manifest.target.profile.as_deref(),
        cli.username.as_deref(),
        cli.password.as_deref(),
    )?;
    let api = crate::ApiClient::new(
        cli,
        selected.target.as_deref(),
        selected.username,
        selected.password,
    )?;
    let target = reqwest::Url::parse(selected.target.as_deref().unwrap_or_default())
        .map_err(|_| ("ARGUMENT", "invalid selected target".into()))?
        .to_string()
        .trim_end_matches('/')
        .to_owned();
    let device_id = device_id(&api)?;
    let state_path = tracking_path(manifest_path, &p.manifest.project.name, &target);
    let mut entries = match fs::read(&state_path) {
        Ok(_) => {
            read_tracking(
                &state_path,
                &p.manifest.project.name,
                &target,
                device_id.as_deref(),
            )?
            .entries
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(_) => return Err((
            "TRACKING_INVALID",
            "tracking state is unreadable; refusing deployment that could desynchronize tracking"
                .into(),
        )),
    };
    let mut operations = Vec::new();
    for m in &p.manifest.modules {
        operations.push(format!("module:{}", m.name));
    }
    for r in &p.manifest.resources {
        operations.push(format!("resource:{}", r.path));
    }
    for s in &p.manifest.scripts {
        operations.push(format!("script:{}", s.name));
    }
    for c in &p.manifest.config {
        operations.push(format!("config:{}", c.script));
    }
    let mut succeeded = Vec::new();
    let mut index = 0usize;
    let execution: Result<()> = (|| {
        for (i, m) in p.manifest.modules.iter().enumerate() {
            index += 1;
            let v = api.raw_put(&format!("/api/v1/apps/script/{}", m.name), &p.modules[i])?;
            if v.get("error").is_some_and(|e| !e.is_null()) {
                return Err((
                    "BERRY_ERROR",
                    "module deployment returned a Berry error".into(),
                ));
            }
            let operation = operations[index - 1].clone();
            succeeded.push(operation.clone());
            persist_success(
                &state_path,
                &p.manifest.project.name,
                &target,
                device_id.as_deref(),
                &mut entries,
                &operation,
            )?;
        }
        for (i, r) in p.manifest.resources.iter().enumerate() {
            index += 1;
            let filename = r
                .path
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .ok_or(("PROJECT_INVALID", "resource path requires filename".into()))?;
            let dir = r
                .path
                .rsplit_once('/')
                .map(|(d, _)| if d.is_empty() { "/" } else { d })
                .unwrap_or("/");
            let form = reqwest::blocking::multipart::Form::new().part(
                "file",
                reqwest::blocking::multipart::Part::bytes(p.resources[i].clone())
                    .file_name(filename.to_owned()),
            );
            api.multipart_post(&format!("/api/v1/files?dir={}", encode(dir)), form)?;
            let operation = operations[index - 1].clone();
            succeeded.push(operation.clone());
            persist_success(
                &state_path,
                &p.manifest.project.name,
                &target,
                device_id.as_deref(),
                &mut entries,
                &operation,
            )?;
        }
        for (i, s) in p.manifest.scripts.iter().enumerate() {
            index += 1;
            crate::scripts::deploy_project(
                &api,
                &s.name,
                &p.sources[i],
                p.expected[i].as_deref(),
                s.create,
                force,
            )?;
            let operation = operations[index - 1].clone();
            succeeded.push(operation.clone());
            persist_success(
                &state_path,
                &p.manifest.project.name,
                &target,
                device_id.as_deref(),
                &mut entries,
                &operation,
            )?;
        }
        for (i, c) in p.manifest.config.iter().enumerate() {
            index += 1;
            api.mutate(
                reqwest::Method::PATCH,
                &format!("/api/v1/apps/{}/config", c.script),
                &p.configs[i],
            )?;
            succeeded.push(operations[index - 1].clone());
        }
        Ok(())
    })();
    if let Err((code, message)) = execution {
        let tracking_error = code == "TRACKING_WRITE";
        let failed = (!tracking_error)
            .then(|| operations.get(index.saturating_sub(1)).cloned())
            .flatten();
        let not_run: Vec<_> = operations.iter().skip(index).cloned().collect();
        if let Some(uncertain) = failed
            .as_deref()
            .filter(|entry| valid_uncertain_entry(entry))
        {
            if let Err((_, state_error)) = persist_uncertain(
                &state_path,
                &p.manifest.project.name,
                &target,
                device_id.as_deref(),
                &entries,
                uncertain,
            ) {
                return Err((
                    "PROJECT_DEPLOY_FAILED",
                    format!(
                        "{message}; tracking update failed: {state_error}; deployment report: {}",
                        json!({"succeeded":succeeded,"failed":failed,"not_run":not_run,"uncertain":[uncertain],"tracking_error":true,"transactional":false,"cause":code})
                    ),
                ));
            }
        }
        // Only confirmed successes are tracked. A failed mutation is marked uncertain because
        // the device may have applied it before the response was lost.
        return Err((
            "PROJECT_DEPLOY_FAILED",
            format!(
                "{message}; deployment report: {}",
                json!({"succeeded":succeeded,"failed":failed,"not_run":not_run,"uncertain":if tracking_error {Vec::<String>::new()} else {failed.clone().into_iter().collect()},"tracking_error":tracking_error,"transactional":false,"failed_operation":failed,"cause":code})
            ),
        ));
    }
    Ok(
        json!({"project":p.manifest.project.name,"target":target,"target_origin":selected.origin,"succeeded":succeeded,"failed":null,"not_run":[],"uncertain":[],"additive":true,"transactional":false}),
    )
}

fn persist_success(
    path: &Path,
    project: &str,
    target: &str,
    device_id: Option<&str>,
    entries: &mut Vec<String>,
    operation: &str,
) -> Result<()> {
    if !valid_tracked_entry(operation) || entries.contains(&operation.to_owned()) {
        return Ok(());
    }
    entries.push(operation.to_owned());
    let mut state = Tracking {
        version: 2,
        project: project.to_owned(),
        target: target.to_owned(),
        device_id: device_id.map(str::to_owned),
        entries: entries.clone(),
        uncertain: Vec::new(),
    };
    let existing = fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Tracking>(&b).ok());
    state.uncertain = existing.map(|s| s.uncertain).unwrap_or_default();
    state.uncertain.retain(|entry| entry != operation);
    if let Err(error) = save_tracking(path, &state) {
        entries.pop();
        return Err(error);
    }
    Ok(())
}

fn persist_uncertain(
    path: &Path,
    project: &str,
    target: &str,
    device_id: Option<&str>,
    entries: &[String],
    uncertain: &str,
) -> Result<()> {
    let previous = fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Tracking>(&b).ok());
    let mut uncertain_entries = previous
        .as_ref()
        .map(|s| s.uncertain.clone())
        .unwrap_or_default();
    if !uncertain_entries.iter().any(|item| item == uncertain) {
        uncertain_entries.push(uncertain.to_owned());
    }
    save_tracking(
        path,
        &Tracking {
            version: 2,
            project: project.to_owned(),
            target: target.to_owned(),
            device_id: device_id.map(str::to_owned),
            entries: entries.to_vec(),
            uncertain: uncertain_entries,
        },
    )
}

fn prune(p: Loaded, manifest_path: &Path, cli: &crate::Cli, dry_run: bool) -> Result<Value> {
    let explicit = std::env::args().any(|a| a == "--target" || a.starts_with("--target="));
    let selected = crate::profiles::resolve_project(
        if explicit {
            cli.target.as_deref()
        } else {
            None
        },
        cli.profile.as_deref(),
        p.manifest.target.profile.as_deref(),
        cli.username.as_deref(),
        cli.password.as_deref(),
    )?;
    let target = reqwest::Url::parse(selected.target.as_deref().unwrap_or_default())
        .map_err(|_| ("ARGUMENT", "invalid selected target".into()))?
        .to_string()
        .trim_end_matches('/')
        .to_owned();
    let path = tracking_path(manifest_path, &p.manifest.project.name, &target);
    let mut state = read_tracking(&path, &p.manifest.project.name, &target, None)?;
    let api = crate::ApiClient::new(
        cli,
        selected.target.as_deref(),
        selected.username,
        selected.password,
    )?;
    let actual_device_id = device_id(&api)?;
    if state.device_id.as_deref() != actual_device_id.as_deref() {
        return Err((
            "TRACKING_INVALID",
            "device identity changed at this endpoint; refusing deletion".into(),
        ));
    }
    if !state.uncertain.is_empty() {
        return Err(("TRACKING_UNCERTAIN", "tracking contains operations with unknown remote effects; inspect and reconcile before pruning".into()));
    }
    let current = p
        .manifest
        .modules
        .iter()
        .map(|x| format!("module:{}", x.name))
        .chain(
            p.manifest
                .resources
                .iter()
                .map(|x| format!("resource:{}", x.path)),
        )
        .chain(
            p.manifest
                .scripts
                .iter()
                .map(|x| format!("script:{}", x.name)),
        )
        .chain(
            p.manifest
                .config
                .iter()
                .map(|x| format!("config:{}", x.script)),
        )
        .collect::<Vec<_>>();
    let obsolete = state
        .entries
        .iter()
        .filter(|x| !current.contains(x))
        .cloned()
        .collect::<Vec<_>>();
    if dry_run {
        return Ok(
            json!({"project":state.project,"target":target,"dry_run":true,"planned":obsolete,"deleted":[],"failed":null,"not_run":[]}),
        );
    }
    let mut deleted = Vec::new();
    let mut not_run = Vec::new();
    for (i, entry) in obsolete.iter().enumerate() {
        let result = if let Some(name) = entry
            .strip_prefix("script:")
            .or_else(|| entry.strip_prefix("module:"))
        {
            api.mutate(
                reqwest::Method::DELETE,
                &format!("/api/v1/apps/{name}"),
                &Value::Null,
            )
            .map(|_| ())
        } else if let Some(path) = entry.strip_prefix("resource:") {
            api.resource_request(
                reqwest::Method::DELETE,
                &format!("/api/v1/files?path={}", encode(path)),
                None,
                None,
            )
            .map(|_| ())
        } else {
            continue;
        };
        match result {
            Ok(()) => {
                deleted.push(entry.clone());
                state.entries.retain(|tracked| tracked != entry);
                if let Err((_, message)) = save_tracking(&path, &state) {
                    return Err(("PROJECT_DEPLOY_FAILED", format!("deletion succeeded but tracking update failed: {message}; deployment report: {}", json!({"succeeded":deleted,"failed":null,"not_run":obsolete.iter().skip(i + 1).collect::<Vec<_>>(),"uncertain":[],"tracking_error":true,"transactional":false}))));
                }
            }
            Err((code, _)) => {
                not_run.extend(obsolete.iter().skip(i + 1).cloned());
                if !state.uncertain.iter().any(|item| item == entry) {
                    state.uncertain.push(entry.clone());
                }
                if let Err((_, message)) = save_tracking(&path, &state) {
                    return Err(("PROJECT_DEPLOY_FAILED", format!("prune failed and tracking update failed: {message}; deployment report: {}", json!({"succeeded":deleted,"failed":entry,"not_run":not_run,"uncertain":[entry],"tracking_error":true,"cause":code,"transactional":false}))));
                }
                return Err((
                    "PROJECT_DEPLOY_FAILED",
                    format!(
                        "prune failed; deployment report: {}",
                        json!({"succeeded":deleted,"failed":entry,"not_run":not_run,"uncertain":[entry],"cause":code,"transactional":false})
                    ),
                ));
            }
        }
    }
    Ok(
        json!({"project":state.project,"target":target,"dry_run":false,"planned":obsolete,"deleted":deleted,"failed":null,"not_run":[]}),
    )
}

fn reconcile(
    p: Loaded,
    manifest_path: &Path,
    cli: &crate::Cli,
    forget_uncertain: bool,
) -> Result<Value> {
    if !forget_uncertain {
        return Err((
            "ARGUMENT",
            "reconcile requires explicit --forget-uncertain; remote outcomes are not verified"
                .into(),
        ));
    }
    let explicit = std::env::args().any(|a| a == "--target" || a.starts_with("--target="));
    let selected = crate::profiles::resolve_project(
        if explicit {
            cli.target.as_deref()
        } else {
            None
        },
        cli.profile.as_deref(),
        p.manifest.target.profile.as_deref(),
        cli.username.as_deref(),
        cli.password.as_deref(),
    )?;
    let target = reqwest::Url::parse(selected.target.as_deref().unwrap_or_default())
        .map_err(|_| ("ARGUMENT", "invalid selected target".into()))?
        .to_string()
        .trim_end_matches('/')
        .to_owned();
    let path = tracking_path(manifest_path, &p.manifest.project.name, &target);
    let mut state = read_tracking(&path, &p.manifest.project.name, &target, None)?;
    let api = crate::ApiClient::new(
        cli,
        selected.target.as_deref(),
        selected.username,
        selected.password,
    )?;
    let actual_device_id = device_id(&api)?;
    if state.device_id.as_deref() != actual_device_id.as_deref() {
        return Err((
            "TRACKING_INVALID",
            "device identity changed at this endpoint; refusing reconciliation".into(),
        ));
    }

    let forgotten = std::mem::take(&mut state.uncertain);
    let released = state
        .entries
        .iter()
        .filter(|entry| forgotten.contains(entry))
        .cloned()
        .collect::<Vec<_>>();
    state.entries.retain(|entry| !forgotten.contains(entry));
    save_tracking(&path, &state)?;
    Ok(json!({
        "project": state.project,
        "target": target,
        "forgotten_uncertain": forgotten,
        "released_tracked_entries": released,
        "remaining_tracked_entries": state.entries,
        "remote_mutations": false,
        "remote_effect": "unknown; inspect the device manually before managing forgotten items"
    }))
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~/".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

pub fn describe(topic: &str) -> Value {
    let command = topic.strip_prefix("project ").unwrap_or("project");
    if command == "project reconcile" || command == "reconcile" {
        return json!({
            "command": "project reconcile",
            "parameters": {"--manifest": "manifest path (default awtrix.toml)", "--forget-uncertain": "required explicit local release of uncertain ownership; no device mutation or outcome claim"},
            "output_fields": ["project", "target", "forgotten_uncertain", "released_tracked_entries", "remaining_tracked_entries", "remote_mutations", "remote_effect"],
            "examples": ["awtrix --json project reconcile --manifest awtrix.toml --forget-uncertain"],
            "safety": "validates project, endpoint, and available device identifier; removes uncertain names from tracking ownership and clears uncertainty only. Forgotten names are never automatically pruned and require manual device inspection. Known unaffected tracked entries remain tracked. No remote mutations are sent."
        });
    }
    json!({"command":command,"parameters":{"PATH":"project directory","MANIFEST":"manifest path (default awtrix.toml)","--force":"explicit unprotected overwrite for declared scripts","project prune --dry-run":"preview explicit tracked deletions"},"inputs":["awtrix.toml identity, target profile, scripts/modules/resources/config paths relative to manifest","private project-local identity-keyed .awtrix-tracking-*.json state"],"outputs":["validation counts","ordered deploy report with succeeded/failed/not_run/uncertain/tracking_error","prune report with planned/deleted/failed/not_run/uncertain/tracking_error"],"examples":["awtrix project init ./demo","awtrix project validate --manifest demo/awtrix.toml","awtrix --json project deploy --manifest demo/awtrix.toml","awtrix --json project prune --manifest demo/awtrix.toml --dry-run","awtrix --json project prune --manifest demo/awtrix.toml"],"prerequisites":["personal profile configuration for named targets; no credentials in project manifest","scriptUpdates capability for protected scripts","prune requires valid tracking state matching project, normalized endpoint, and available device ID; uncertain entries must be manually reconciled"],"tracking_contract":{"version":2,"identity":"project name plus normalized effective endpoint, and UID/deviceId/serial from /api/v1/device when supplied; credentials are never stored","entries":"successfully deployed scripts, modules and resources only; config patches are not deletable resources","uncertain":"failed operations are durably recorded separately; prune is blocked until manual reconciliation","safety":"missing, corrupt, foreign-project, foreign-endpoint, or changed-device state fails closed; deploy remains additive","state_file":".awtrix-tracking-<identity-key>.json beside the manifest, mode 0600 on Unix, atomically persisted after each successful operation; separate from credential configuration"},"schema":{"project":{"name":"string","version":"string"},"target":{"profile":"optional personal profile name"},"scripts":[{"name":"AWTRIX script name","file":"relative Berry source path","create":"boolean","expected_source_file":"relative original source used for conflict protection"}],"modules":[{"name":"module name","file":"relative Berry source with # @module"}],"resources":[{"path":"absolute device file path","file":"relative local binary path"}],"config":[{"script":"declared script name","file":"relative JSON object path"}]}})
}
