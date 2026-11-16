//! Background jobs: exporting + building the jar with Gradle, fetching versions,
//! and app settings on disk.

use crate::codegen;
use crate::model::{Platform, Project, Versions};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Done,
    Warn,
    Error,
}

pub enum Msg {
    Line(Level, String),
    Finished(Result<PathBuf, String>),
}

pub struct Job {
    pub rx: Receiver<Msg>,
    cancel: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

impl Job {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Ok(mut c) = self.child.lock() {
            if let Some(ch) = c.as_mut() {
                let _ = ch.kill();
            }
        }
    }
}

/// Folder the Gradle project is exported to, next to the project file.
pub fn export_dir(project_file: &Path, project: &Project) -> PathBuf {
    let parent = project_file
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let plat = match project.settings.platform {
        Platform::Paper => "paper",
        Platform::Fabric => "fabric",
    };
    parent.join(format!("{}-{}-build", project.settings.id, plat))
}

fn send(tx: &Sender<Msg>, lvl: Level, s: impl Into<String>) {
    let _ = tx.send(Msg::Line(lvl, s.into()));
}

/// Exports the project and, if `run_gradle`, builds the jar.
pub fn start(
    project: Project,
    project_file: Option<PathBuf>,
    dir: PathBuf,
    run_gradle: bool,
    server: Option<crate::server::ServerConfig>,
    ctx: egui::Context,
) -> Job {
    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let child: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let job = Job {
        rx,
        cancel: cancel.clone(),
        child: child.clone(),
    };
    std::thread::spawn(move || {
        let res = run(
            &project,
            project_file.as_deref(),
            &dir,
            run_gradle,
            server.as_ref(),
            &tx,
            &cancel,
            &child,
        );
        let _ = tx.send(Msg::Finished(res));
        ctx.request_repaint();
    });
    job
}

