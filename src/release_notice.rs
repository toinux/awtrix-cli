//! Best-effort stable release notices. Failures are deliberately silent: this
//! check must never change an ordinary command's result or structured output.
use serde::Deserialize;
use std::{
    fs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DAY: u64 = 24 * 60 * 60;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    #[serde(default)]
    draft: bool,
}

/// Check GitHub at most once per day. Fixture destination overrides are honored
/// only in debug builds; published release binaries always use fixed defaults.
pub(crate) fn notice() -> Option<String> {
    if std::env::var_os("AWTRIX_NO_UPDATE_CHECK").is_some() {
        return None;
    }
    // Only debug/test executables may redirect these destinations. Release
    // artifacts (built with debug assertions disabled) cannot be redirected.
    let cache_override = cfg!(debug_assertions)
        .then(|| std::env::var_os("AWTRIX_RELEASE_CACHE"))
        .flatten();
    let cache = cache_override.map(Into::into).or_else(default_cache_path)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    if fs::read_to_string(&cache)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|last| now.saturating_sub(last) < DAY)
    {
        return None;
    }
    // Write before attempting network so an outage is also throttled.
    if let Some(parent) = cache.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&cache, now.to_string()).is_err() {
        return None;
    }
    let endpoint = (cfg!(debug_assertions).then(|| std::env::var("AWTRIX_RELEASE_API_URL").ok()))
        .flatten()
        .unwrap_or_else(|| "https://api.github.com/repos/toinux/awtrix-cli/releases".into());
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .user_agent(concat!("awtrix-cli/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let releases: Vec<Release> = client
        .get(endpoint)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .ok()?;
    let local = env!("CARGO_PKG_VERSION");
    releases.into_iter().filter(|r| !r.prerelease && !r.draft).find_map(|r| {
        let remote = r.tag_name.strip_prefix('v').unwrap_or(&r.tag_name);
        (version(remote) > version(local)).then(|| format!("A newer stable awtrix-cli release is available: {remote} (installed: {local}); see https://github.com/toinux/awtrix-cli/releases to update."))
    })
}

fn default_cache_path() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let root = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .map(Into::into);
    #[cfg(not(windows))]
    let root = std::env::var_os("XDG_CACHE_HOME")
        .map(Into::into)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(Into::into)
                .map(|home: std::path::PathBuf| home.join(".cache"))
        });
    root.map(|root: std::path::PathBuf| root.join("awtrix-cli/release-check"))
}

fn version(v: &str) -> (u64, u64, u64) {
    let mut parts = v
        .split('.')
        .map(|part| part.split('-').next().unwrap_or("").parse().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}
