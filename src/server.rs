//! Game server panels: upload the built jar straight into `plugins/` or `mods/`.
//!
//! Supported panel: Calagopus (https://github.com/calagopus/panel). Its client API:
//! * `Authorization: Bearer c7sp_…` (API key from Account → API keys)
//! * `GET  /api/client/account`
//! * `GET  /api/client/servers`
//! * `GET  /api/client/servers/{uuid}/files/list?directory=/plugins`
//! * `POST /api/client/servers/{uuid}/files/create-directory` `{root, name}`
//! * `POST /api/client/servers/{uuid}/files/delete` `{root, files}`
//! * `GET  /api/client/servers/{uuid}/files/upload` → signed node URL, then
//!   multipart `POST <url>&directory=/plugins`
//! * `POST /api/client/servers/{uuid}/power` `{action}` and `/command` `{command}`

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AfterUpload {
    /// Only upload.
    #[default]
    Nothing,
    /// Restart the server.
    Restart,
    /// Run a console command (e.g. `reload confirm`).
    Command,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    pub id: u64,
    /// Name shown in the app.
    #[serde(default)]
    pub name: String,
    /// Base address of the panel, e.g. `https://panel.example.com`.
    pub url: String,
    #[serde(default)]
    pub api_key: String,
    /// Server on the panel the jar goes to.
    #[serde(default)]
    pub server_uuid: String,
    #[serde(default)]
    pub server_name: String,
    #[serde(default)]
    pub after: AfterUpload,
    #[serde(default)]
    pub command: String,
}

impl ServerConfig {
    pub fn display_name(&self) -> String {
        if !self.name.trim().is_empty() {
            self.name.trim().to_string()
        } else if !self.server_name.is_empty() {
            self.server_name.clone()
        } else {
            host_of(&self.url)
        }
    }
    pub fn ready(&self) -> bool {
        !self.url.is_empty() && looks_like_key(&self.api_key) && !self.server_uuid.is_empty()
    }
}

/// A server the API key can access.
#[derive(Clone, Debug, PartialEq)]
pub struct RemoteServer {
    pub uuid: String,
    pub name: String,
    pub description: String,
    pub node: String,
}

/// `panel.example.com/whatever` -> `https://panel.example.com`
pub fn normalize_url(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return String::new();
    }
    let (scheme, rest) = match s.split_once("://") {
        Some((sch, rest)) if sch.eq_ignore_ascii_case("http") => ("http", rest),
        Some((_, rest)) => ("https", rest),
        None => ("https", s),
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("").trim();
    if host.is_empty() {
        return String::new();
    }
    format!("{scheme}://{}", host.to_lowercase())
}

pub fn host_of(url: &str) -> String {
    url.split_once("://")
        .map(|x| x.1)
        .unwrap_or(url)
        .trim_end_matches('/')
        .to_string()
}

/// Page of the panel where a new API key is created.
pub fn api_key_page(url: &str) -> String {
    format!("{}/account/api-keys/create", normalize_url(url))
}

/// Calagopus keys: `c7sp_` followed by 43 letters/digits.
pub fn looks_like_key(s: &str) -> bool {
    let s = s.trim();
    s.len() == 48
        && s.starts_with("c7sp_")
        && s[5..].chars().all(|c| c.is_ascii_alphanumeric())
}

/// Finds an API key inside copied text (e.g. the clipboard).
pub fn find_key(text: &str) -> Option<String> {
    let mut rest = text;
    while let Some(i) = rest.find("c7sp_") {
        let cand: String = rest[i..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if looks_like_key(&cand) {
            return Some(cand);
        }
        rest = &rest[i + 5..];
    }
    None
}

fn agent(timeout_secs: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(timeout_secs)))
        .http_status_as_error(false)
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .user_agent("minecraft-builder")
        .build()
        .into()
}