fn run(
    project: &Project,
    project_file: Option<&Path>,
    dir: &Path,
    run_gradle: bool,
    server: Option<&crate::server::ServerConfig>,
    tx: &Sender<Msg>,
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<Child>>,
) -> Result<PathBuf, String> {
    let mut out = codegen::generate(project);
    if out.has_errors() {
        for p in out.problems.iter().filter(|p| p.error) {
            send(tx, Level::Error, p.msg.clone());
        }
        return Err(crate::i18n::t(
            "Das Projekt hat Fehler – siehe „Probleme“.",
            "The project has errors – see “Problems”.",
        )
        .into());
    }
    send(
        tx,
        Level::Info,
        format!("Exportiere nach {}", dir.display()),
    );
    let pack = codegen::pack_files(project, project_file)?;
    if !pack.is_empty() {
        send(
            tx,
            Level::Info,
            format!("Resourcepack: {} Dateien werden eingebaut", pack.len()),
        );
    }
    out.files.extend(pack);
    codegen::write_to(&out, dir, &project.settings.package)
        .map_err(|e| format!("Export fehlgeschlagen: {e}"))?;
    if !run_gradle {
        return Ok(dir.to_path_buf());
    }

    let java = find_java().ok_or_else(|| {
        crate::i18n::t(
            "Java wurde nicht gefunden. Installiere Java 21 oder neuer (z. B. von adoptium.net) und starte den Builder neu.",
            "Java was not found. Install Java 21 or newer (e.g. from adoptium.net) and restart the builder.",
        )
        .to_string()
    })?;
    send(tx, Level::Info, format!("Java: {}", java.display()));
    let first = !crate::tools::gradle_ready();
    send(
        tx,
        Level::Info,
        if first {
            crate::i18n::t(
                "Starte Gradle… Beim allerersten Build werden Gradle und die Minecraft-Dateien heruntergeladen – das dauert einmalig einige Minuten.",
                "Starting Gradle… The very first build downloads Gradle and the Minecraft files – this takes a few minutes once.",
            )
        } else {
            crate::i18n::t("Baue …", "Building …")
        },
    );
    let started = std::time::Instant::now();

    // Old builds (e.g. of an earlier version number) would otherwise stay in build/libs.
    if let Ok(entries) = std::fs::read_dir(dir.join("build/libs")) {
        for e in entries.flatten() {
            if e.path().extension().is_some_and(|x| x == "jar") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }

    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("gradlew.bat");
        c
    } else {
        Command::new(dir.join("gradlew"))
    };
    cmd.args(["build", "--console=plain", "--warning-mode=none"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(home) = java.parent().and_then(|b| b.parent()) {
        cmd.env("JAVA_HOME", home);
    }
    crate::tools::hide_window(&mut cmd);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Gradle konnte nicht gestartet werden: {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    *child_slot.lock().unwrap() = Some(child);

    let mut readers = vec![];
    if let Some(s) = stdout {
        let tx = tx.clone();
        readers.push(std::thread::spawn(move || {
            pump(BufReader::new(s), &tx, false)
        }));
    }
    if let Some(s) = stderr {
        let tx = tx.clone();
        readers.push(std::thread::spawn(move || {
            pump(BufReader::new(s), &tx, true)
        }));
    }
    for r in readers {
        let _ = r.join();
    }
    let status = {
        let mut slot = child_slot.lock().unwrap();
        let st = slot.as_mut().map(|c| c.wait());
        *slot = None;
        st
    };
    if cancel.load(Ordering::Relaxed) {
        return Err(crate::i18n::t("Build abgebrochen.", "Build cancelled.").into());
    }
    match status {
        Some(Ok(s)) if s.success() => {}
        _ => {
            return Err(crate::i18n::t(
                "Gradle-Build fehlgeschlagen – Details stehen in der Konsole.",
                "Gradle build failed – see the console for details.",
            )
            .into());
        }
    }

    let jar = find_jar(&dir.join("build/libs")).ok_or("Keine .jar-Datei in build/libs gefunden")?;
    send(
        tx,
        Level::Info,
        format!(
            "{} {:.0} s",
            crate::i18n::t("Build fertig nach", "Build finished after"),
            started.elapsed().as_secs_f32()
        ),
    );
    if !project.settings.deploy_dir.trim().is_empty() {
        let target_dir = PathBuf::from(project.settings.deploy_dir.trim());
        let target = target_dir.join(jar.file_name().unwrap());
        match std::fs::create_dir_all(&target_dir).and_then(|_| std::fs::copy(&jar, &target)) {
            Ok(_) => {
                send(
                    tx,
                    Level::Done,
                    format!("Kopiert nach {}", target.display()),
                );
                // Remove older versions of this plugin, otherwise the server loads it twice.
                let new_name = target
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let mut removed = vec![];
                if let Ok(entries) = std::fs::read_dir(&target_dir) {
                    for e in entries.flatten() {
                        let n = e.file_name().to_string_lossy().to_string();
                        if is_old_build(&n, &project.settings.id, &new_name)
                            && std::fs::remove_file(e.path()).is_ok()
                        {
                            removed.push(n);
                        }
                    }
                }
                if !removed.is_empty() {
                    send(
                        tx,
                        Level::Info,
                        format!("Alte Version entfernt: {}", removed.join(", ")),
                    );
                }
                // Optionally delete the data folder of the local test server too.
                if project.settings.wipe_data_on_deploy {
                    let (parent, name) = project.settings.data_folder();
                    let data = match project.settings.platform {
                        Platform::Paper => Some(target_dir.join(&name)),
                        Platform::Fabric => target_dir.parent().map(|d| d.join(parent).join(&name)),
                    };
                    if let Some(d) = data.filter(|d| d.is_dir() && !name.trim().is_empty()) {
                        match std::fs::remove_dir_all(&d) {
                            Ok(()) => send(
                                tx,
                                Level::Info,
                                format!("Datenordner gelöscht: {}", d.display()),
                            ),
                            Err(e) => send(
                                tx,
                                Level::Warn,
                                format!("Datenordner {} nicht gelöscht: {e}", d.display()),
                            ),
                        }
                    }
                }
            }
            Err(e) => send(
                tx,
                Level::Warn,
                format!("Kopieren nach {} fehlgeschlagen: {e}", target_dir.display()),
            ),
        }
    }
    if let Some(server) = server {
        send(
            tx,
            Level::Info,
            format!(
                "Lade {} auf den Server „{}“ hoch…",
                jar.file_name().unwrap_or_default().to_string_lossy(),
                server.display_name()
            ),
        );
        let folder = match project.settings.platform {
            Platform::Paper => "plugins",
            Platform::Fabric => "mods",
        };
        let (data_parent, data_name) = project.settings.data_folder();
        let wipe = project
            .settings
            .wipe_data_on_deploy
            .then_some((data_parent, data_name.as_str()));
        match crate::server::deploy(server, &jar, folder, &project.settings.id, wipe) {
            Ok(lines) => {
                for l in lines {
                    send(tx, Level::Done, l);
                }
            }
            Err(e) => send(tx, Level::Error, format!("Upload fehlgeschlagen: {e}")),
        }
    }
    Ok(jar)
}

fn pump<R: BufRead>(r: R, tx: &Sender<Msg>, is_err: bool) {
    for line in r.lines().map_while(Result::ok) {
        let l = line.trim_end();
        if l.is_empty() || l.starts_with("Picked up JAVA_TOOL_OPTIONS") {
            continue;
        }
        let lower = l.to_lowercase();
        let lvl = if lower.contains("error:")
            || l.contains("FAILED")
            || lower.starts_with("* what went wrong")
        {
            Level::Error
        } else if lower.contains("warning:") || (is_err && !lower.starts_with("note:")) {
            Level::Warn
        } else if l.contains("BUILD SUCCESSFUL") {
            Level::Done
        } else {
            Level::Info
        };
        let _ = tx.send(Msg::Line(lvl, l.to_string()));
    }
}

/// Is `name` an older build of the project `id` (`<id>-<version>.jar`), not `new_name`?
/// The version must start with a digit, so "home-extra-1.0.jar" is not taken for "home".
pub fn is_old_build(name: &str, id: &str, new_name: &str) -> bool {
    let Some(rest) = name.strip_prefix(&format!("{id}-")) else {
        return false;
    };
    name != new_name
        && name.ends_with(".jar")
        && rest.chars().next().is_some_and(|c| c.is_ascii_digit())
}

fn find_jar(libs: &Path) -> Option<PathBuf> {
    let mut jars: Vec<PathBuf> = std::fs::read_dir(libs)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            n.ends_with(".jar") && !n.contains("-sources") && !n.contains("-dev")
        })
        .collect();
    jars.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
    jars.pop()
}

pub use crate::tools::find_java;

// ---------------------------------------------------------------------------
// Version lookup
// ---------------------------------------------------------------------------

fn http_get(url: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .user_agent("minecraft-builder")
        .build()
        .into();
    agent
        .get(url)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())
}

