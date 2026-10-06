//! Raw Berry source operations. The conditional route is atomic; raw PUT is explicitly force-only.
use clap::Subcommand;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

#[derive(Subcommand)]
pub enum Command {
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
    },
}

fn valid_name(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn run(command: &Command, cli: &crate::Cli) -> crate::CliResult<Value> {
    let api = crate::ApiClient::new(cli)?;
    match command {
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
                operational_result(result, false, None)
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
                        let verification = verify_start(&api, name);
                        operational_result(result, true, verification)
                    }
                }
            }
        }
    }
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
