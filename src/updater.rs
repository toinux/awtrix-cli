//! Explicit, checksum-verified self-update. Production URLs and destinations
//! are fixed; debug-only overrides exist solely for deterministic CLI fixtures.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Duration};

const RELEASES: &str = "https://api.github.com/repos/toinux/awtrix-cli/releases/latest";
const RELEASE_PAGE: &str = "https://github.com/toinux/awtrix-cli/releases/latest";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    draft: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub(crate) fn run() -> Result<String, (&'static str, String)> {
    run_for(current_exe()?)
}

fn run_for(target: PathBuf) -> Result<String, (&'static str, String)> {
    let (api, asset_base) = endpoints();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("awtrix-cli/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(network)?;
    let release: Release = client
        .get(api)
        .send()
        .map_err(network)?
        .error_for_status()
        .map_err(network)?
        .json()
        .map_err(network)?;
    if release.prerelease || release.draft {
        return Err((
            "UPDATE_RELEASE",
            format!("No stable release is available; install manually from {RELEASE_PAGE}"),
        ));
    }
    let name = asset_name().ok_or_else(|| manual("this host is not supported"))?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .ok_or_else(|| {
            manual(&format!(
                "release {} does not include {name}",
                release.tag_name
            ))
        })?;
    let checksum_asset = release
        .assets
        .iter()
        .find(|asset| asset.name == "SHA256SUMS")
        .ok_or_else(|| manual("release is missing SHA256SUMS"))?;
    let checksums = download(
        &client,
        &url(
            &asset_base,
            &checksum_asset.name,
            &checksum_asset.browser_download_url,
        ),
    )?;
    let expected = checksum_entry(&checksums, name)?;
    let bytes = download(
        &client,
        &url(&asset_base, name, &asset.browser_download_url),
    )?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != expected {
        return Err((
            "UPDATE_CHECKSUM",
            "download checksum does not match SHA256SUMS; existing executable was not changed"
                .into(),
        ));
    }
    replace(&target, &bytes)?;
    Ok(format!(
        "Updated to {}. Restart awtrix-cli to use the new version.",
        release.tag_name
    ))
}

fn endpoints() -> (String, String) {
    #[cfg(debug_assertions)]
    if let Ok(endpoint) = std::env::var("AWTRIX_UPDATE_API_URL") {
        let base =
            std::env::var("AWTRIX_UPDATE_ASSET_BASE_URL").unwrap_or_else(|_| endpoint.clone());
        return (endpoint, base);
    }
    (RELEASES.into(), RELEASE_PAGE.into())
}

fn url(_base: &str, name: &str, published: &str) -> String {
    #[cfg(debug_assertions)]
    if std::env::var_os("AWTRIX_UPDATE_API_URL").is_some() {
        return format!("{}/{}", _base.trim_end_matches('/'), name);
    }
    let parsed = reqwest::Url::parse(published).ok();
    if parsed
        .as_ref()
        .is_some_and(|url| url.scheme() == "https" && url.host_str() == Some("github.com"))
    {
        published.to_owned()
    } else {
        format!("https://github.com/toinux/awtrix-cli/releases/latest/download/{name}")
    }
}

fn download(
    client: &reqwest::blocking::Client,
    address: &str,
) -> Result<Vec<u8>, (&'static str, String)> {
    client
        .get(address)
        .send()
        .map_err(network)?
        .error_for_status()
        .map_err(network)?
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(network)
}

fn checksum_entry(checksums: &[u8], name: &str) -> Result<String, (&'static str, String)> {
    let text = std::str::from_utf8(checksums)
        .map_err(|_| ("UPDATE_CHECKSUM", "SHA256SUMS is not valid UTF-8".into()))?;
    let mut selected = None;
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let hash = fields.next().unwrap_or_default();
        let file = fields.next().unwrap_or_default().trim_start_matches('*');
        if hash.len() != 64
            || !hash.bytes().all(|b| b.is_ascii_hexdigit())
            || fields.next().is_some()
        {
            return Err((
                "UPDATE_CHECKSUM",
                "SHA256SUMS contains a malformed entry; existing executable was not changed".into(),
            ));
        }
        if file == name {
            if selected.is_some() {
                return Err((
                    "UPDATE_CHECKSUM",
                    "SHA256SUMS contains duplicate entries for the selected asset".into(),
                ));
            }
            selected = Some(hash.to_ascii_lowercase());
        }
    }
    selected.ok_or_else(|| {
        (
            "UPDATE_CHECKSUM",
            format!("SHA256SUMS has no entry for {name}"),
        )
    })
}

fn asset_name() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    {
        return Some("awtrix-cli-x86_64-unknown-linux-gnu");
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Some("awtrix-cli-aarch64-apple-darwin");
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64", target_env = "msvc"))]
    {
        return Some("awtrix-cli-x86_64-pc-windows-msvc.exe");
    }
    #[allow(unreachable_code)]
    None
}