fn maven_versions(xml: &str) -> Vec<String> {
    let mut out = vec![];
    let mut rest = xml;
    while let Some(s) = rest.find("<version>") {
        let after = &rest[s + 9..];
        if let Some(e) = after.find("</version>") {
            out.push(after[..e].trim().to_string());
            rest = &after[e..];
        } else {
            break;
        }
    }
    out
}

/// Numeric key for sorting versions like "26.3.build.249-stable".
fn version_key(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

pub fn java_for(mc: &str) -> u32 {
    if mc.starts_with("1.") { 21 } else { 25 }
}

/// Looks up the newest library versions that fit the Minecraft version.
pub fn fetch_versions(platform: Platform, mc: &str) -> Result<Versions, String> {
    let mc = mc.trim();
    let mut v = Versions::default_for(platform);
    v.minecraft = mc.to_string();
    v.java = java_for(mc);
    match platform {
        Platform::Paper => {
            let xml = http_get(
                "https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/maven-metadata.xml",
            )?;
            let all = maven_versions(&xml);
            let best = if mc.starts_with("1.") {
                let want = format!("{mc}-R0.1-SNAPSHOT");
                all.into_iter().find(|x| *x == want)
            } else {
                let prefix = format!("{mc}.build.");
                all.into_iter()
                    .filter(|x| x.starts_with(&prefix) && x.ends_with("-stable"))
                    .max_by_key(|x| version_key(x))
            };
            v.api = best.ok_or_else(|| format!("Keine Paper-API für Minecraft {mc} gefunden"))?;
        }
        Platform::Fabric => {
            if mc.starts_with("1.") {
                return Err("Fabric-Mods werden ab Minecraft 26.1 unterstützt".into());
            }
            let loader = http_get("https://meta.fabricmc.net/v2/versions/loader")?;
            let loader: serde_json::Value =
                serde_json::from_str(&loader).map_err(|e| e.to_string())?;
            if let Some(l) = loader
                .as_array()
                .and_then(|a| a.iter().find(|x| x["stable"].as_bool() == Some(true)))
                .and_then(|x| x["version"].as_str())
            {
                v.loader = l.to_string();
            }
            let xml = http_get(
                "https://maven.fabricmc.net/net/fabricmc/fabric-api/fabric-api/maven-metadata.xml",
            )?;
            let suffix = format!("+{mc}");
            v.api = maven_versions(&xml)
                .into_iter()
                .filter(|x| x.ends_with(&suffix))
                .max_by_key(|x| version_key(x.split('+').next().unwrap_or("")))
                .ok_or_else(|| format!("Keine Fabric API für Minecraft {mc} gefunden"))?;
        }
    }
    Ok(v)
}

/// Minecraft versions offered in the version picker.
pub fn fetch_minecraft_versions() -> Result<Vec<String>, String> {
    let txt = http_get("https://meta.fabricmc.net/v2/versions/game")?;
    let val: serde_json::Value = serde_json::from_str(&txt).map_err(|e| e.to_string())?;
    Ok(val
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|x| x["stable"].as_bool() == Some(true))
                .filter_map(|x| x["version"].as_str().map(String::from))
                .filter(|s| !s.starts_with("1.") || s == "1.21.11")
                .take(12)
                .collect()
        })
        .unwrap_or_default())
}

