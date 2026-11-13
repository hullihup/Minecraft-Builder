//! Build tools: finds Java, downloads a portable JDK if needed and prepares Gradle,
//! so the first real build is fast. Used by the first-start wizard, the
//! "Bauen & Export" panel and the installer (`--setup-tools`).

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};

pub const JDK_VERSION: u32 = 25;

pub enum SetupMsg {
    Status(String),
    /// 0.0..=1.0, or negative for "unknown".
    Progress(f32),
    Line(String),
    Done(Result<(), String>),
}

/// Folder for the portable JDK and the Gradle warm-up project.
pub fn tools_dir() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("MinecraftBuilder")
}

fn exe() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

/// The JDK downloaded by this app, if any.
pub fn bundled_java() -> Option<PathBuf> {
    let jdk = tools_dir().join("jdk");
    [
        jdk.join("bin").join(exe()),
        jdk.join("Contents/Home/bin").join(exe()),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

/// Major version of a java executable (`java -version` output).
pub fn java_version(java: &Path) -> Option<u32> {
    let mut cmd = Command::new(java);
    cmd.arg("-version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_window(&mut cmd);
    let out = cmd.output().ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let start = text.find("version \"")? + 9;
    let ver = &text[start..];
    let ver = &ver[..ver.find('"')?];
    let mut parts = ver.split(['.', '-', '+', '_']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

fn system_java_candidates() -> Vec<PathBuf> {
    let mut out = vec![];
    if let Ok(home) = std::env::var("JAVA_HOME") {
        out.push(PathBuf::from(home).join("bin").join(exe()));
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&paths) {
            out.push(d.join(exe()));
        }
    }
    if cfg!(windows) {
        for root in [
            "C:\\Program Files\\Eclipse Adoptium",
            "C:\\Program Files\\Java",
            "C:\\Program Files\\Microsoft",
            "C:\\Program Files\\Zulu",
        ] {
            if let Ok(rd) = std::fs::read_dir(root) {
                let mut found: Vec<PathBuf> = rd
                    .filter_map(|e| e.ok())
                    .map(|e| e.path().join("bin").join(exe()))
                    .collect();
                found.sort();
                found.reverse();
                out.extend(found);
            }
        }
    }
    out.into_iter().filter(|p| p.is_file()).collect()
}

/// Best java for building: the app's own JDK first, then any system Java 17+.
pub fn find_java() -> Option<PathBuf> {
    if let Some(j) = bundled_java() {
        return Some(j);
    }
    let cands = system_java_candidates();
    cands
        .iter()
        .find(|p| java_version(p).map(|v| v >= 17).unwrap_or(false))
        .cloned()
        .or_else(|| cands.into_iter().next())
}

/// Has the Gradle distribution used by exported projects been downloaded already?
pub fn gradle_ready() -> bool {
    let home = std::env::var_os("GRADLE_USER_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(|h| PathBuf::from(h).join(".gradle"))
        });
    let Some(home) = home else { return false };
    let dist = home
        .join("wrapper/dists")
        .join(format!("gradle-{}-bin", crate::codegen::GRADLE_VERSION));
    std::fs::read_dir(dist)
        .map(|rd| {
            rd.filter_map(|e| e.ok()).any(|e| {
                e.path()
                    .join(format!("gradle-{}", crate::codegen::GRADLE_VERSION))
                    .join("bin")
                    .is_dir()
            })
        })
        .unwrap_or(false)
}

pub fn hide_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

/// Starts the setup in a background thread.
pub fn start_setup(force_jdk: bool, ctx: Option<egui::Context>) -> Receiver<SetupMsg> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let r = run_setup(force_jdk, &tx, ctx.as_ref());
        let _ = tx.send(SetupMsg::Done(r));
        if let Some(c) = &ctx {
            c.request_repaint();
        }
    });
    rx
}

fn say(tx: &Sender<SetupMsg>, ctx: Option<&egui::Context>, m: SetupMsg) {
    let _ = tx.send(m);
    if let Some(c) = ctx {
        c.request_repaint();
    }
}

