//! Starter projects for "New project" and a self-test project that uses every block.

use crate::blocks::{self, Kind, PinType};
use crate::model::{
    Link, Node, NodeId, PinRef, Platform, Project, ProjectSettings, Versions, Workspace,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Starter {
    Empty,
    Welcome,
    Counter,
    Kit,
}

impl Starter {
    pub const ALL: [Starter; 4] = [
        Starter::Empty,
        Starter::Welcome,
        Starter::Counter,
        Starter::Kit,
    ];
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            Starter::Empty => ("Leeres Projekt", "Empty project"),
            Starter::Welcome => ("Willkommensnachricht", "Welcome message"),
            Starter::Counter => (
                "/zaehlen-Befehl mit Speicher",
                "/count command with storage",
            ),
            Starter::Kit => ("/kit und /heal Befehle", "/kit and /heal commands"),
        }
    }
    pub fn desc(self) -> (&'static str, &'static str) {
        match self {
            Starter::Empty => ("Nur ein „Beim Start“-Block.", "Just an “On start” block."),
            Starter::Welcome => (
                "Begrüßt Spieler beim Betreten mit Nachricht, Titel und Sound.",
                "Greets players on join with a message, title and sound.",
            ),
            Starter::Counter => (
                "Zählt pro Spieler, wie oft der Befehl benutzt wurde – bleibt nach Neustart erhalten.",
                "Counts per player how often the command was used – survives restarts.",
            ),
            Starter::Kit => (
                "Ein Starter-Kit-Befehl und ein Heil-Befehl nur für OPs.",
                "A starter kit command and an OP-only heal command.",
            ),
        }
    }
}

struct Builder<'a> {
    p: &'a mut Project,
    ws: usize,
}

impl<'a> Builder<'a> {
    fn node(&mut self, kind: &str, x: f32, y: f32, values: &[(&str, &str)]) -> NodeId {
        let id = self.p.alloc_id();
        let mut v = BTreeMap::new();
        for (k, val) in values {
            v.insert(k.to_string(), val.to_string());
        }
        self.p.workspaces[self.ws].nodes.push(Node {
            id,
            kind: kind.into(),
            pos: [x, y],
            values: v,
        });
        id
    }
    fn link(&mut self, from: NodeId, fpin: &str, to: NodeId, tpin: &str) {
        self.p.workspaces[self.ws].links.push(Link {
            from: PinRef {
                node: from,
                pin: fpin.into(),
            },
            to: PinRef {
                node: to,
                pin: tpin.into(),
            },
        });
    }
    fn flow(&mut self, from: NodeId, to: NodeId) {
        self.link(from, "next", to, "in");
    }
}