// ---------------------------------------------------------------------------
// App settings
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub german: bool,
    pub recent: Vec<PathBuf>,
    pub snap: bool,
    #[serde(default)]
    pub ui_scale: f32,
    /// First-start wizard finished.
    #[serde(default)]
    pub onboarded: bool,
    /// Left drag on empty canvas draws a selection box instead of panning.
    #[serde(default)]
    pub left_drag_selects: bool,
    /// Recently placed block ids (newest first).
    #[serde(default)]
    pub recent_blocks: Vec<String>,
    #[serde(default)]
    pub favorite_blocks: Vec<String>,
    /// Download and install updates automatically.
    #[serde(default = "default_true")]
    pub auto_update: bool,
    /// Show a short description on every block.
    #[serde(default = "default_true")]
    pub block_descriptions: bool,
    /// Show the block number (#12) in block headers.
    #[serde(default)]
    pub block_ids: bool,
    /// Show the type name next to output sockets.
    #[serde(default = "default_true")]
    pub pin_types: bool,
    /// Animated dots on the wires that show the flow direction.
    #[serde(default = "default_true")]
    pub wire_flow: bool,
    /// Use item textures from a Minecraft installation on this computer in the menu editor.
    #[serde(default = "default_true")]
    pub vanilla_textures: bool,
    /// Vanilla textures and models: a template resource pack (.zip) or the
    /// Minecraft client .jar (empty = look for an installed Minecraft).
    #[serde(default, alias = "minecraft_jar")]
    pub texture_source: String,
    /// Save the project automatically before building.
    #[serde(default = "default_true")]
    pub autosave_on_build: bool,
    /// Game server panels the built jar can be uploaded to.
    #[serde(default)]
    pub servers: Vec<crate::server::ServerConfig>,
    #[serde(default)]
    pub next_server_id: u64,
    /// Views moved away from their usual side.
    #[serde(default)]
    pub docks: Vec<(crate::app::View, crate::app::Dock)>,
    /// MCP server for AI assistants: on/off, port and secret part of the address.
    #[serde(default)]
    pub mcp_enabled: bool,
    #[serde(default)]
    pub mcp_port: u16,
    #[serde(default)]
    pub mcp_token: String,
}