/// Makes sure a JDK exists and Gradle is downloaded.
pub fn run_setup(
    force_jdk: bool,
    tx: &Sender<SetupMsg>,
    ctx: Option<&egui::Context>,
) -> Result<(), String> {
    let de = crate::i18n::is_german();
    let tr = |a: &str, b: &str| if de { a.to_string() } else { b.to_string() };

    // 1. Java
    let mut java = find_java();
    let usable = java
        .as_ref()
        .and_then(|j| java_version(j))
        .map(|v| v >= 17)
        .unwrap_or(false);
    if force_jdk || !usable {
        say(
            tx,
            ctx,
            SetupMsg::Status(tr(
                "Lade Java 25 (Eclipse Temurin) herunter…",
                "Downloading Java 25 (Eclipse Temurin)…",
            )),
        );
        download_jdk(tx, ctx)?;
        java = bundled_java();
    } else if let Some(j) = &java {
        say(
            tx,
            ctx,
            SetupMsg::Line(format!(
                "Java {} gefunden: {}",
                java_version(j).unwrap_or(0),
                j.display()
            )),
        );
    }
    let java = java.ok_or_else(|| {
        tr(
            "Java konnte nicht eingerichtet werden.",
            "Java could not be set up.",
        )
    })?;

    // 2. Gradle: export a tiny project and build it once. This downloads Gradle,
    //    the build JDK and the Paper API into the user's Gradle cache.
    say(
        tx,
        ctx,
        SetupMsg::Status(tr(
            "Bereite Gradle vor (einmalig, kann ein paar Minuten dauern)…",
            "Preparing Gradle (one time, may take a few minutes)…",
        )),
    );
    say(tx, ctx, SetupMsg::Progress(-1.0));
    let dir = tools_dir().join("warmup");
    let project = crate::examples::new_project(
        "Warmup",
        crate::model::Platform::Paper,
        crate::examples::Starter::Empty,
    );
    let out = crate::codegen::generate(&project);
    crate::codegen::write_to(&out, &dir, &project.settings.package).map_err(|e| e.to_string())?;
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("gradlew.bat");
        c
    } else {
        Command::new(dir.join("gradlew"))
    };
    cmd.args(["build", "--console=plain", "--no-daemon"])
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(home) = java.parent().and_then(|b| b.parent()) {
        cmd.env("JAVA_HOME", home);
    }
    hide_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| format!("Gradle: {e}"))?;
    let mut threads = vec![];
    for r in [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let tx = tx.clone();
        let c = ctx.cloned();
        threads.push(std::thread::spawn(move || {
            for line in BufReader::new(r).lines().map_while(Result::ok) {
                if !line.trim().is_empty() && !line.starts_with("Picked up JAVA_TOOL_OPTIONS") {
                    say(&tx, c.as_ref(), SetupMsg::Line(line));
                }
            }
        }));
    }
    for t in threads {
        let _ = t.join();
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(tr(
            "Gradle konnte nicht vorbereitet werden (Internetverbindung?). Du kannst es später erneut versuchen.",
            "Gradle could not be prepared (internet connection?). You can try again later.",
        ));
    }
    say(tx, ctx, SetupMsg::Progress(1.0));
    say(tx, ctx, SetupMsg::Status(tr("Alles bereit!", "All set!")));
    Ok(())
}

fn jdk_url() -> String {
    let os = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x64"
    };
    format!(
        "https://api.adoptium.net/v3/binary/latest/{JDK_VERSION}/ga/{os}/{arch}/jdk/hotspot/normal/eclipse"
    )
}