pub fn new_project(name: &str, platform: Platform, starter: Starter) -> Project {
    let id = crate::model::slug(name);
    let mut p = Project {
        settings: ProjectSettings {
            name: name.to_string(),
            id: id.clone(),
            package: format!("de.mcbuilder.{}", id),
            platform,
            versions: Versions::default_for(platform),
            ..ProjectSettings::default()
        },
        ..Project::default()
    };
    p.workspaces = vec![Workspace::new(crate::i18n::t("Start", "Start"))];
    let mut b = Builder { p: &mut p, ws: 0 };
    match starter {
        Starter::Empty => {
            let s = b.node("ev_start", 32.0, 32.0, &[]);
            let l = b.node("log", 336.0, 32.0, &[("text", "Plugin wurde gestartet!")]);
            b.flow(s, l);
        }
        Starter::Welcome => {
            let ev = b.node("ev_join", 32.0, 32.0, &[]);
            let name = b.node("player_name", 32.0, 208.0, &[]);
            let fmt = b.node(
                "format",
                336.0,
                208.0,
                &[("format", "&aWillkommen &e{0}&a auf dem Server!")],
            );
            let msg = b.node("send_message", 336.0, 32.0, &[]);
            let title = b.node(
                "title",
                656.0,
                32.0,
                &[("title", "&6Willkommen!"), ("subtitle", "&7Viel Spaß")],
            );
            let snd = b.node("sound", 976.0, 32.0, &[("sound", "entity.player.levelup")]);
            b.link(ev, "player", name, "player");
            b.link(name, "name", fmt, "p0");
            b.link(fmt, "res", msg, "text");
            b.flow(ev, msg);
            b.flow(msg, title);
            b.flow(title, snd);
            let q = b.node("ev_quit", 32.0, 544.0, &[]);
            let qn = b.node("player_name", 32.0, 720.0, &[]);
            let qf = b.node(
                "format",
                336.0,
                720.0,
                &[("format", "&7{0} hat den Server verlassen.")],
            );
            let bc = b.node("broadcast", 336.0, 544.0, &[]);
            b.link(q, "player", qn, "player");
            b.link(qn, "name", qf, "p0");
            b.link(qf, "res", bc, "text");
            b.flow(q, bc);
            let id = b.p.alloc_id();
            b.p.workspaces[0].notes.push(crate::model::Note {
                id,
                pos: [8.0, -48.0],
                size: [1260.0, 540.0],
                title: "Begrüßung".into(),
                text: "Neue Spieler bekommen eine Nachricht, einen Titel und einen Sound.".into(),
                color: 0,
            });
        }
        Starter::Counter => {
            let ev = b.node(
                "ev_command",
                32.0,
                32.0,
                &[("name", "zaehlen"), ("description", "Zählt mit")],
            );
            let add = b.node("data_add", 336.0, 32.0, &[("key", "zaehler")]);
            let fmt = b.node(
                "format",
                656.0,
                240.0,
                &[("format", "&aDu hast schon &e{0}&a-mal gezählt!")],
            );
            let msg = b.node("send_message", 656.0, 32.0, &[]);
            b.flow(ev, add);
            b.flow(add, msg);
            b.link(add, "value", fmt, "p0");
            b.link(fmt, "res", msg, "text");
        }
        Starter::Kit => {
            let ev = b.node(
                "ev_command",
                32.0,
                32.0,
                &[("name", "kit"), ("description", "Starter-Kit")],
            );
            let g1 = b.node(
                "give_item",
                336.0,
                32.0,
                &[("item", "minecraft:stone_sword")],
            );
            let g2 = b.node(
                "give_item",
                656.0,
                32.0,
                &[("item", "minecraft:bread"), ("amount", "16")],
            );
            let m = b.node(
                "send_message",
                976.0,
                32.0,
                &[("text", "&aDu hast dein Kit bekommen!")],
            );
            b.flow(ev, g1);
            b.flow(g1, g2);
            b.flow(g2, m);
            let h = b.node(
                "ev_command",
                32.0,
                400.0,
                &[
                    ("name", "heal"),
                    ("description", "Heilt dich"),
                    ("op", "true"),
                ],
            );
            let heal = b.node("heal", 336.0, 400.0, &[]);
            let m2 = b.node(
                "send_message",
                656.0,
                400.0,
                &[("text", "&aDu wurdest geheilt.")],
            );
            b.flow(h, heal);
            b.flow(heal, m2);
        }
    }
    p.migrate_commands();
    p
}

/// A project that uses every block that exists for `platform`. Used to test the code generator.
/// A minimal resource pack with one item model, for tests.
pub fn test_pack_zip() -> Vec<u8> {
    use std::io::Write;
    const PNG: &[u8] = include_bytes!("../assets/test-stab.png");
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut buf);
        let o = zip::write::SimpleFileOptions::default();
        let files: [(&str, &[u8]); 3] = [
            (
                "pack.mcmeta",
                br#"{"pack":{"description":"Testpack","pack_format":75}}"#,
            ),
            (
                "assets/magie/models/item/stab.json",
                br#"{"parent":"minecraft:item/handheld","textures":{"layer0":"magie:item/stab"}}"#,
            ),
            ("assets/magie/textures/item/stab.png", PNG),
        ];
        for (n, d) in files {
            zw.start_file(n, o).unwrap();
            zw.write_all(d).unwrap();
        }
        zw.finish().unwrap();
    }
    buf.into_inner()
}