fn explain(status: u16, body: &str) -> String {
    let detail = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v["errors"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .or_else(|| v["error"].as_str().map(str::to_string))
                .or_else(|| v["message"].as_str().map(str::to_string))
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    let hint = match status {
        401 => crate::i18n::t(
            "Der API-Key wurde nicht akzeptiert.",
            "The API key was not accepted.",
        ),
        403 => crate::i18n::t(
            "Keine Berechtigung – dem API-Key fehlen Rechte (Dateien / Server).",
            "No permission – the API key lacks rights (files / servers).",
        ),
        404 => crate::i18n::t(
            "Nicht gefunden – stimmt die Panel-Adresse und der Server?",
            "Not found – are the panel address and the server right?",
        ),
        _ => "",
    };
    let mut s = format!("HTTP {status}");
    if !hint.is_empty() {
        s.push_str(&format!(" · {hint}"));
    }
    if !detail.is_empty() {
        s.push_str(&format!(" ({detail})"));
    }
    s
}

fn net_err(e: ureq::Error) -> String {
    format!(
        "{}: {e}",
        crate::i18n::t("Panel nicht erreichbar", "Panel not reachable")
    )
}

fn get(url: &str, key: &str) -> Result<serde_json::Value, String> {
    let mut resp = agent(25)
        .get(url)
        .header("Authorization", &format!("Bearer {}", key.trim()))
        .header("Accept", "application/json")
        .call()
        .map_err(net_err)?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().read_to_string().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(explain(status, &body));
    }
    serde_json::from_str(&body).map_err(|_| {
        crate::i18n::t(
            "Unerwartete Antwort – ist das wirklich ein Calagopus-Panel?",
            "Unexpected answer – is this really a Calagopus panel?",
        )
        .to_string()
    })
}

fn post(url: &str, key: &str, json: &serde_json::Value) -> Result<(), String> {
    let mut resp = agent(40)
        .post(url)
        .header("Authorization", &format!("Bearer {}", key.trim()))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .send(json.to_string())
        .map_err(net_err)?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        let body = resp.body_mut().read_to_string().unwrap_or_default();
        return Err(explain(status, &body));
    }
    Ok(())
}

/// Checks address + key. Returns the account's user name.
pub fn check_account(url: &str, key: &str) -> Result<String, String> {
    let v = get(&format!("{}/api/client/account", normalize_url(url)), key)?;
    let u = &v["user"];
    Ok(u["username"]
        .as_str()
        .or_else(|| u["name"].as_str())
        .or_else(|| u["email"].as_str())
        .unwrap_or("?")
        .to_string())
}

pub fn list_servers(url: &str, key: &str) -> Result<Vec<RemoteServer>, String> {
    let base = normalize_url(url);
    let mut out = vec![];
    for page in 1..=10 {
        let v = get(
            &format!("{base}/api/client/servers?page={page}&per_page=50"),
            key,
        )?;
        let list = &v["servers"];
        let Some(data) = list["data"].as_array() else {
            break;
        };
        for s in data {
            let uuid = s["uuid"].as_str().unwrap_or_default().to_string();
            if uuid.is_empty() {
                continue;
            }
            out.push(RemoteServer {
                uuid,
                name: s["name"].as_str().unwrap_or("?").to_string(),
                description: s["description"].as_str().unwrap_or_default().to_string(),
                node: s["node_name"].as_str().unwrap_or_default().to_string(),
            });
        }
        let total = list["total"].as_i64().unwrap_or(0);
        if (out.len() as i64) >= total || data.is_empty() {
            break;
        }
    }
    Ok(out)
}

