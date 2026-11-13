//! Project data model: everything that gets saved into a `.mcb` project file.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type NodeId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Platform {
    /// Server plugin for Paper (also runs on Purpur, Folia excluded).
    #[default]
    Paper,
    /// Mod for the Fabric loader.
    Fabric,
}

impl Platform {
    pub fn label(self) -> &'static str {
        match self {
            Platform::Paper => "Paper Plugin",
            Platform::Fabric => "Fabric Mod",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            Platform::Paper => "Paper",
            Platform::Fabric => "Fabric",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Versions {
    pub minecraft: String,
    /// Paper: paper-api version. Fabric: fabric-api version.
    pub api: String,
    /// Fabric only: loader version.
    #[serde(default)]
    pub loader: String,
    /// Fabric only: loom plugin version.
    #[serde(default)]
    pub loom: String,
    pub java: u32,
}

impl Versions {
    pub fn default_for(p: Platform) -> Self {
        match p {
            Platform::Paper => Versions {
                minecraft: "26.3".into(),
                api: "26.3.build.249-stable".into(),
                loader: String::new(),
                loom: String::new(),
                java: 25,
            },
            Platform::Fabric => Versions {
                minecraft: "26.3".into(),
                api: "0.166.2+26.3".into(),
                loader: "0.20.1".into(),
                loom: "1.19-SNAPSHOT".into(),
                java: 25,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub name: String,
    /// Lowercase id: plugin name / mod id.
    pub id: String,
    pub package: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub platform: Platform,
    pub versions: Versions,
    /// Optional folder of a test server (plugins/ or mods/) the jar gets copied into.
    #[serde(default)]
    pub deploy_dir: String,
    /// Optional resource pack (textures/models) bundled into the plugin/mod.
    #[serde(default)]
    pub pack: PackSettings,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackSettings {
    /// Path of the resource pack .zip, relative to the project file if possible.
    pub path: String,
    /// Paper: port of the small web server that hands the pack to players.
    pub port: u16,
    /// Paper: players must accept the pack or get kicked.
    pub required: bool,
    /// Paper: send the pack automatically when a player joins.
    pub send_on_join: bool,
    /// Paper: text shown in the download prompt.
    pub prompt: String,
}

impl Default for PackSettings {
    fn default() -> Self {
        Self {
            path: String::new(),
            port: 8164,
            required: false,
            send_on_join: true,
            prompt: "&aDieser Server nutzt eigene Texturen.".into(),
        }
    }
}

impl PackSettings {
    pub fn enabled(&self) -> bool {
        !self.path.trim().is_empty()
    }
    /// Absolute path, resolving relative paths against the project file's folder.
    pub fn resolve(&self, project_file: Option<&std::path::Path>) -> std::path::PathBuf {
        let p = std::path::PathBuf::from(self.path.trim());
        if p.is_absolute() {
            return p;
        }
        match project_file.and_then(|f| f.parent()) {
            Some(dir) => dir.join(p),
            None => p,
        }
    }
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            name: "MeinPlugin".into(),
            id: "meinplugin".into(),
            package: "de.example.meinplugin".into(),
            version: "1.0.0".into(),
            author: "Ich".into(),
            description: "Gebaut mit Minecraft Builder".into(),
            platform: Platform::Paper,
            versions: Versions::default_for(Platform::Paper),
            deploy_dir: String::new(),
            pack: PackSettings::default(),
        }
    }
}

impl ProjectSettings {
    /// Java class name of the main class.
    pub fn main_class(&self) -> String {
        let mut s = String::new();
        let mut up = true;
        for c in self.name.chars() {
            if c.is_ascii_alphanumeric() {
                if up {
                    s.extend(c.to_uppercase());
                    up = false;
                } else {
                    s.push(c);
                }
            } else {
                up = true;
            }
        }
        if s.is_empty() || s.chars().next().unwrap().is_ascii_digit() {
            s.insert_str(0, "Main");
        }
        s
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    /// Block definition id, see `blocks.rs`.
    pub kind: String,
    pub pos: [f32; 2],
    /// Values of fields and of unconnected input pins, keyed by field/pin id.
    #[serde(default)]
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PinRef {
    pub node: NodeId,
    pub pin: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Link {
    pub from: PinRef,
    pub to: PinRef,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: NodeId,
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub title: String,
    pub text: String,
    /// Index into the note color palette.
    #[serde(default)]
    pub color: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
    #[serde(default)]
    pub notes: Vec<Note>,
}

impl Workspace {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            nodes: vec![],
            links: vec![],
            notes: vec![],
        }
    }
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }
    pub fn link_into(&self, node: NodeId, pin: &str) -> Option<&Link> {
        self.links
            .iter()
            .find(|l| l.to.node == node && l.to.pin == pin)
    }
    pub fn links_from<'a>(
        &'a self,
        node: NodeId,
        pin: &'a str,
    ) -> impl Iterator<Item = &'a Link> + 'a {
        self.links
            .iter()
            .filter(move |l| l.from.node == node && l.from.pin == pin)
    }
    pub fn remove_node(&mut self, id: NodeId) {
        self.nodes.retain(|n| n.id != id);
        self.links.retain(|l| l.from.node != id && l.to.node != id);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub format: u32,
    pub settings: ProjectSettings,
    pub workspaces: Vec<Workspace>,
    pub next_id: NodeId,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            format: 1,
            settings: ProjectSettings::default(),
            workspaces: vec![Workspace::new("Start")],
            next_id: 1,
        }
    }
}

impl Project {
    pub fn alloc_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut p: Project = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        // Repair next_id in case the file was edited by hand.
        let max = p
            .workspaces
            .iter()
            .flat_map(|w| {
                w.nodes
                    .iter()
                    .map(|n| n.id)
                    .chain(w.notes.iter().map(|n| n.id))
            })
            .max()
            .unwrap_or(0);
        p.next_id = p.next_id.max(max + 1);
        if p.workspaces.is_empty() {
            p.workspaces.push(Workspace::new("Start"));
        }
        Ok(p)
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| e.to_string())
    }
}

/// Turns any display name into a valid lowercase identifier.
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if c == '_' || c == '-' || c == ' ' {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "projekt".into()
    } else {
        out
    }
}