pub fn all_blocks_project(platform: Platform) -> Project {
    let mut p = new_project("Alle Bloecke", platform, Starter::Empty);
    p.settings.pack.path = "testpack.zip".into();
    p.workspaces = vec![Workspace::new("Test")];
    let mut b = Builder { p: &mut p, ws: 0 };
    let defs: Vec<_> = blocks::all()
        .iter()
        .filter(|d| d.supports(platform))
        .collect();

    // A command runs every action and flow block in sequence.
    let cmd = b.node("ev_command", 0.0, 0.0, &[("name", "testall")]);
    let mut last = (cmd, "next");
    let mut x = 300.0;
    for d in defs
        .iter()
        .filter(|d| matches!(d.kind, Kind::Action | Kind::Flow))
    {
        if d.id == "cancel_event" || d.id == "stop" {
            continue;
        }
        let vals: &[(&str, &str)] = if d.id == "custom_item" {
            &[("model", "magie:stab")]
        } else {
            &[]
        };
        let n = b.node(d.id, x, 0.0, vals);
        x += 300.0;
        b.link(last.0, last.1, n, "in");
        // Branches get a log block each.
        for o in d
            .outputs
            .iter()
            .filter(|o| o.ty == PinType::Exec && o.id != "next")
        {
            let l = b.node("log", x, 300.0, &[("text", &o.id)]);
            b.link(n, &o.id, l, "in");
            // Use loop/action outputs inside the branch.
            for out in d.outputs.iter().filter(|o| o.ty != PinType::Exec) {
                let l2 = b.node("log", x, 500.0, &[]);
                b.link(l, "next", l2, "in");
                b.link(n, &out.id, l2, "text");
            }
        }
        if d.outputs
            .iter()
            .any(|o| o.id == "next" && o.ty == PinType::Exec)
        {
            last = (n, "next");
        } else {
            // "if"/"chance" end the chain; continue from the log in the first branch.
            let l = b.node("log", x, 700.0, &[]);
            b.link(n, "then", l, "in");
            last = (l, "next");
        }
    }
    // Every value output goes into a log.
    for d in defs.iter().filter(|d| d.kind == Kind::Value) {
        let n = b.node(d.id, x, 400.0, &[]);
        for o in d.outputs.iter() {
            let l = b.node("log", x, 0.0, &[]);
            x += 300.0;
            b.link(last.0, last.1, l, "in");
            b.link(n, &o.id, l, "text");
            last = (l, "next");
        }
    }
    let stop = b.node("stop", x, 0.0, &[]);
    b.link(last.0, last.1, stop, "in");

    // Every other event logs its outputs, cancellable ones also get cancelled.
    let mut y = 1200.0;
    for d in defs
        .iter()
        .filter(|d| d.kind == Kind::Event && d.id != "ev_command")
    {
        let ev = b.node(d.id, 0.0, y, &[]);
        let mut last = (ev, "next");
        let mut x = 300.0;
        for o in d.outputs.iter().filter(|o| o.ty != PinType::Exec) {
            let l = b.node("log", x, y, &[]);
            x += 300.0;
            b.link(last.0, last.1, l, "in");
            b.link(ev, &o.id, l, "text");
            last = (l, "next");
        }
        if d.code(platform).map(|c| c.cancellable).unwrap_or(false) {
            let c = b.node("cancel_event", x, y, &[]);
            b.link(last.0, last.1, c, "in");
            last = (c, "next");
            x += 300.0;
        }
        let w = b.node("wait", x, y, &[]);
        b.link(last.0, last.1, w, "in");
        let m = b.node("send_message", x + 300.0, y, &[]);
        if d.outputs.iter().any(|o| o.ty == PinType::Player) {
            b.link(w, "next", m, "in");
        }
        y += 400.0;
    }
    p.migrate_commands();

    // A command that uses every option type.
    use crate::model::{CmdAccess, CmdOption, CmdSenders, CommandDef, OptType};
    let cid = p.alloc_id();
    let mut c = CommandDef::new(cid, "testopts");
    c.description = "Alle Optionstypen".into();
    c.aliases = vec!["topts".into()];
    c.access = CmdAccess::Permission;
    c.permission = "alle.testopts".into();
    c.senders = CmdSenders::Everyone;
    let opt = |name: &str, ty: OptType, required: bool, choices: &[&str]| CmdOption {
        name: name.into(),
        description: format!("Beschreibung von {name}"),
        ty,
        required,
        choices: choices.iter().map(|x| x.to_string()).collect(),
    };
    c.options = vec![
        opt("wort", OptType::Word, true, &["eins", "zwei"]),
        opt("zahl", OptType::Number, true, &[]),
        opt("ziel", OptType::Player, false, &[]),
        opt("an", OptType::Bool, false, &[]),
        opt("modus", OptType::Choice, false, &["leicht", "schwer"]),
        opt("ganz", OptType::Integer, false, &[]),
        opt("rest", OptType::Text, false, &[]),
    ];
    let n_opts = c.options.len();
    p.commands.push(c);
    let mut b = Builder { p: &mut p, ws: 0 };
    let cid_s = cid.to_string();
    let ev = b.node("ev_command", 0.0, y, &[("cmd", cid_s.as_str())]);
    let mut last = (ev, "next".to_string());
    let mut x = 300.0;
    for i in 0..n_opts {
        let l = b.node("log", x, y, &[]);
        x += 300.0;
        b.link(last.0, &last.1, l, "in");
        b.link(ev, &crate::model::opt_pin(i), l, "text");
        last = (l, "next".into());
    }
    p
}