fn default_true() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            german: true,
            recent: vec![],
            snap: true,
            ui_scale: 1.0,
            onboarded: false,
            left_drag_selects: false,
            recent_blocks: vec![],
            favorite_blocks: vec![],
            auto_update: true,
            block_descriptions: true,
            block_ids: false,
            pin_types: true,
            wire_flow: true,
            vanilla_textures: true,
            texture_source: String::new(),
            autosave_on_build: true,
            servers: vec![],
            next_server_id: 1,
            docks: vec![],
            mcp_enabled: false,
            mcp_port: 8765,
            mcp_token: String::new(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("MinecraftBuilder")
}

/// Default folder for new projects: Documents/Minecraft Builder.
pub fn default_projects_dir() -> PathBuf {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let docs = ["Documents", "Dokumente"]
        .iter()
        .map(|d| home.join(d))
        .find(|p| p.is_dir())
        .unwrap_or(home);
    docs.join("Minecraft Builder")
}

impl AppSettings {
    pub fn load() -> Self {
        std::fs::read_to_string(config_dir().join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) {
        let dir = config_dir();
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(dir.join("settings.json"), s);
        }
    }
    /// The zip/jar the vanilla item pictures come from, if any.
    /// The Minecraft textures to use. Looking on disk is slow (the .minecraft search
    /// lists folders), so the answer is kept for a few seconds.
    pub fn vanilla_source(&self) -> Option<PathBuf> {
        type Cached = Option<(std::time::Instant, String, Option<PathBuf>)>;
        static CACHE: std::sync::Mutex<Cached> = std::sync::Mutex::new(None);
        let custom = self.texture_source.trim().to_string();
        if let Ok(c) = CACHE.lock() {
            if let Some((at, key, found)) = c.as_ref() {
                if *key == custom && at.elapsed().as_secs_f32() < 5.0 {
                    return found.clone();
                }
            }
        }
        let found = if !custom.is_empty() {
            let p = PathBuf::from(&custom);
            p.is_file().then_some(p)
        } else {
            crate::resourcepack::find_minecraft_jar()
        };
        if let Ok(mut c) = CACHE.lock() {
            *c = Some((std::time::Instant::now(), custom, found.clone()));
        }
        found
    }
    pub fn server(&self, id: u64) -> Option<&crate::server::ServerConfig> {
        self.servers.iter().find(|s| s.id == id && id != 0)
    }
    pub fn add_recent(&mut self, p: &Path) {
        self.recent.retain(|r| r != p);
        self.recent.insert(0, p.to_path_buf());
        self.recent.truncate(8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_old_builds_only_of_this_project() {
        assert!(is_old_build("home-1.0.0.jar", "home", "home-1.1.0.jar"));
        assert!(!is_old_build("home-1.1.0.jar", "home", "home-1.1.0.jar"));
        assert!(!is_old_build(
            "home-extra-1.0.jar",
            "home",
            "home-1.1.0.jar"
        ));
        assert!(!is_old_build("homes-1.0.jar", "home", "home-1.1.0.jar"));
        assert!(!is_old_build("home-1.0.txt", "home", "home-1.1.0.jar"));
    }
}
