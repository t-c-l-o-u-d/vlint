// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};

const GITHUB_OWNER: &str = "t-c-l-o-u-d";
const GITHUB_REPO: &str = "vlint";
const UPDATE_COOLDOWN: Duration = Duration::from_secs(24 * 3600);

/// Try to auto-update vlint to the latest GitHub release. Non-fatal.
pub fn try_auto_update(verbose: bool) {
    if let Err(e) = run_update(verbose)
        && verbose
    {
        eprintln!("vlint: auto-update failed: {e}");
    }
}

fn run_update(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !cooldown_elapsed() {
        return Ok(());
    }
    record_cooldown();

    let current = env!("CARGO_PKG_VERSION");
    if verbose {
        eprintln!("vlint: checking for updates (current: v{current})...");
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("vlint/{}", env!("CARGO_PKG_VERSION")))
        .build()?;

    let (latest, bin_url, sha_url) = get_latest_release(&client, verbose)?;

    if !is_newer(&latest, current) {
        if verbose {
            eprintln!("vlint: already up to date (v{current})");
        }
        return Ok(());
    }

    if verbose {
        eprintln!("vlint: downloading v{latest} from {bin_url}");
    }

    let exe = std::env::current_exe()?;
    let tmp = exe.with_extension("update");
    download_file(&client, &bin_url, &tmp)?;
    verify_sha256(&client, &sha_url, &tmp)?;
    replace_binary(&tmp, &exe)?;

    println!("Updated vlint from v{current} to v{latest}");
    Ok(())
}

fn get_latest_release(
    client: &reqwest::blocking::Client,
    verbose: bool,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let api_url =
        format!("https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest");

    if verbose {
        eprintln!("vlint: fetching release info from {api_url}");
    }

    let resp = client.get(&api_url).send()?;

    if !resp.status().is_success() {
        return Err(format!(
            "failed to fetch release info: {} ({})",
            api_url,
            resp.status()
        )
        .into());
    }

    let release: serde_json::Value = serde_json::from_str(&resp.text()?)?;

    let tag = release["tag_name"]
        .as_str()
        .ok_or("no tag_name in release")?;
    let version = tag.strip_prefix('v').unwrap_or(tag);

    let assets = release["assets"].as_array().ok_or("no assets in release")?;

    let mut bin_url: Option<String> = None;
    let mut sha_url: Option<String> = None;
    for a in assets {
        let Some(name) = a["name"].as_str() else {
            continue;
        };
        let Some(url) = a["browser_download_url"].as_str() else {
            continue;
        };
        if name.contains("x86_64") && name.contains("linux") {
            if name.ends_with(".sha256") {
                sha_url = Some(url.to_string());
            } else {
                bin_url = Some(url.to_string());
            }
        }
    }

    let bin_url = bin_url.ok_or("no x86_64-linux asset found in release")?;
    let sha_url = sha_url.ok_or("no x86_64-linux .sha256 asset found in release")?;

    Ok((version.to_string(), bin_url, sha_url))
}

fn download_file(
    client: &reqwest::blocking::Client,
    url: &str,
    dest: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let resp = client.get(url).send()?;

    if !resp.status().is_success() {
        return Err(format!("failed to download: {} ({})", url, resp.status()).into());
    }

    let bytes = resp.bytes()?;
    fs::write(dest, &bytes)?;
    fs::set_permissions(dest, fs::Permissions::from_mode(0o755))?;

    Ok(())
}

fn verify_sha256(
    client: &reqwest::blocking::Client,
    sha_url: &str,
    bin_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let resp = client.get(sha_url).send()?;

    if !resp.status().is_success() {
        return Err(format!(
            "failed to download checksum: {} ({})",
            sha_url,
            resp.status()
        )
        .into());
    }

    let expected = resp
        .text()?
        .split_whitespace()
        .next()
        .ok_or("malformed sha256 file")?
        .to_lowercase();

    let bytes = fs::read(bin_path)?;
    let actual = Sha256::digest(&bytes)
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            write!(s, "{b:02x}").expect("write to String is infallible");
            s
        });

    if actual != expected {
        let _ = fs::remove_file(bin_path);
        return Err(format!("sha256 mismatch (expected {expected}, got {actual})").into());
    }

    Ok(())
}

fn replace_binary(new: &Path, current: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let old = current.with_extension("old");
    let _ = fs::remove_file(&old);

    fs::rename(current, &old).map_err(|e| format!("failed to backup current binary: {e}"))?;

    if let Err(e) = fs::rename(new, current) {
        let _ = fs::rename(&old, current);
        return Err(format!("failed to install new binary: {e}").into());
    }

    let _ = fs::remove_file(&old);
    Ok(())
}

#[must_use]
pub fn cooldown_elapsed() -> bool {
    let Some(path) = cooldown_path() else {
        return true;
    };
    let Ok(meta) = fs::metadata(&path) else {
        return true;
    };
    let Ok(modified) = meta.modified() else {
        return true;
    };
    let Ok(age) = std::time::SystemTime::now().duration_since(modified) else {
        return true;
    };
    age >= UPDATE_COOLDOWN
}

pub fn record_cooldown() {
    let Some(path) = cooldown_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(&path, "");
}

pub fn cooldown_path() -> Option<PathBuf> {
    let cache = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|h| PathBuf::from(h).join(".cache"))
        })?;
    Some(cache.join("vlint").join("last-update-check"))
}

/// Returns true if `latest` is strictly greater than `current` by semver.
#[must_use]
pub fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> [u64; 3] {
        let mut parts = s.splitn(3, '.').map(|p| p.parse().unwrap_or(0));
        [
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        ]
    };
    parse(latest) > parse(current)
}