fn multipart(file_name: &str, data: &[u8]) -> (String, Vec<u8>) {
    let boundary = format!(
        "----mcbuilder{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(1)
    );
    let mut body = Vec::with_capacity(data.len() + 512);
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"{}\"\r\nContent-Type: application/java-archive\r\n\r\n",
            file_name.replace('"', "")
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

/// Uploads the jar into `/<folder>` of the server, removes older versions of the same
/// plugin/mod and runs the configured follow-up. Returns lines for the console.
pub fn deploy(
    cfg: &ServerConfig,
    jar: &Path,
    folder: &str,
    project_id: &str,
) -> Result<Vec<String>, String> {
    if !cfg.ready() {
        return Err(crate::i18n::t(
            "Der Server ist nicht fertig eingerichtet (Einstellungen → Server).",
            "The server is not set up completely (Settings → Servers).",
        )
        .into());
    }
    let base = format!(
        "{}/api/client/servers/{}",
        normalize_url(&cfg.url),
        cfg.server_uuid
    );
    let key = cfg.api_key.trim();
    let file_name = jar
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("jar?")?;
    let data = std::fs::read(jar).map_err(|e| e.to_string())?;
    let dir = format!("/{folder}");
    let mut log = vec![];

    // The folder may not exist yet (fresh server): an error here just means it does.
    let _ = post(
        &format!("{base}/files/create-directory"),
        key,
        &serde_json::json!({ "root": "/", "name": folder }),
    );

    // Older builds of the same project would be loaded twice.
    if let Ok(v) = get(
        &format!("{base}/files/list?directory={}", urlencode(&dir)),
        key,
    ) {
        let prefix = format!("{project_id}-");
        let old: Vec<String> = v["entries"]["data"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|e| e["name"].as_str())
                    .filter(|n| n.starts_with(&prefix) && n.ends_with(".jar") && *n != file_name)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if !old.is_empty()
            && post(
                &format!("{base}/files/delete"),
                key,
                &serde_json::json!({ "root": dir, "files": old }),
            )
            .is_ok()
        {
            log.push(format!("Alte Version entfernt: {}", old.join(", ")));
        }
    }

    let v = get(&format!("{base}/files/upload"), key)?;
    let upload = v["url"]
        .as_str()
        .ok_or("Das Panel hat keine Upload-Adresse geliefert.")?;
    let sep = if upload.contains('?') { '&' } else { '?' };
    let upload = format!("{upload}{sep}directory={}", urlencode(&dir));
    let (content_type, body) = multipart(&file_name, &data);
    let mut resp = agent(300)
        .post(&upload)
        .header("Content-Type", &content_type)
        .header("Accept", "application/json")
        .send(&body[..])
        .map_err(net_err)?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        let text = resp.body_mut().read_to_string().unwrap_or_default();
        return Err(explain(status, &text));
    }
    log.push(format!(
        "Hochgeladen: {dir}/{file_name} → {} ({} KB)",
        cfg.display_name(),
        (data.len() / 1024).max(1)
    ));

    match cfg.after {
        AfterUpload::Nothing => {}
        AfterUpload::Restart => {
            match post(
                &format!("{base}/power"),
                key,
                &serde_json::json!({ "action": "restart" }),
            ) {
                Ok(()) => log.push("Server wird neu gestartet.".into()),
                Err(e) => log.push(format!("Neustart nicht möglich: {e}")),
            }
        }
        AfterUpload::Command => {
            let c = cfg.command.trim().trim_start_matches('/');
            if !c.is_empty() {
                match post(
                    &format!("{base}/command"),
                    key,
                    &serde_json::json!({ "command": c }),
                ) {
                    Ok(()) => log.push(format!("Befehl gesendet: {c}")),
                    Err(e) => log.push(format!("Befehl nicht gesendet: {e}")),
                }
            }
        }
    }
    Ok(log)
}

fn server_base(cfg: &ServerConfig) -> String {
    format!(
        "{}/api/client/servers/{}",
        normalize_url(&cfg.url),
        cfg.server_uuid
    )
}

/// The last lines of the server console.
pub fn fetch_logs(cfg: &ServerConfig, lines: u32) -> Result<String, String> {
    let mut resp = agent(20)
        .get(&format!("{}/logs?lines={}", server_base(cfg), lines.clamp(1, 1000)))
        .header("Authorization", &format!("Bearer {}", cfg.api_key.trim()))
        .call()
        .map_err(net_err)?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().read_to_string().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(explain(status, &body));
    }
    Ok(body)
}

/// Sends a console command to the server.
pub fn send_command(cfg: &ServerConfig, command: &str) -> Result<(), String> {
    post(
        &format!("{}/command", server_base(cfg)),
        cfg.api_key.trim(),
        &serde_json::json!({ "command": command.trim().trim_start_matches('/') }),
    )
}

