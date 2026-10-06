//! Personal device profiles. Configuration is private user data, never project data.
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::PathBuf};

type Result<T> = std::result::Result<T, (&'static str, String)>;

#[derive(Subcommand)]
pub enum ProfileCommand {
    Add {
        name: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        password: Option<String>,
    },
    Update {
        name: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        password: Option<String>,
    },
    List,
    Show {
        name: String,
    },
    SetDefault {
        name: String,
    },
    Delete {
        name: String,
    },
}

#[derive(Default, Serialize, Deserialize)]
struct Config {
    profiles: BTreeMap<String, Entry>,
    default: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
}
pub struct Resolved {
    pub target: Option<String>,
    pub origin: &'static str,
    pub username: Option<String>,
    pub password: Option<String>,
}

fn path() -> PathBuf {
    if let Some(path) = std::env::var_os("AWTRIX_CONFIG") {
        return PathBuf::from(path);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join(".config/awtrix/config.json")
}
fn load() -> Result<Config> {
    match fs::read(path()) {
        Ok(data) => {
            let config: Config = serde_json::from_slice(&data)
                .map_err(|_| ("CONFIG_INVALID", "profile configuration is invalid".into()))?;
            if config
                .profiles
                .values()
                .any(|entry| !valid_url(&entry.target))
                || config
                    .default
                    .as_ref()
                    .is_some_and(|name| !config.profiles.contains_key(name))
            {
                return Err((
                    "CONFIG_INVALID",
                    "profile configuration contains an invalid target or default".into(),
                ));
            }
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(_) => Err((
            "CONFIG_INVALID",
            "profile configuration cannot be read".into(),
        )),
    }
}
fn save(config: &Config) -> Result<()> {
    let path = path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| {
            (
                "CONFIG_INVALID",
                "profile configuration cannot be written".into(),
            )
        })?;
    }
    let bytes = serde_json::to_vec(config).map_err(|_| {
        (
            "CONFIG_INVALID",
            "profile configuration cannot be serialized".into(),
        )
    })?;
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes).map_err(|_| {
        (
            "CONFIG_INVALID",
            "profile configuration cannot be written".into(),
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp, fs::Permissions::from_mode(0o600))
            .map_err(|_| ("CONFIG_INVALID", "profile permissions cannot be set".into()))?;
    }
    fs::rename(temp, path).map_err(|_| {
        (
            "CONFIG_INVALID",
            "profile configuration cannot be replaced".into(),
        )
    })
}
fn valid_url(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|u| {
        matches!(u.scheme(), "http" | "https")
            && u.host_str().is_some()
            && u.username().is_empty()
            && u.password().is_none()
    })
}
pub fn run(action: &ProfileCommand) -> Result<Value> {
    let mut c = load()?;
    match action {
        ProfileCommand::Add {
            name,
            target,
            username,
            password,
        } => {
            if !valid_url(target) {
                return Err(("ARGUMENT", "target must be an absolute HTTP URL".into()));
            }
            c.profiles.insert(
                name.clone(),
                Entry {
                    target: target.clone(),
                    username: username.clone(),
                    password: password.clone(),
                },
            );
            save(&c)?;
            Ok(json!({"name":name,"target":target,"saved":true}))
        }
        ProfileCommand::Update {
            name,
            target,
            username,
            password,
        } => {
            if !c.profiles.contains_key(name) {
                return Err((
                    "PROFILE_NOT_FOUND",
                    format!("profile '{name}' does not exist"),
                ));
            }
            if !valid_url(target) {
                return Err(("ARGUMENT", "target must be an absolute HTTP URL".into()));
            }
            c.profiles.insert(
                name.clone(),
                Entry {
                    target: target.clone(),
                    username: username.clone(),
                    password: password.clone(),
                },
            );
            save(&c)?;
            Ok(json!({"name":name,"target":target,"updated":true}))
        }
        ProfileCommand::List => Ok(
            json!({"profiles":c.profiles.iter().map(|(n,e)|json!({"name":n,"target":e.target})).collect::<Vec<_>>(),"default":c.default}),
        ),
        ProfileCommand::Show { name } => {
            let e = c.profiles.get(name).ok_or((
                "PROFILE_NOT_FOUND",
                format!("profile '{name}' does not exist"),
            ))?;
            Ok(
                json!({"name":name,"target":e.target,"username_configured":e.username.is_some(),"password_configured":e.password.is_some()}),
            )
        }
        ProfileCommand::SetDefault { name } => {
            if !c.profiles.contains_key(name) {
                return Err((
                    "PROFILE_NOT_FOUND",
                    format!("profile '{name}' does not exist"),
                ));
            }
            c.default = Some(name.clone());
            save(&c)?;
            Ok(json!({"default":name}))
        }
        ProfileCommand::Delete { name } => {
            if c.profiles.remove(name).is_none() {
                return Err((
                    "PROFILE_NOT_FOUND",
                    format!("profile '{name}' does not exist"),
                ));
            }
            if c.default.as_deref() == Some(name) {
                c.default = None;
            }
            save(&c)?;
            Ok(json!({"deleted":name}))
        }
    }
}
pub fn resolve(
    explicit: Option<&str>,
    profile: Option<&str>,
    username_override: Option<&str>,
    password_override: Option<&str>,
) -> Result<Resolved> {
    if let Some(target) = explicit {
        return Ok(Resolved {
            target: Some(target.to_string()),
            origin: "command-line",
            username: username_override
                .map(str::to_owned)
                .or_else(|| std::env::var("AWTRIX_USERNAME").ok()),
            password: password_override
                .map(str::to_owned)
                .or_else(|| std::env::var("AWTRIX_PASSWORD").ok()),
        });
    }
    if let Ok(target) = std::env::var("AWTRIX_URL") {
        return Ok(Resolved {
            target: Some(target),
            origin: "environment",
            username: username_override
                .map(str::to_owned)
                .or_else(|| std::env::var("AWTRIX_USERNAME").ok()),
            password: password_override
                .map(str::to_owned)
                .or_else(|| std::env::var("AWTRIX_PASSWORD").ok()),
        });
    }
    // Project profile reference is reserved here; ticket 13 will supply it between environment and default.
    let c = load()?;
    let name = profile.or(c.default.as_deref());
    if let Some(name) = name {
        let entry = c.profiles.get(name).ok_or((
            "PROFILE_NOT_FOUND",
            format!("profile '{name}' does not exist"),
        ))?;
        return Ok(Resolved {
            target: Some(entry.target.clone()),
            origin: if profile.is_some() {
                "profile"
            } else {
                "default-profile"
            },
            username: username_override
                .map(str::to_owned)
                .or_else(|| std::env::var("AWTRIX_USERNAME").ok())
                .or_else(|| entry.username.clone()),
            password: password_override
                .map(str::to_owned)
                .or_else(|| std::env::var("AWTRIX_PASSWORD").ok())
                .or_else(|| entry.password.clone()),
        });
    }
    Ok(Resolved {
        target: None,
        origin: "none",
        username: username_override
            .map(str::to_owned)
            .or_else(|| std::env::var("AWTRIX_USERNAME").ok()),
        password: password_override
            .map(str::to_owned)
            .or_else(|| std::env::var("AWTRIX_PASSWORD").ok()),
    })
}
