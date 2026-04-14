// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::catalog::linter::OwnedToolDef;

pub const DEFAULT_REGISTRY: &str = "ghcr.io/t-c-l-o-u-d/vlint-images";

/// Resolves the special value `"today"` to the current date as `YYYY-MM-DD`.
/// Any other value is returned as-is.
fn resolve_tag(tag: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    if tag != "today" {
        return tag.to_string();
    }
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86400;
    // Gregorian calendar conversion (Hinnant's algorithm, all u64)
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[must_use]
pub fn image_name(tool: &OwnedToolDef, registry: Option<&str>, prefix: &str, tag: &str) -> String {
    let reg = registry.unwrap_or(DEFAULT_REGISTRY);
    if prefix.is_empty() {
        format!("{reg}/{}:{}", tool.container_image, resolve_tag(tag))
    } else {
        format!(
            "{reg}/{prefix}{}:{}",
            tool.container_image,
            resolve_tag(tag)
        )
    }
}

pub struct ContainerConfig {
    pub runtime: String,
    pub registry: Option<String>,
    pub image_prefix: String,
    pub tag: String,
}

/// Read the current process's real UID and GID from `/proc/self/status`.
fn uid_gid() -> Option<(u32, u32)> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let mut uid = None;
    let mut gid = None;
    for line in status.lines() {
        if uid.is_none() && line.starts_with("Uid:") {
            uid = line.split_whitespace().nth(1).and_then(|s| s.parse().ok());
        }
        if gid.is_none() && line.starts_with("Gid:") {
            gid = line.split_whitespace().nth(1).and_then(|s| s.parse().ok());
        }
        if uid.is_some() && gid.is_some() {
            break;
        }
    }
    Some((uid?, gid?))
}

#[must_use]
pub fn build_run_args(
    config: &ContainerConfig,
    tool: &OwnedToolDef,
    workspace: &std::path::Path,
    args: &[&str],
    config_path: Option<&std::path::Path>,
) -> Vec<String> {
    let mut run_args = vec![
        "run".to_string(),
        "--rm".to_string(),
        "--pull=always".to_string(),
    ];

    let mount_opt = if tool.container_needs_rw_mount {
        "z"
    } else {
        "ro,z"
    };
    run_args.push("-v".to_string());
    run_args.push(format!("{}:/workspace:{mount_opt}", workspace.display()));
    run_args.push("-w".to_string());
    run_args.push("/workspace".to_string());

    // Mount the config file's parent directory read-only if it lives outside the workspace.
    // A targeted mount avoids SELinux relabeling restrictions that prevent mounting all of $HOME.
    if let Some(dir) = config_path
        .and_then(|p| p.parent())
        .filter(|d| !d.as_os_str().is_empty() && !d.starts_with(workspace))
    {
        run_args.push("-v".to_string());
        run_args.push(format!("{}:{}:ro,z", dir.display(), dir.display()));
    }

    if config.runtime == "podman" {
        run_args.push("--userns=keep-id".to_string());
    } else if config.runtime == "docker"
        && let Some((uid, gid)) = uid_gid()
    {
        run_args.push("--user".to_string());
        run_args.push(format!("{uid}:{gid}"));
    }

    if !tool.container_needs_network {
        run_args.push("--network=none".to_string());
    }

    for (key, val) in tool.env_vars.iter().chain(&tool.container_env_vars) {
        run_args.push("-e".to_string());
        run_args.push(format!("{key}={val}"));
    }

    run_args.push(image_name(
        tool,
        config.registry.as_deref(),
        &config.image_prefix,
        &config.tag,
    ));

    run_args.push(tool.binary_name.clone());
    run_args.extend(args.iter().map(|a| (*a).to_string()));

    run_args
}

/// Return the local image ID for a fully qualified image reference, or `None`
/// if the image is not present locally.
#[must_use]
pub fn local_image_id(runtime: &str, image: &str) -> Option<String> {
    std::process::Command::new(runtime)
        .args(["image", "inspect", "--format", "{{.Id}}", image])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Remove a specific image by ID.  Silently ignored if the image is still
/// tagged or otherwise in use.
pub fn remove_image(runtime: &str, id: &str) {
    let _ = std::process::Command::new(runtime)
        .args(["rmi", id])
        .output();
}
