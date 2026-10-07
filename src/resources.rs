//! Files use the official generic file routes; Berry modules use script routes.
use clap::Subcommand;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Subcommand)]
pub enum Command {
    Files {
        #[command(subcommand)]
        action: FileCommand,
    },
    Modules {
        #[command(subcommand)]
        action: ModuleCommand,
    },
}
#[derive(Subcommand)]
pub enum FileCommand {
    List {
        #[arg(long, default_value = "/ICONS")]
        dir: String,
    },
    Upload {
        #[arg(long, default_value = "/ICONS")]
        dir: String,
        file: PathBuf,
    },
    Download {
        path: String,
        #[arg(long)]
        output: PathBuf,
    },
    Delete {
        path: String,
    },
}
#[derive(Subcommand)]
pub enum ModuleCommand {
    List,
    Get {
        name: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Deploy {
        name: String,
        #[arg(long, conflicts_with = "file")]
        source: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
    },
    Delete {
        name: String,
    },
}

fn valid_name(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
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
fn validate_module(name: &str) -> crate::CliResult<()> {
    if valid_name(name) {
        Ok(())
    } else {
        Err((
            "ARGUMENT",
            "module name must match [A-Za-z0-9_-]{1,32}".into(),
        ))
    }
}
pub(crate) fn declares_module(source: &str) -> bool {
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let Some(comment) = trimmed.strip_prefix('#') else {
            return false;
        };
        let comment = comment.trim_start();
        if let Some(rest) = comment.strip_prefix("@module") {
            return rest.is_empty() || rest.starts_with(char::is_whitespace);
        }
    }
    false
}
pub(crate) fn validate_icon(dir: &str, path: &Path, bytes: &[u8]) -> crate::CliResult<()> {
    if dir == "/ICONS" {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let gif = bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a");
        let jpeg = bytes.starts_with(&[0xff, 0xd8, 0xff]);
        if !((ext == "gif" && gif) || (ext == "jpg" && jpeg)) {
            return Err((
                "INVALID_RESOURCE",
                "/ICONS requires lowercase .gif or .jpg with matching GIF/JPEG file magic".into(),
            ));
        }
    }
    Ok(())
}
pub fn run(command: &Command, api: &crate::ApiClient) -> crate::CliResult<Value> {
    match command {
        Command::Files { action } => match action {
            FileCommand::List { dir } => api.get(&format!("/api/v1/files?dir={}", encode(dir))),
            FileCommand::Delete { path } => api.resource_request(
                reqwest::Method::DELETE,
                &format!("/api/v1/files?path={}", encode(path)),
                None,
                None,
            ),
            FileCommand::Download { .. } => Err((
                "UNSUPPORTED",
                "the official AWTRIX NG API defines no generic file-download route".into(),
            )),
            FileCommand::Upload { dir, file } => {
                let bytes = fs::read(file)
                    .map_err(|_| ("FILE_READ", "could not read resource file".into()))?;
                validate_icon(dir, file, &bytes)?;
                let name = file
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or(("ARGUMENT", "resource filename must be UTF-8".into()))?;
                let form = reqwest::blocking::multipart::Form::new().part(
                    "file",
                    reqwest::blocking::multipart::Part::bytes(bytes).file_name(name.to_owned()),
                );
                api.multipart_post(&format!("/api/v1/files?dir={}", encode(dir)), form)
            }
        },
        Command::Modules { action } => match action {
            ModuleCommand::List => {
                let value = api.get("/api/v1/apps")?;
                let apps = value
                    .as_array()
                    .ok_or(("INVALID_RESPONSE", "app inventory is not an array".into()))?;
                Ok(
                    json!({"modules":apps.iter().filter(|app|app.get("origin").and_then(Value::as_str)==Some("module")).collect::<Vec<_>>()}),
                )
            }
            ModuleCommand::Get { name, output } => {
                validate_module(name)?;
                let source = api.raw_get(&format!("/api/v1/apps/script/{name}"))?;
                if let Some(path) = output {
                    fs::write(path, source)
                        .map_err(|_| ("FILE_WRITE", "could not write module source".into()))?;
                    Ok(json!({"path":path}))
                } else {
                    Ok(json!({"source":source}))
                }
            }
            ModuleCommand::Deploy { name, source, file } => {
                validate_module(name)?;
                let source = match (source, file) {
                    (Some(s), None) => s.clone(),
                    (None, Some(path)) => fs::read_to_string(path).map_err(|_| {
                        ("FILE_READ", "could not read module source as UTF-8".into())
                    })?,
                    _ => {
                        return Err((
                            "ARGUMENT",
                            "provide exactly one of --source or --file".into(),
                        ))
                    }
                };
                if source.is_empty() || !declares_module(&source) {
                    return Err((
                        "ARGUMENT",
                        "non-empty module source must declare # @module in its header".into(),
                    ));
                }
                let result = api.raw_put(&format!("/api/v1/apps/script/{name}"), &source)?;
                if result.get("error").is_some_and(|e| !e.is_null()) {
                    return Err(("BERRY_ERROR", "device rejected Berry module source".into()));
                }
                Ok(
                    json!({"name":name,"source_saved":true,"rotation_app":false,"references_rewritten":false,"result":result}),
                )
            }
            ModuleCommand::Delete { name } => {
                validate_module(name)?;
                api.resource_request(
                    reqwest::Method::DELETE,
                    &format!("/api/v1/apps/{name}"),
                    None,
                    None,
                )
            }
        },
    }
}

pub fn describe(topic: Option<&str>) -> Value {
    let op = topic.unwrap_or("resources");
    let (parameters, outputs, example) = match op {
        "resources files list" => (
            json!({"--dir":"asset folder, default /ICONS"}),
            json!(["files", "usedBytes", "totalBytes"]),
            "awtrix-cli resources files list --dir /ICONS",
        ),
        "resources files upload" => (
            json!({"FILE":"local file path","--dir":"destination directory (default /ICONS)"}),
            json!(["ok"]),
            "awtrix-cli resources files upload icon.gif --dir /ICONS",
        ),
        "resources files delete" => (
            json!({"PATH":"full device path"}),
            json!(["ok"]),
            "awtrix-cli resources files delete /ICONS/icon.gif",
        ),
        "resources files download" => (
            json!({"PATH":"full device path","--output":"local destination"}),
            json!(["error"]),
            "awtrix-cli resources files download /ICONS/icon.gif --output icon.gif",
        ),
        "resources modules list" => (
            json!({}),
            json!(["modules"]),
            "awtrix-cli resources modules list",
        ),
        "resources modules get" => (
            json!({"NAME":"module identifier","--output":"optional local destination"}),
            json!(["source", "path"]),
            "awtrix-cli resources modules get helpers --output helpers.be",
        ),
        "resources modules deploy" => (
            json!({"NAME":"module identifier","--source":"Berry source","--file":"UTF-8 source file"}),
            json!([
                "name",
                "source_saved",
                "rotation_app",
                "references_rewritten",
                "result"
            ]),
            "awtrix-cli resources modules deploy helpers --file helpers.be",
        ),
        "resources modules delete" => (
            json!({"NAME":"module identifier"}),
            json!(["ok"]),
            "awtrix-cli resources modules delete helpers",
        ),
        _ => (
            json!({"files":"list/upload/delete at /api/v1/files; no download route","modules":"list/get/deploy/delete via documented app/script routes"}),
            json!(["operation-specific result"]),
            "awtrix-cli resources files list",
        ),
    };
    let output_fields: Vec<String> = outputs
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let mut parameters = parameters;
    parameters["--target"] =
        json!("HTTP base URL; required for execution, optional for connected describe");
    parameters["--profile"] = json!("named device profile (or AWTRIX_PROFILE)");
    parameters["--username"] = json!("HTTP Basic username");
    parameters["--password"] = json!("HTTP Basic password");
    parameters["--timeout"] = json!("bounded request timeout in milliseconds (default 3000)");
    parameters["--json"] = json!("emit compact machine-readable JSON");
    parameters["--fields"] = json!("comma-separated top-level result fields");
    json!({"command":op,"parameters":parameters,"outputs":outputs,"output_fields":output_fields,"examples":[example],"prerequisites":["AWTRIX NG HTTP API","module deploy source must carry # @module header"],"limitations":["No generic file-download API route is documented. Berry module source is available via GET /api/v1/apps/script/{name}.","Inline pushed-app icon values are icon IDs (up to 64 characters) or the documented data URLs data:image/gif.Base64,... / data:image/jpeg.Base64,...; this CLI file uploader does not reinterpret those JSON values.","Module replacement or deletion does not rewrite application references. Device request/storage capacity is enforced by the official route (413/507); no undocumented variant-independent cap is imposed."]})
}