fn current_exe() -> Result<PathBuf, (&'static str, String)> {
    #[cfg(debug_assertions)]
    {
        if let Some(path) = std::env::var_os("AWTRIX_UPDATE_EXECUTABLE") {
            return Ok(path.into());
        }
    }
    std::env::current_exe().map_err(|_| manual("the running executable path could not be resolved"))
}

fn replace(target: &std::path::Path, bytes: &[u8]) -> Result<(), (&'static str, String)> {
    let parent = target
        .parent()
        .ok_or_else(|| manual("the executable has no safe parent directory"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| manual("cannot create a temporary file beside the executable"))?;
    use std::io::Write;
    temporary
        .write_all(bytes)
        .map_err(|_| manual("could not write the validated download"))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| manual("could not flush the validated download"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o755))
            .map_err(|_| manual("could not mark the new executable as runnable"))?;
    }

    let running = std::env::current_exe()
        .ok()
        .and_then(|path| path.canonicalize().ok());
    let resolved_target = target.canonicalize().ok();
    let is_running_executable = resolved_target.is_some() && resolved_target == running;

    if is_running_executable {
        #[cfg(windows)]
        let backup = {
            let mut backup = tempfile::NamedTempFile::new_in(parent)
                .map_err(|_| manual("cannot stage a recovery copy beside the executable"))?;
            let mut current = fs::File::open(target)
                .map_err(|_| manual("cannot open the current executable for recovery"))?;
            std::io::copy(&mut current, backup.as_file_mut())
                .map_err(|_| manual("cannot preserve the current executable before replacement"))?;
            backup
                .as_file()
                .sync_all()
                .map_err(|_| manual("cannot flush the recovery copy before replacement"))?;
            backup
        };

        #[cfg(debug_assertions)]
        let result = if std::env::var_os("AWTRIX_UPDATE_REPLACE_FAIL").is_some() {
            Err(std::io::Error::other("test-injected replacement failure"))
        } else {
            self_replace::self_replace(temporary.path())
        };
        #[cfg(not(debug_assertions))]
        let result = self_replace::self_replace(temporary.path());
        if let Err(error) = result {
            #[cfg(windows)]
            let recovery = if !target.exists() {
                fs::copy(backup.path(), target).map(|_| ())
            } else {
                Ok(())
            };
            #[cfg(not(windows))]
            let recovery: std::io::Result<()> = Ok(());

            let message = if recovery.is_ok() {
                format!(
                    "replacement failed ({error}); existing executable was preserved or restored"
                )
            } else {
                format!(
                    "replacement failed ({error}) and automatic restoration failed ({recovery:?}); restore a saved executable before retrying"
                )
            };
            return Err(manual(&message));
        }

        #[cfg(windows)]
        {
            let backup_path = backup.path().to_path_buf();
            backup.close().map_err(|error| {
                (
                    "UPDATE_CLEANUP",
                    format!(
                        "update installed, but the previous executable backup at {} could not be removed ({error})",
                        backup_path.display()
                    ),
                )
            })?;
        }
        return Ok(());
    }

    #[cfg(not(debug_assertions))]
    return Err(manual(
        "the resolved update target is not the running executable; install manually",
    ));

    #[cfg(debug_assertions)]
    replace_fixture_target(target, temporary)
}

#[cfg(debug_assertions)]
fn replace_fixture_target(
    target: &std::path::Path,
    temporary: tempfile::NamedTempFile,
) -> Result<(), (&'static str, String)> {
    #[cfg(windows)]
    {
        let backup = tempfile::Builder::new()
            .prefix(".awtrix-cli-update-backup-")
            .tempfile_in(
                target
                    .parent()
                    .ok_or_else(|| manual("the executable has no safe parent directory"))?,
            )
            .map_err(|_| manual("cannot reserve a unique recovery path"))?;
        let backup_path = backup.into_temp_path();
        fs::remove_file(&backup_path).map_err(|_| {
            manual("cannot prepare the recovery path; existing executable was not changed")
        })?;
        fs::rename(target, &backup_path).map_err(|_| manual("cannot move the test target to a recovery path; existing executable was not changed"))?;
        if let Err(error) = temporary.persist(target) {
            let restore = fs::rename(&backup_path, target);
            return Err(manual(if restore.is_ok() {
                &format!("replacement failed ({error}); previous executable restored")
            } else {
                "replacement failed and automatic restoration failed; restore the backup beside the executable"
            }));
        }
        fs::remove_file(&backup_path).map_err(|_| {
            manual("update installed, but the previous executable backup could not be removed")
        })?;
    }
    #[cfg(not(windows))]
    temporary
        .persist(target)
        .map_err(|_| manual("atomic replacement failed; existing executable was preserved"))?;
    Ok(())
}

fn network<E: std::fmt::Display>(error: E) -> (&'static str, String) {
    ("UPDATE_NETWORK", format!("could not obtain the stable release ({error}); existing executable was not changed. Download and verify manually at {RELEASE_PAGE}"))
}

fn manual(reason: &str) -> (&'static str, String) {
    ("UPDATE_FAILED", format!("{reason}; existing executable was not changed. Follow the manual installation instructions at {RELEASE_PAGE}"))
}
