//! Portable self-update from the fork's GitHub releases.
//!
//! The release ZIP is signed with the fork's own key (`tauri signer sign`);
//! nothing is installed unless the signature matches `PUBKEY`. Files are
//! swapped by renaming (Windows lets a running exe and its DLLs be renamed),
//! the replaced ones are parked in `_old/` and removed on the next start.
//! `Data/` is never touched.

use base64::Engine;
use futures_util::StreamExt;
use std::io;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

const LATEST_RELEASE_API: &str = "https://api.github.com/repos/gaviash/Handy-Flow/releases/latest";
const ZIP_ASSET: &str = "Handy-Flow-portable-win-x64.zip";
/// Public half of the key in `%USERPROFILE%\.handy-flow\updater.key`.
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IENBRkU3M0I3RDgwNzkzQQpSV1E2ZVlCOU8rZXZERHBGTHkyMlBjTUtRYlZYZmtPWnlteFlhWGxhN21zaUZMZ01aZU9Xb1k5Vwo=";
const STAGING: &str = "_update";
const BACKUP: &str = "_old";
/// Top-level entries an update never overwrites.
const KEEP: [&str; 3] = ["Data", STAGING, BACKUP];

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(serde::Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// `Some(version)` when the release tag (`flow-v0.2.2`) is newer than `current`.
fn newer_version(tag: &str, current: &str) -> Option<String> {
    let remote = semver::Version::parse(tag.trim_start_matches("flow-v")).ok()?;
    let local = semver::Version::parse(current).ok()?;
    (remote > local).then(|| remote.to_string())
}

fn verify_signature(data: &[u8], sig_file: &str, pubkey_b64: &str) -> Result<(), String> {
    let b64 = |s: &str| -> Result<String, String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(s.trim())
            .map_err(|e| e.to_string())?;
        String::from_utf8(bytes).map_err(|e| e.to_string())
    };
    let pk = minisign_verify::PublicKey::decode(&b64(pubkey_b64)?).map_err(|e| e.to_string())?;
    let sig = minisign_verify::Signature::decode(&b64(sig_file)?).map_err(|e| e.to_string())?;
    pk.verify(data, &sig, true).map_err(|e| e.to_string())
}

/// Every file under `dir`, relative to it.
fn files_under(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        for entry in std::fs::read_dir(dir.join(&rel))? {
            let entry = entry?;
            let path = rel.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    Ok(out)
}

/// Moves every file of `root/_update` into `root`, parking the files it
/// replaces under `root/_old`. Any failure puts everything back.
fn swap_in(root: &Path) -> io::Result<()> {
    let staging = root.join(STAGING);
    let backup = root.join(BACKUP);
    let mut done: Vec<(PathBuf, bool)> = Vec::new();
    let result = (|| {
        for rel in files_under(&staging)? {
            let top = rel.components().next().map(|c| c.as_os_str().to_owned());
            if top.is_some_and(|t| KEEP.iter().any(|k| t == *k)) {
                continue;
            }
            let target = root.join(&rel);
            let had_old = target.exists();
            if had_old {
                std::fs::create_dir_all(backup.join(&rel).parent().unwrap())?;
                std::fs::rename(&target, backup.join(&rel))?;
            }
            done.push((rel.clone(), had_old));
            std::fs::create_dir_all(target.parent().unwrap())?;
            std::fs::rename(staging.join(&rel), &target)?;
        }
        Ok(())
    })();
    if result.is_err() {
        for (rel, had_old) in done.iter().rev() {
            let target = root.join(rel);
            if target.exists() {
                let _ = std::fs::rename(&target, staging.join(rel));
            }
            if *had_old {
                let _ = std::fs::rename(backup.join(rel), &target);
            }
        }
    }
    result
}

fn portable_root() -> Result<PathBuf, String> {
    crate::portable::data_dir()
        .and_then(|d| d.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| "self-update only runs in the portable build".to_string())
}

/// Leftovers of the previous update. Best effort: the old exe may still be
/// exiting, in which case the next start finishes the job.
pub fn cleanup_previous_update() {
    if let Ok(root) = portable_root() {
        for dir in [STAGING, BACKUP] {
            let _ = std::fs::remove_dir_all(root.join(dir));
        }
    }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Handy-Flow-updater")
        .build()
        .map_err(|e| e.to_string())
}

async fn latest_release(client: &reqwest::Client) -> Result<Release, String> {
    client
        .get(LATEST_RELEASE_API)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

fn asset_url<'a>(release: &'a Release, name: &str) -> Result<&'a str, String> {
    release
        .assets
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.browser_download_url.as_str())
        .ok_or_else(|| format!("release {} has no {name}", release.tag_name))
}

/// The newer version published on the fork, if any.
#[tauri::command]
#[specta::specta]
pub async fn check_for_update() -> Result<Option<String>, String> {
    if crate::settings::update_checks_forced_disabled() {
        return Ok(None);
    }
    let release = latest_release(&client()?).await?;
    Ok(newer_version(&release.tag_name, env!("CARGO_PKG_VERSION")))
}

/// Downloads, verifies and installs the latest release, then restarts.
/// Emits `update-download-progress` (0-100) while downloading.
#[tauri::command]
#[specta::specta]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    install(app)
        .await
        .inspect_err(|e| log::error!("Update failed: {e}"))
}

