//! TOML project loading, complete local preflight, and additive device deployment.
use clap::Subcommand;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
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
        Command::Deploy { manifest, force } => deploy(load(manifest)?, cli, *force),
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

fn deploy(p: Loaded, cli: &crate::Cli, force: bool) -> Result<Value> {
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
            succeeded.push(operations[index - 1].clone());
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
            succeeded.push(operations[index - 1].clone());
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
            succeeded.push(operations[index - 1].clone());
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
        let failed = operations.get(index.saturating_sub(1)).cloned();
        let not_run: Vec<_> = operations.iter().skip(index).cloned().collect();
        return Err((
            "PROJECT_DEPLOY_FAILED",
            format!(
                "{message}; deployment report: {}",
                json!({"succeeded":succeeded,"failed":failed,"not_run":not_run,"transactional":false,"failed_operation":failed,"cause":code})
            ),
        ));
    }
    Ok(
        json!({"project":p.manifest.project.name,"target":selected.target,"target_origin":selected.origin,"succeeded":succeeded,"failed":null,"not_run":[],"additive":true,"transactional":false}),
    )
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
    json!({"command":command,"parameters":{"PATH":"project directory","MANIFEST":"manifest path (default awtrix.toml)","--force":"explicit unprotected overwrite for declared scripts"},"inputs":["awtrix.toml identity, target profile, scripts/modules/resources/config paths relative to manifest"],"outputs":["validation counts","ordered deploy report with succeeded/failed/not_run"],"examples":["awtrix project init ./demo","awtrix project validate --manifest demo/awtrix.toml","awtrix --json project deploy --manifest demo/awtrix.toml"],"prerequisites":["personal profile configuration for named targets; no credentials in project manifest","scriptUpdates capability for protected scripts"],"schema":{"project":{"name":"string","version":"string"},"target":{"profile":"optional personal profile name"},"scripts":[{"name":"AWTRIX script name","file":"relative Berry source path","create":"boolean","expected_source_file":"relative original source used for conflict protection"}],"modules":[{"name":"module name","file":"relative Berry source with # @module"}],"resources":[{"path":"absolute device file path","file":"relative local binary path"}],"config":[{"script":"declared script name","file":"relative JSON object path"}]}})
}
