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
    /// Id of the server (app settings) the jar is uploaded to after a build (0 = none).
    #[serde(default)]
    pub deploy_server: u64,
    /// Plugin/mod tells OPs in chat which events run (for finding mistakes).
    #[serde(default)]
    pub debug: bool,
    /// Delete the plugin's data folder (plugins/<Name>/ or config/<id>/) when the new jar
    /// is uploaded or copied. Resets all saved data (homes, bans, money, …).
    #[serde(default)]
    pub wipe_data_on_deploy: bool,
}

/// One resource pack .zip of the project. Several packs are merged into one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackEntry {
    /// Path of the .zip, relative to the project file if possible.
    pub path: String,
    #[serde(default = "yes")]
    pub enabled: bool,
}

/// What happens with one item of the resource pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ItemMode {
    /// Keep the pack as it is: custom models stay custom, vanilla replacements stay replacements.
    #[default]
    Auto,
    /// The vanilla item looks like this everywhere.
    Replace,
    /// Only obtainable through blocks / the give command; vanilla items stay untouched.
    CommandOnly,
    /// Not exported at all.
    Off,
}

impl ItemMode {
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            ItemMode::Auto => ("Wie im Pack", "As in the pack"),
            ItemMode::Replace => ("Ersetzt Vanilla-Item", "Replaces vanilla item"),
            ItemMode::CommandOnly => ("Nur per Befehl / Block", "Command / block only"),
            ItemMode::Off => ("Aus (nicht einbauen)", "Off (leave out)"),
        }
    }
}

/// Per-item settings of the resource pack, keyed by item model id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ItemRule {
    #[serde(default)]
    pub mode: ItemMode,
    /// Vanilla item that carries / gets replaced by the model, e.g. `minecraft:stick`.
    #[serde(default)]
    pub base: String,
    /// Display name used by the give command (with & color codes).
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub lore: String,
}