async fn install(app: AppHandle) -> Result<(), String> {
    if crate::settings::update_checks_forced_disabled() {
        return Err("updates are disabled".into());
    }
    let root = portable_root()?;
    let client = client()?;
    let release = latest_release(&client).await?;
    let sig = client
        .get(asset_url(&release, &format!("{ZIP_ASSET}.sig"))?)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let response = client
        .get(asset_url(&release, ZIP_ASSET)?)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let total = response.content_length().unwrap_or(0);
    let mut zip = Vec::with_capacity(total as usize);
    let mut stream = response.bytes_stream();
    let mut last_percent = None;
    while let Some(chunk) = stream.next().await {
        zip.extend_from_slice(&chunk.map_err(|e| e.to_string())?);
        let percent = (zip.len() as u64 * 100).checked_div(total);
        if percent.is_some() && percent != last_percent {
            last_percent = percent;
            let _ = app.emit("update-download-progress", percent);
        }
    }

    verify_signature(&zip, &sig, PUBKEY).map_err(|e| format!("update signature rejected: {e}"))?;

    let staging = root.join(STAGING);
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_dir_all(root.join(BACKUP));
    zip::ZipArchive::new(io::Cursor::new(zip))
        .and_then(|mut archive| archive.extract(&staging))
        .map_err(|e| format!("could not unpack the update: {e}"))?;
    swap_in(&root).map_err(|e| format!("could not install the update: {e}"))?;

    log::info!("Updated to {}, restarting", release.tag_name);
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_strictly_newer_tags_count() {
        assert_eq!(
            newer_version("flow-v0.2.2", "0.2.1").as_deref(),
            Some("0.2.2")
        );
        assert_eq!(
            newer_version("flow-v0.10.0", "0.9.9").as_deref(),
            Some("0.10.0")
        );
        assert_eq!(newer_version("flow-v0.2.1", "0.2.1"), None);
        assert_eq!(newer_version("flow-v0.2.0", "0.2.1"), None);
        assert_eq!(newer_version("nightly", "0.2.1"), None);
    }

    #[test]
    fn pubkey_matches_the_fork_key_and_not_upstream() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_ne!(conf["plugins"]["updater"]["pubkey"], PUBKEY);
        assert!(minisign_verify::PublicKey::decode(
            &String::from_utf8(
                base64::engine::general_purpose::STANDARD
                    .decode(PUBKEY)
                    .unwrap()
            )
            .unwrap()
        )
        .is_ok());
    }

    #[test]
    fn swap_replaces_files_keeps_data_and_parks_old_ones() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let write = |p: &str, s: &str| {
            let p = root.join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, s).unwrap();
        };
        write("Handy Flow.exe", "old exe");
        write("resources/a.wav", "old a");
        write("Data/settings.json", "mine");
        write("_update/Handy Flow.exe", "new exe");
        write("_update/resources/a.wav", "new a");
        write("_update/resources/b.wav", "new b");
        write("_update/Data/settings.json", "never");

        swap_in(root).unwrap();

        let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
        assert_eq!(read("Handy Flow.exe"), "new exe");
        assert_eq!(read("resources/a.wav"), "new a");
        assert_eq!(read("resources/b.wav"), "new b");
        assert_eq!(read("Data/settings.json"), "mine");
        assert_eq!(read("_old/Handy Flow.exe"), "old exe");
    }

    #[test]
    fn failed_swap_restores_the_previous_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.dll"), "old a").unwrap();
        std::fs::create_dir_all(root.join("_update/zz")).unwrap();
        std::fs::write(root.join("_update/a.dll"), "new a").unwrap();
        std::fs::write(root.join("_update/zz/f"), "new f").unwrap();
        // A file where a directory must go makes the second move fail.
        std::fs::write(root.join("zz"), "blocker").unwrap();

        assert!(swap_in(root).is_err());
        assert_eq!(
            std::fs::read_to_string(root.join("a.dll")).unwrap(),
            "old a"
        );
        assert_eq!(std::fs::read_to_string(root.join("zz")).unwrap(), "blocker");
    }

    #[test]
    fn signature_must_match_the_data() {
        // Signed with a throwaway key generated by `tauri signer generate`.
        let (pk, sig) = (TEST_PUBKEY, TEST_SIG);
        assert!(verify_signature(b"handy flow\n", sig, pk).is_ok());
        assert!(verify_signature(b"tampered\n", sig, pk).is_err());
        assert!(verify_signature(b"handy flow\n", sig, PUBKEY).is_err());
    }

    const TEST_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEU4NTQxRDEwRTczRUU2MTEKUldRUjVqN25FQjFVNkpyV0g1TDFXNUJoUjNyYWNPWWVpK2c5eU9GOUdvZGVxSExJRUdDSVBrTWoK";
    const TEST_SIG: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRUjVqN25FQjFVNkF5bnIva2FvRTlZMXZ5dVp0aXZoZlpGQTlXMjNZdVZTUkt4MU92c2lhYW5DdTZCTmZnMGJYOVV0MlpuN3hoaldIY2JrS1F2NXJDM0VDTGkraXFaS2cwPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk0OTM5MzU5CWZpbGU6ZGF0YS50eHQKalpOME4rUFlYWFU0RUN1T1B3d2ZaWmJJQ3lHQlVOSks1dFplU2c4eVNiSlljbWVlbXk2ZDg2MFhkYi95TEp6UFh4OGtTTmNwVEE0SXNadEFTa3ZlRGc9PQo=";
}
