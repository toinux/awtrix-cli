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

/// Check GitHub at most once per day. `AWTRIX_NO_UPDATE_CHECK` disables checks;
/// `AWTRIX_RELEASE_API_URL` and `AWTRIX_RELEASE_CACHE` are fixture seams.
pub(crate) fn notice() -> Option<String> {
    if std::env::var_os("AWTRIX_NO_UPDATE_CHECK").is_some() {
        return None;
    }
    let cache = std::env::var_os("AWTRIX_RELEASE_CACHE")
        .map(Into::into)
        .unwrap_or_else(|| {
            std::env::var_os("XDG_CACHE_HOME")
                .map(Into::into)
                .unwrap_or_else(std::env::temp_dir)
                .join("awtrix-cli/release-check")
        });
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
    let _ = fs::write(&cache, now.to_string());
    let endpoint = std::env::var("AWTRIX_RELEASE_API_URL")
        .unwrap_or_else(|_| "https://api.github.com/repos/toinux/awtrix-cli/releases".into());
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