impl ItemRule {
    pub fn is_default(&self) -> bool {
        *self == ItemRule::default()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackSettings {
    /// Old projects: path of the single resource pack. Moved into `packs` on load.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    /// Resource packs, highest priority first. They are merged into one pack.
    #[serde(default)]
    pub packs: Vec<PackEntry>,
    /// Paper: port of the small web server that hands the pack to players.
    pub port: u16,
    /// Paper: players must accept the pack or get kicked.
    pub required: bool,
    /// Paper: send the pack automatically when a player joins.
    pub send_on_join: bool,
    /// Paper: text shown in the download prompt.
    pub prompt: String,
    /// Per-item settings (replace vanilla / command only / off).
    #[serde(default)]
    pub items: BTreeMap<String, ItemRule>,
    /// Name of the generated command that hands out the custom items (empty = none).
    #[serde(default)]
    pub give_command: String,
}

impl Default for PackSettings {
    fn default() -> Self {
        Self {
            path: String::new(),
            packs: vec![],
            port: 8164,
            required: false,
            send_on_join: true,
            prompt: "&aDieser Server nutzt eigene Texturen.".into(),
            items: BTreeMap::new(),
            give_command: String::new(),
        }
    }
}

impl PackSettings {
    pub fn enabled(&self) -> bool {
        self.packs
            .iter()
            .any(|p| p.enabled && !p.path.trim().is_empty())
    }
    /// Old single-pack projects -> pack list.
    pub fn migrate(&mut self) {
        if !self.path.trim().is_empty() {
            let path = std::mem::take(&mut self.path);
            if !self.packs.iter().any(|p| p.path == path) {
                self.packs.insert(
                    0,
                    PackEntry {
                        path,
                        enabled: true,
                    },
                );
            }
        }
    }
    pub fn resolve_one(path: &str, project_file: Option<&std::path::Path>) -> std::path::PathBuf {
        let p = std::path::PathBuf::from(path.trim());
        if p.is_absolute() {
            return p;
        }
        match project_file.and_then(|f| f.parent()) {
            Some(dir) => dir.join(p),
            None => p,
        }
    }
    /// Absolute paths of the enabled packs (highest priority first).
    pub fn resolve(&self, project_file: Option<&std::path::Path>) -> Vec<std::path::PathBuf> {
        self.packs
            .iter()
            .filter(|p| p.enabled && !p.path.trim().is_empty())
            .map(|p| Self::resolve_one(&p.path, project_file))
            .collect()
    }
    pub fn rule(&self, id: &str) -> ItemRule {
        self.items.get(id).cloned().unwrap_or_default()
    }
    pub fn set_rule(&mut self, id: &str, rule: ItemRule) {
        if rule.is_default() {
            self.items.remove(id);
        } else {
            self.items.insert(id.to_string(), rule);
        }
    }
    /// Cleaned name of the give command, if enabled.
    pub fn give_cmd(&self) -> Option<String> {
        let c: String = self
            .give_command
            .trim()
            .trim_start_matches('/')
            .to_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if c.is_empty() || !self.enabled() {
            None
        } else {
            Some(c)
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
            deploy_server: 0,
            debug: false,
            wipe_data_on_deploy: false,
        }
    }
}

impl ProjectSettings {
    /// Data folder of the plugin/mod on a server, relative to the server root:
    /// Paper `plugins/<name>` (from plugin.yml), Fabric `config/<id>`.
    pub fn data_folder(&self) -> (&'static str, String) {
        match self.platform {
            Platform::Paper => ("plugins", self.name.replace(' ', "_")),
            Platform::Fabric => ("config", self.id.clone()),
        }
    }

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
    /// Chat commands; each one becomes an event block "Command · /name".
    #[serde(default)]
    pub commands: Vec<CommandDef>,
    /// Settings that end up in the plugin's config.yml (server owners can change them).
    #[serde(default)]
    pub config: Vec<ConfigEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConfigType {
    #[default]
    Text,
    Number,
    Bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigEntry {
    pub key: String,
    #[serde(default)]
    pub ty: ConfigType,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CmdAccess {
    #[default]
    Everyone,
    Op,
    Permission,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CmdSenders {
    #[default]
    Players,
    Everyone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OptType {
    /// One word.
    #[default]
    Word,
    /// The rest of the message (only as last option).
    Text,
    Number,
    Integer,
    Player,
    Bool,
    /// One of a fixed list of words.
    Choice,
}

impl OptType {
    pub const ALL: [OptType; 7] = [
        OptType::Word,
        OptType::Text,
        OptType::Number,
        OptType::Integer,
        OptType::Player,
        OptType::Bool,
        OptType::Choice,
    ];
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            OptType::Word => ("Wort", "Word"),
            OptType::Text => ("Text (Rest)", "Text (rest)"),
            OptType::Number => ("Zahl", "Number"),
            OptType::Integer => ("Ganze Zahl", "Whole number"),
            OptType::Player => ("Spieler", "Player"),
            OptType::Bool => ("Ja/Nein", "Yes/No"),
            OptType::Choice => ("Auswahl", "Choice"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CmdOption {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub ty: OptType,
    #[serde(default = "yes")]
    pub required: bool,
    /// Choice values / tab suggestions.
    #[serde(default)]
    pub choices: Vec<String>,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandDef {
    pub id: NodeId,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub access: CmdAccess,
    #[serde(default)]
    pub permission: String,
    #[serde(default)]
    pub senders: CmdSenders,
    #[serde(default)]
    pub options: Vec<CmdOption>,
}

impl CommandDef {
    pub fn new(id: NodeId, name: &str) -> Self {
        Self {
            id,
            name: name.into(),
            description: String::new(),
            aliases: vec![],
            access: CmdAccess::Everyone,
            permission: String::new(),
            senders: CmdSenders::Players,
            options: vec![],
        }
    }
    /// How players type it, e.g. `/heal <spieler> [menge]`.
    pub fn usage(&self) -> String {
        let mut u = format!("/{}", self.name);
        for o in &self.options {
            if o.required {
                u.push_str(&format!(" <{}>", o.name));
            } else {
                u.push_str(&format!(" [{}]", o.name));
            }
        }
        u
    }
}

/// Output pin id of a command option.
pub fn opt_pin(index: usize) -> String {
    format!("opt{index}")
}

impl Default for Project {
    fn default() -> Self {
        Self {
            format: 1,
            settings: ProjectSettings::default(),
            workspaces: vec![Workspace::new("Start")],
            next_id: 1,
            commands: vec![],
            config: vec![],
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
        p.next_id = p
            .next_id
            .max(p.commands.iter().map(|c| c.id + 1).max().unwrap_or(0));
        p.migrate_commands();
        p.settings.pack.migrate();
        Ok(p)
    }

    pub fn command(&self, id: NodeId) -> Option<&CommandDef> {
        self.commands.iter().find(|c| c.id == id)
    }

    /// Command of a command event node.
    pub fn node_command(&self, node: &Node) -> Option<&CommandDef> {
        node.values
            .get("cmd")
            .and_then(|v| v.parse().ok())
            .and_then(|id| self.command(id))
    }

    /// Old projects stored commands in the block's fields: turn them into command definitions.
    pub fn migrate_commands(&mut self) {
        let mut new_cmds = vec![];
        let mut next = self.next_id;
        for ws in &mut self.workspaces {
            let ws_links = ws.links.clone();
            let ws = &mut *ws;
            for n in ws
                .nodes
                .iter_mut()
                .filter(|n| n.kind == "ev_command" && !n.values.contains_key("cmd"))
            {
                let get = |k: &str| n.values.get(k).cloned().unwrap_or_default();
                let id = next;
                next += 1;
                let name: String = get("name").trim().trim_start_matches('/').to_lowercase();
                let mut c = CommandDef::new(id, if name.is_empty() { "hallo" } else { &name });
                c.description = get("description");
                c.aliases = get("aliases")
                    .split([',', ' '])
                    .filter(|a| !a.trim().is_empty())
                    .map(|a| a.trim().into())
                    .collect();
                if !get("permission").trim().is_empty() {
                    c.access = CmdAccess::Permission;
                    c.permission = get("permission").trim().into();
                } else if get("op") == "true" {
                    c.access = CmdAccess::Op;
                }
                if get("players_only") == "false" {
                    c.senders = CmdSenders::Everyone;
                }
                let min: usize = get("min_args").parse::<f64>().unwrap_or(0.0) as usize;
                let used: Vec<usize> = ws_links
                    .iter()
                    .filter(|l| l.from.node == n.id)
                    .filter_map(|l| match l.from.pin.as_str() {
                        "arg1" => Some(1),
                        "arg2" => Some(2),
                        "arg3" => Some(3),
                        _ => None,
                    })
                    .collect();
                let count = used.into_iter().max().unwrap_or(0).max(min).min(3);
                for i in 0..count {
                    c.options.push(CmdOption {
                        name: format!("argument{}", i + 1),
                        description: String::new(),
                        ty: OptType::Word,
                        required: i < min,
                        choices: if i == 0 {
                            get("suggest")
                                .split(',')
                                .map(|x| x.trim().to_string())
                                .filter(|x| !x.is_empty())
                                .collect()
                        } else {
                            vec![]
                        },
                    });
                }
                n.values.clear();
                n.values.insert("cmd".into(), id.to_string());
                new_cmds.push(c);
            }
        }
        // Old pin names -> option pins.
        for ws in &mut self.workspaces {
            let cmd_nodes: Vec<NodeId> = ws
                .nodes
                .iter()
                .filter(|n| n.kind == "ev_command")
                .map(|n| n.id)
                .collect();
            for l in &mut ws.links {
                if cmd_nodes.contains(&l.from.node) {
                    match l.from.pin.as_str() {
                        "arg1" => l.from.pin = opt_pin(0),
                        "arg2" => l.from.pin = opt_pin(1),
                        "arg3" => l.from.pin = opt_pin(2),
                        _ => {}
                    }
                }
            }
        }
        self.next_id = next;
        self.commands.extend(new_cmds);
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
