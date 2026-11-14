//! Automatic updates from GitHub releases.
//!
//! The app asks GitHub for the latest release. If it is newer, the installer
//! (`MinecraftBuilder-Setup-<version>.exe`) is downloaded in the background and
//! installed silently when the app closes – or right away with "Restart now".

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{Receiver, Sender, channel};

pub const REPO: &str = "hullihup/Minecraft-Builder";

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub page: String,
    /// Download URL of the installer, if the release has one.
    pub installer: Option<String>,
    pub installer_name: String,
}

pub enum UpdateMsg {
    /// Result of the check: newest release if it is newer than this app.
    Checked(Result<Option<Release>, String>),
    Progress(f32),
    Downloaded(Result<PathBuf, String>),
}

/// "0.3.1" > "0.3.0"?
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim()
            .trim_start_matches(['v', 'V'])
            .split(['.', '-', '+'])
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parse(candidate), parse(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x > y;
        }
    }
    false
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// True if this exe was installed with the installer (an uninstaller sits next to it).
pub fn is_installed() -> bool {
    cfg!(windows)
        && std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .map(|d| d.join("unins000.exe").is_file())
            .unwrap_or(false)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(600)))
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .user_agent("minecraft-builder-updater")
        .build()
        .into()
}

/// Looks up the latest release on GitHub.
pub fn check() -> Result<Option<Release>, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let mut resp = agent()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(404) => crate::i18n::t(
                "Es gibt noch kein veröffentlichtes Update.",
                "There is no published update yet.",
            )
            .to_string(),
            other => other.to_string(),
        })?;
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let tag = v["tag_name"].as_str().unwrap_or_default().to_string();
    if tag.is_empty() || !is_newer(&tag, current_version()) {
        return Ok(None);
    }
    let asset = v["assets"].as_array().and_then(|a| {
        a.iter().find(|x| {
            let n = x["name"].as_str().unwrap_or_default();
            n.starts_with("MinecraftBuilder-Setup") && n.ends_with(".exe")
        })
    });
    Ok(Some(Release {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        notes: v["body"].as_str().unwrap_or_default().to_string(),
        page: v["html_url"].as_str().unwrap_or_default().to_string(),
        installer: asset
            .and_then(|a| a["browser_download_url"].as_str())
            .map(String::from),
        installer_name: asset
            .and_then(|a| a["name"].as_str())
            .unwrap_or("MinecraftBuilder-Setup.exe")
            .to_string(),
    }))
}

pub fn start_check(ctx: egui::Context) -> Receiver<UpdateMsg> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(UpdateMsg::Checked(check()));
        ctx.request_repaint();
    });
    rx
}

/// Downloads the installer into the temp folder.
pub fn start_download(rel: Release, ctx: egui::Context) -> Receiver<UpdateMsg> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let r = download(&rel, &tx, &ctx);
        let _ = tx.send(UpdateMsg::Downloaded(r));
        ctx.request_repaint();
    });
    rx
}

fn download(rel: &Release, tx: &Sender<UpdateMsg>, ctx: &egui::Context) -> Result<PathBuf, String> {
    let url = rel.installer.clone().ok_or_else(|| {
        crate::i18n::t(
            "Dieses Update hat keinen Installer.",
            "This update has no installer.",
        )
        .to_string()
    })?;
    let dir = std::env::temp_dir().join("MinecraftBuilder-Update");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let target = dir.join(&rel.installer_name);
    let part = dir.join(format!("{}.part", rel.installer_name));
    let mut resp = agent().get(&url).call().map_err(|e| e.to_string())?;
    let total: u64 = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut reader = resp.body_mut().as_reader();
    let mut file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 128 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if total > 0 {
            let _ = tx.send(UpdateMsg::Progress(done as f32 / total as f32));
            ctx.request_repaint();
        }
    }
    drop(file);
    if total > 0 && done != total {
        return Err(crate::i18n::t("Download unvollständig.", "Download incomplete.").into());
    }
    std::fs::rename(&part, &target).map_err(|e| e.to_string())?;
    Ok(target)
}

/// Starts the installer silently. With `relaunch` the app is started again afterwards.
pub fn run_installer(path: &Path, relaunch: bool) -> Result<(), String> {
    let mut cmd = Command::new(path);
    cmd.args([
        "/VERYSILENT",
        "/SUPPRESSMSGBOXES",
        "/NORESTART",
        "/CLOSEAPPLICATIONS",
    ]);
    if relaunch {
        cmd.arg("/RELAUNCH=1");
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn versions_compare() {
        assert!(is_newer("v0.3.1", "0.3.0"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(!is_newer("v0.3.0", "0.3.0"));
        assert!(!is_newer("0.2.9", "0.3.0"));
    }
}
