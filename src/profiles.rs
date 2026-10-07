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

#[derive(Clone, Copy)]
#[allow(dead_code)] // Both variants are exercised by path-resolution unit tests on every host.
enum Platform {
    Windows,
    Unix,
}

fn config_path(
    override_path: Option<PathBuf>,
    appdata: Option<PathBuf>,
    home: Option<PathBuf>,
    platform: Platform,
) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path);
    }
    match platform {
        Platform::Windows => appdata
            .map(|root| root.join("awtrix-cli").join("config.json"))
            .or_else(|| {
                home.map(|root| {
                    root.join("AppData")
                        .join("Roaming")
                        .join("awtrix-cli")
                        .join("config.json")
                })
            })
            .ok_or((
                "CONFIG_INVALID",
                "set AWTRIX_CONFIG, APPDATA, or USERPROFILE to locate the personal configuration"
                    .into(),
            )),
        Platform::Unix => home
            .map(|root| root.join(".config").join("awtrix-cli").join("config.json"))
            .ok_or((
                "CONFIG_INVALID",
                "set AWTRIX_CONFIG or HOME to locate the personal configuration".into(),
            )),
    }
}

pub(crate) fn path() -> Result<PathBuf> {
    let override_path = std::env::var_os("AWTRIX_CONFIG").map(PathBuf::from);
    #[cfg(windows)]
    {
        config_path(
            override_path,
            std::env::var_os("APPDATA").map(PathBuf::from),
            std::env::var_os("USERPROFILE").map(PathBuf::from),
            Platform::Windows,
        )
    }
    #[cfg(not(windows))]
    {
        config_path(
            override_path,
            None,
            std::env::var_os("HOME").map(PathBuf::from),
            Platform::Unix,
        )
    }
}
fn load() -> Result<Config> {
    match fs::read(path()?) {
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
    let path = path()?;
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

pub fn describe(action: Option<&str>) -> Result<Value> {
    let common = json!({"parameters":{"--json":"compact JSON output","--fields":"comma-separated top-level result fields"},"inputs":["personal configuration at AWTRIX_CONFIG or the platform default"],"prerequisites":["profile names are local identifiers; target URLs are HTTP(S) without embedded credentials"]});
    let (command, parameters, fields, example) = match action.unwrap_or("profile") {
        "profile" => (
            "profile",
            json!({"<action>":"add, update, list, show, set-default, or delete"}),
            json!(["action-specific result fields"]),
            "awtrix-cli profile list",
        ),
        "add" => (
            "profile add",
            json!({"NAME":"profile name","--target":"HTTP base URL","--username":"optional Basic username","--password":"optional Basic password"}),
            json!(["name", "target", "saved"]),
            "awtrix-cli profile add desk --target http://awtrix.local",
        ),
        "update" => (
            "profile update",
            json!({"NAME":"existing profile name","--target":"HTTP base URL","--username":"optional Basic username","--password":"optional Basic password"}),
            json!(["name", "target", "updated"]),
            "awtrix-cli profile update desk --target http://awtrix.local",
        ),
        "list" => (
            "profile list",
            json!({}),
            json!(["profiles", "default"]),
            "awtrix-cli --json profile list",
        ),
        "show" => (
            "profile show",
            json!({"NAME":"profile name"}),
            json!([
                "name",
                "target",
                "username_configured",
                "password_configured"
            ]),
            "awtrix-cli profile show desk",
        ),
        "set-default" => (
            "profile set-default",
            json!({"NAME":"profile name"}),
            json!(["default"]),
            "awtrix-cli profile set-default desk",
        ),
        "delete" => (
            "profile delete",
            json!({"NAME":"profile name"}),
            json!(["deleted"]),
            "awtrix-cli profile delete desk",
        ),
        other => {
            return Err((
                "ARGUMENT",
                format!("unknown profile description topic '{other}'"),
            ))
        }
    };
    let mut description = common;
    description["command"] = json!(command);
    description["parameters"]
        .as_object_mut()
        .unwrap()
        .extend(parameters.as_object().unwrap().clone());
    description["output_fields"] = fields;
    description["outputs"] =
        json!(["compact JSON object with the documented output_fields; credentials are omitted"]);
    description["examples"] = json!([example]);
    Ok(description)
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

/// Resolve a project target between environment and the global default profile.
pub fn resolve_project(
    explicit: Option<&str>,
    profile: Option<&str>,
    project: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Resolved> {
    if explicit.is_some() || std::env::var("AWTRIX_URL").is_ok() || profile.is_some() {
        return resolve(explicit, profile, username, password);
    }
    if let Some(name) = project {
        let c = load()?;
        let entry = c.profiles.get(name).ok_or((
            "PROFILE_NOT_FOUND",
            format!("project profile '{name}' does not exist"),
        ))?;
        return Ok(Resolved {
            target: Some(entry.target.clone()),
            origin: "project-profile",
            username: username
                .map(str::to_owned)
                .or_else(|| std::env::var("AWTRIX_USERNAME").ok())
                .or_else(|| entry.username.clone()),
            password: password
                .map(str::to_owned)
                .or_else(|| std::env::var("AWTRIX_PASSWORD").ok())
                .or_else(|| entry.password.clone()),
        });
    }
    resolve(explicit, profile, username, password)
}

#[cfg(test)]
mod tests {
    use super::{config_path, Platform};
    use std::path::PathBuf;

    #[test]
    fn windows_personal_path_uses_appdata_then_userprofile() {
        assert_eq!(
            config_path(
                None,
                Some(PathBuf::from("C:/Users/A/AppData/Roaming")),
                None,
                Platform::Windows
            )
            .unwrap(),
            PathBuf::from("C:/Users/A/AppData/Roaming/awtrix-cli/config.json")
        );
        assert_eq!(
            config_path(
                None,
                None,
                Some(PathBuf::from("C:/Users/A")),
                Platform::Windows
            )
            .unwrap(),
            PathBuf::from("C:/Users/A/AppData/Roaming/awtrix-cli/config.json")
        );
    }

    #[test]
    fn personal_config_override_and_unix_home_are_respected_without_temp_fallback() {
        assert_eq!(
            config_path(
                Some(PathBuf::from("custom.json")),
                None,
                None,
                Platform::Unix
            )
            .unwrap(),
            PathBuf::from("custom.json")
        );
        assert_eq!(
            config_path(None, None, Some(PathBuf::from("/home/a")), Platform::Unix).unwrap(),
            PathBuf::from("/home/a/.config/awtrix-cli/config.json")
        );
        assert!(config_path(None, None, None, Platform::Unix).is_err());
    }

    #[test]
    fn explicit_config_file_is_authoritative_on_all_platforms() {
        let selected = PathBuf::from("custom/location/profiles.json");
        for platform in [Platform::Windows, Platform::Unix] {
            assert_eq!(
                config_path(
                    Some(selected.clone()),
                    Some(PathBuf::from("C:/Users/A/AppData/Roaming")),
                    Some(PathBuf::from("/home/a")),
                    platform,
                )
                .unwrap(),
                selected
            );
        }
    }
}