/// start / stop / restart / kill
pub fn power(cfg: &ServerConfig, action: &str) -> Result<(), String> {
    post(
        &format!("{}/power", server_base(cfg)),
        cfg.api_key.trim(),
        &serde_json::json!({ "action": action }),
    )
}

/// Removes ANSI color sequences from console output.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if c != '\r' {
            out.push(c);
        }
    }
    out
}

fn urlencode(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_normalized() {
        assert_eq!(normalize_url("panel.example.com"), "https://panel.example.com");
        assert_eq!(
            normalize_url(" https://Panel.Example.com/server/abc?x=1 "),
            "https://panel.example.com"
        );
        assert_eq!(normalize_url("http://10.0.0.5:8000/"), "http://10.0.0.5:8000");
        assert_eq!(normalize_url(""), "");
        assert_eq!(
            api_key_page("panel.example.com"),
            "https://panel.example.com/account/api-keys/create"
        );
    }

    #[test]
    fn keys_are_recognized() {
        let key = format!("c7sp_{}", "aB3".repeat(14) + "x");
        assert!(looks_like_key(&key));
        assert!(!looks_like_key("c7sp_short"));
        assert!(!looks_like_key(&key.replace("c7sp_", "ptlc_")));
        assert_eq!(find_key(&format!("Your key: {key}\n")), Some(key.clone()));
        assert_eq!(find_key("nothing here c7sp_ nope"), None);
    }

    #[test]
    fn multipart_body_is_well_formed() {
        let (ct, body) = multipart("x-1.0.jar", b"DATA");
        let boundary = ct.split("boundary=").nth(1).unwrap();
        let text = String::from_utf8(body).unwrap();
        assert!(text.starts_with(&format!("--{boundary}\r\n")));
        assert!(text.contains("name=\"files\"; filename=\"x-1.0.jar\""));
        assert!(text.contains("\r\n\r\nDATA\r\n"));
        assert!(text.ends_with(&format!("--{boundary}--\r\n")));
    }

    /// A tiny stand-in for the panel + node: answers the calls the client makes and records them.
    fn mock_panel(requests: usize) -> (String, std::thread::JoinHandle<Vec<(String, String, Vec<u8>)>>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let base2 = base.clone();
        let handle = std::thread::spawn(move || {
            let mut seen = vec![];
            for _ in 0..requests {
                let (mut sock, _) = listener.accept().unwrap();
                let mut buf = Vec::new();
                let mut tmp = [0u8; 4096];
                let (head_end, len) = loop {
                    let n = sock.read(&mut tmp).unwrap();
                    buf.extend_from_slice(&tmp[..n]);
                    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..i]).to_lowercase();
                        let len = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        break (i + 4, len);
                    }
                    assert!(n > 0);
                };
                while buf.len() < head_end + len {
                    let n = sock.read(&mut tmp).unwrap();
                    buf.extend_from_slice(&tmp[..n]);
                }
                let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
                let first = head.lines().next().unwrap_or("").to_string();
                let path = first.split(' ').nth(1).unwrap_or("").to_string();
                let auth = head
                    .lines()
                    .find(|l| l.to_lowercase().starts_with("authorization:"))
                    .unwrap_or("")
                    .to_string();
                let body = buf[head_end..].to_vec();
                let (status, json) = if path.starts_with("/upload/file") {
                    (200, "{}".to_string())
                } else if !auth.contains("Bearer c7sp_") {
                    (401, r#"{"errors":["invalid api key"]}"#.to_string())
                } else if path == "/api/client/account" {
                    (200, r#"{"user":{"username":"steve"}}"#.to_string())
                } else if path.starts_with("/api/client/servers?") {
                    (200, r#"{"servers":{"total":1,"per_page":50,"page":1,"data":[{"uuid":"1111-2222","uuid_short":"1111","name":"Survival","description":null,"node_name":"node1"}]}}"#.to_string())
                } else if path.contains("/logs?lines=") {
                    (200, "[12:00:00 INFO]: \u{1b}[32mDone\u{1b}[0m (1.2s)!\r\n[12:00:01 WARN]: x".to_string())
                } else if path.contains("/files/list") {
                    (200, r#"{"entries":{"total":2,"data":[{"name":"meinplugin-0.9.jar"},{"name":"anderes.jar"}]}}"#.to_string())
                } else if path.ends_with("/files/upload") {
                    (200, format!(r#"{{"url":"{base2}/upload/file?token=abc"}}"#))
                } else {
                    (200, "{}".to_string())
                };
                let resp = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
                    json.len()
                );
                sock.write_all(resp.as_bytes()).unwrap();
                seen.push((first, auth, body));
            }
            seen
        });
        (base, handle)
    }

    #[test]
    fn connects_and_lists_servers() {
        let key = format!("c7sp_{}", "k".repeat(43));
        let (base, h) = mock_panel(2);
        assert_eq!(check_account(&base, &key).unwrap(), "steve");
        let servers = list_servers(&base, &key).unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "Survival");
        assert_eq!(servers[0].uuid, "1111-2222");
        let seen = h.join().unwrap();
        assert!(seen[0].1.ends_with(&key));

        let (base, h) = mock_panel(1);
        let err = check_account(&base, "falsch").unwrap_err();
        assert!(err.contains("401"), "{err}");
        h.join().unwrap();
    }

    #[test]
    fn reads_the_console_and_sends_commands() {
        let (base, h) = mock_panel(3);
        let cfg = ServerConfig {
            id: 1,
            url: base,
            api_key: format!("c7sp_{}", "k".repeat(43)),
            server_uuid: "1111-2222".into(),
            ..Default::default()
        };
        let log = strip_ansi(&fetch_logs(&cfg, 200).unwrap());
        assert_eq!(log, "[12:00:00 INFO]: Done (1.2s)!\n[12:00:01 WARN]: x");
        send_command(&cfg, "/say hi").unwrap();
        power(&cfg, "restart").unwrap();
        let seen = h.join().unwrap();
        assert!(seen[0].0.starts_with("GET /api/client/servers/1111-2222/logs?lines=200"));
        assert!(String::from_utf8_lossy(&seen[1].2).contains("\"command\":\"say hi\""));
        assert!(String::from_utf8_lossy(&seen[2].2).contains("restart"));
    }

    #[test]
    fn uploads_the_jar_and_cleans_up_old_versions() {
        let dir = std::env::temp_dir().join(format!("mcb-deploy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let jar = dir.join("meinplugin-1.0.0.jar");
        std::fs::write(&jar, b"JARDATA").unwrap();
        let (base, h) = mock_panel(6);
        let cfg = ServerConfig {
            id: 1,
            url: base,
            api_key: format!("c7sp_{}", "k".repeat(43)),
            server_uuid: "1111-2222".into(),
            server_name: "Survival".into(),
            after: AfterUpload::Restart,
            ..Default::default()
        };
        let log = deploy(&cfg, &jar, "plugins", "meinplugin").unwrap();
        let seen = h.join().unwrap();
        let lines: Vec<&str> = seen.iter().map(|s| s.0.as_str()).collect();
        assert!(lines[0].starts_with("POST /api/client/servers/1111-2222/files/create-directory"));
        assert!(String::from_utf8_lossy(&seen[0].2).contains("\"name\":\"plugins\""));
        assert!(lines[1].starts_with("GET /api/client/servers/1111-2222/files/list?directory=%2Fplugins"));
        assert!(lines[2].starts_with("POST /api/client/servers/1111-2222/files/delete"));
        let del = String::from_utf8_lossy(&seen[2].2).to_string();
        assert!(del.contains("meinplugin-0.9.jar") && !del.contains("anderes.jar"), "{del}");
        assert!(lines[3].starts_with("GET /api/client/servers/1111-2222/files/upload"));
        assert!(lines[4].starts_with("POST /upload/file?token=abc&directory=%2Fplugins"));
        let up = String::from_utf8_lossy(&seen[4].2).to_string();
        assert!(up.contains("filename=\"meinplugin-1.0.0.jar\"") && up.contains("JARDATA"));
        assert!(lines[5].starts_with("POST /api/client/servers/1111-2222/power"));
        assert!(String::from_utf8_lossy(&seen[5].2).contains("restart"));
        assert_eq!(log.len(), 3, "{log:?}");
    }
}