fn download_jdk(tx: &Sender<SetupMsg>, ctx: Option<&egui::Context>) -> Result<(), String> {
    let base = tools_dir();
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let archive = base.join(if cfg!(windows) {
        "jdk-download.zip"
    } else {
        "jdk-download.tar.gz"
    });

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .user_agent("minecraft-builder")
        .build()
        .into();
    let url = jdk_url();
    say(tx, ctx, SetupMsg::Line(url.clone()));
    let mut resp = agent
        .get(&url)
        .call()
        .map_err(|e| format!("Download fehlgeschlagen: {e}"))?;
    let total: u64 = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut reader = resp.body_mut().as_reader();
    let mut file = std::fs::File::create(&archive).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut done: u64 = 0;
    let mut last_report = 0u64;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("Download abgebrochen: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if done - last_report > 2 * 1024 * 1024 {
            last_report = done;
            if total > 0 {
                say(tx, ctx, SetupMsg::Progress(done as f32 / total as f32));
            }
            say(
                tx,
                ctx,
                SetupMsg::Status(format!(
                    "Java 25: {} / {} MB",
                    done / 1_048_576,
                    total / 1_048_576
                )),
            );
        }
    }
    drop(file);
    say(
        tx,
        ctx,
        SetupMsg::Status(crate::i18n::t("Entpacke Java…", "Unpacking Java…").into()),
    );
    say(tx, ctx, SetupMsg::Progress(-1.0));

    let tmp = base.join("jdk-unpack");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    if cfg!(windows) {
        let f = std::fs::File::open(&archive).map_err(|e| e.to_string())?;
        let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
        z.extract(&tmp).map_err(|e| e.to_string())?;
    } else {
        let f = std::fs::File::open(&archive).map_err(|e| e.to_string())?;
        let mut t = tar::Archive::new(flate2::read::GzDecoder::new(f));
        t.unpack(&tmp).map_err(|e| e.to_string())?;
    }
    // The archive contains a single folder like "jdk-25.0.1+8".
    let inner = std::fs::read_dir(&tmp)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .ok_or("JDK-Archiv ist leer")?;
    let target = base.join("jdk");
    let _ = std::fs::remove_dir_all(&target);
    std::fs::rename(&inner, &target).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_file(&archive);
    let java = bundled_java().ok_or("java wurde im JDK nicht gefunden")?;
    say(
        tx,
        ctx,
        SetupMsg::Line(format!("Java installiert: {}", java.display())),
    );
    Ok(())
}

/// `minecraft-builder --setup-tools`: used by the installer. Writes a log file.
pub fn setup_cli() -> i32 {
    let (tx, rx) = channel();
    let force = std::env::var_os("MCB_FORCE_JDK").is_some();
    let handle = std::thread::spawn(move || run_setup(force, &tx, None));
    let _ = std::fs::create_dir_all(tools_dir());
    let mut log = std::fs::File::create(tools_dir().join("setup.log")).ok();
    for m in rx {
        let line = match m {
            SetupMsg::Status(s) | SetupMsg::Line(s) => s,
            SetupMsg::Progress(_) => continue,
            SetupMsg::Done(_) => break,
        };
        println!("{line}");
        if let Some(f) = log.as_mut() {
            let _ = writeln!(f, "{line}");
        }
    }
    match handle.join() {
        Ok(Ok(())) => 0,
        Ok(Err(e)) => {
            eprintln!("{e}");
            if let Some(f) = log.as_mut() {
                let _ = writeln!(f, "ERROR {e}");
            }
            1
        }
        Err(_) => 1,
    }
}

/// Cached tool status for the UI (checking Java runs a process, so do it rarely).
#[derive(Clone)]
pub struct Status {
    pub java: Option<PathBuf>,
    pub java_version: Option<u32>,
    pub gradle: bool,
}

static STATUS: std::sync::Mutex<Option<(std::time::Instant, Status)>> = std::sync::Mutex::new(None);

pub fn status() -> Status {
    let mut g = STATUS.lock().unwrap();
    if let Some((t, s)) = g.as_ref() {
        if t.elapsed().as_secs() < 5 {
            return s.clone();
        }
    }
    let java = find_java();
    let s = Status {
        java_version: java.as_deref().and_then(java_version),
        java,
        gradle: gradle_ready(),
    };
    *g = Some((std::time::Instant::now(), s.clone()));
    s
}

pub fn invalidate_status() {
    *STATUS.lock().unwrap() = None;
}
