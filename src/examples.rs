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
    Home,
    Shop,
    Settings,
    Wand,
    Rewards,
    Protect,
    Sidebar,
    Homes,
    HomeMenu,
    WarpMenu,
}

impl Starter {
    pub const ALL: [Starter; 14] = [
        Starter::Empty,
        Starter::Welcome,
        Starter::Kit,
        Starter::Home,
        Starter::Homes,
        Starter::HomeMenu,
        Starter::WarpMenu,
        Starter::Shop,
        Starter::Settings,
        Starter::Counter,
        Starter::Wand,
        Starter::Rewards,
        Starter::Protect,
        Starter::Sidebar,
    ];
    /// Templates that work for the platform (some blocks exist only for Paper).
    pub fn for_platform(p: Platform) -> Vec<Starter> {
        Self::ALL
            .into_iter()
            .filter(|s| s.supports(p))
            .collect()
    }
    pub fn supports(self, p: Platform) -> bool {
        match self {
            Starter::Shop | Starter::Sidebar | Starter::HomeMenu | Starter::WarpMenu => {
                p == Platform::Paper
            }
            _ => true,
        }
    }
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            Starter::Empty => ("Leeres Projekt", "Empty project"),
            Starter::Welcome => ("Willkommensnachricht", "Welcome message"),
            Starter::Counter => ("Zähler mit Speicher", "Counter with storage"),
            Starter::Kit => ("/kit und /heal", "/kit and /heal"),
            Starter::Home => ("/sethome und /home", "/sethome and /home"),
            Starter::Shop => ("Shop-Menü", "Shop menu"),
            Starter::Settings => ("Einstellungs-Datei", "Settings file"),
            Starter::Wand => ("Zauberstab (eigenes Item)", "Magic wand (custom item)"),
            Starter::Rewards => ("Belohnungen", "Rewards"),
            Starter::Protect => ("Spawn-Schutz", "Spawn protection"),
            Starter::Sidebar => ("Scoreboard & Tab-Liste", "Scoreboard & tab list"),
            Starter::Homes => ("Mehrere Homes", "Several homes"),
            Starter::HomeMenu => ("Home-Menü", "Home menu"),
            Starter::WarpMenu => ("Warp-Menü", "Warp menu"),
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
                "/zaehlen merkt sich pro Spieler, wie oft er benutzt wurde.",
                "/count remembers per player how often it was used.",
            ),
            Starter::Kit => (
                "Starter-Kit-Befehl und ein Heil-Befehl nur für OPs.",
                "A starter kit command and an OP-only heal command.",
            ),
            Starter::Home => (
                "Spieler speichern ihre Position und teleportieren sich zurück.",
                "Players store their location and teleport back to it.",
            ),
            Starter::Shop => (
                "/shop öffnet ein Kisten-Menü; ein Klick tauscht Gold gegen Diamanten.",
                "/shop opens a chest menu; a click trades gold for diamonds.",
            ),
            Starter::Settings => (
                "Legt einen Ordner mit settings.yml an und liest die Begrüßung daraus.",
                "Creates a folder with settings.yml and reads the greeting from it.",
            ),
            Starter::Wand => (
                "Ein Item aus deinem Resourcepack, das per Rechtsklick Blitze schießt.",
                "An item from your resource pack that shoots lightning on right click.",
            ),
            Starter::Rewards => (
                "Belohnt das Abbauen von Diamanterz und das Besiegen von Mobs.",
                "Rewards mining diamond ore and defeating mobs.",
            ),
            Starter::Protect => (
                "Im Bereich um den Spawn dürfen nur OPs Blöcke abbauen.",
                "Only OPs may break blocks in the area around spawn.",
            ),
            Starter::Sidebar => (
                "Zeigt jedem Spieler eine Seitenleiste und Text in der Tab-Liste.",
                "Shows every player a sidebar and text in the tab list.",
            ),
            Starter::Homes => (
                "/sethome, /home, /delhome und /homes mit beliebig vielen Namen pro Spieler.",
                "/sethome, /home, /delhome and /homes with any number of names per player.",
            ),
            Starter::HomeMenu => (
                "/homemenu: 3 Homes per Klick setzen und hinteleportieren – ganz ohne Kabel.",
                "/homemenu: set 3 homes and teleport to them by click – no wires at all.",
            ),
            Starter::WarpMenu => (
                "/setwarp für OPs, /warps öffnet ein Menü mit Spawn, Arena und Shop.",
                "/setwarp for OPs, /warps opens a menu with spawn, arena and shop.",
            ),
        }
    }
    /// Short tags shown on the template card.
    pub fn tags(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Starter::Empty => &[],
            Starter::Welcome => &[("Events", "Events"), ("Nachrichten", "Messages")],
            Starter::Counter => &[("Befehl", "Command"), ("Speicher", "Storage")],
            Starter::Kit => &[("Befehle", "Commands"), ("Items", "Items")],
            Starter::Home => &[("Befehle", "Commands"), ("Positionen", "Locations")],
            Starter::Shop => &[("Menü", "Menu"), ("Items", "Items")],
            Starter::Settings => &[("Dateien", "Files"), ("Einstellungen", "Settings")],
            Starter::Wand => &[("Resourcepack", "Resource pack"), ("Cooldown", "Cooldown")],
            Starter::Rewards => &[("Events", "Events"), ("Items", "Items")],
            Starter::Protect => &[("Positionen", "Locations"), ("Logik", "Logic")],
            Starter::Sidebar => &[("Scoreboard", "Scoreboard")],
            Starter::Homes => &[("Befehle", "Commands"), ("Positionen", "Locations")],
            Starter::HomeMenu => &[("Menü", "Menu"), ("Positionen", "Locations")],
            Starter::WarpMenu => &[("Menü", "Menu"), ("Positionen", "Locations")],
        }
    }
    pub fn icon(self) -> crate::ui::icons::Icon {
        use crate::ui::icons::Icon as I;
        match self {
            Starter::Empty => I::Plus,
            Starter::Welcome => I::Message,
            Starter::Counter => I::Database,
            Starter::Kit => I::Slash,
            Starter::Home => I::Home,
            Starter::Shop => I::Chest,
            Starter::Settings => I::Folder,
            Starter::Wand => I::Sparkle,
            Starter::Rewards => I::Star,
            Starter::Protect => I::Pin,
            Starter::Sidebar => I::Layers,
            Starter::Homes => I::Home,
            Starter::HomeMenu => I::Chest,
            Starter::WarpMenu => I::Pin,
        }
    }
    pub fn color(self) -> egui::Color32 {
        use crate::blocks::Cat as C;
        match self {
            Starter::Empty => C::Server.color(),
            Starter::Welcome => C::Messages.color(),
            Starter::Counter => C::Data.color(),
            Starter::Kit => C::Events.color(),
            Starter::Home | Starter::Protect | Starter::Homes => C::Location.color(),
            Starter::HomeMenu | Starter::WarpMenu => C::Player.color(),
            Starter::Shop => C::Player.color(),
            Starter::Settings => C::Files.color(),
            Starter::Wand => C::Resources.color(),
            Starter::Rewards => C::World.color(),
            Starter::Sidebar => C::Flow.color(),
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
    /// A command with options and its event block.
    fn command(
        &mut self,
        name: &str,
        desc: &str,
        op: bool,
        options: &[(&str, crate::model::OptType)],
        x: f32,
        y: f32,
    ) -> NodeId {
        use crate::model::{CmdAccess, CmdOption, CommandDef};
        let cid = self.p.alloc_id();
        let mut c = CommandDef::new(cid, name);
        c.description = desc.into();
        if op {
            c.access = CmdAccess::Op;
        }
        c.options = options
            .iter()
            .map(|(n, ty)| CmdOption {
                name: n.to_string(),
                description: String::new(),
                ty: *ty,
                required: true,
                choices: vec![],
            })
            .collect();
        self.p.commands.push(c);
        let cid_s = cid.to_string();
        self.node("ev_command", x, y, &[("cmd", cid_s.as_str())])
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
        Starter::Home => {
            let set = b.node("ev_command", 32.0, 32.0, &[("name", "sethome"), ("description", "Speichert dein Zuhause")]);
            let save = b.node("loc_save", 352.0, 32.0, &[("key", "home")]);
            let m = b.node("send_message", 672.0, 32.0, &[("text", "&aZuhause gespeichert! Mit &e/home &akommst du zurück.")]);
            b.flow(set, save);
            b.flow(save, m);
            let home = b.node("ev_command", 32.0, 384.0, &[("name", "home"), ("description", "Teleportiert dich nach Hause")]);
            let load = b.node("loc_load", 352.0, 384.0, &[("key", "home")]);
            let tp = b.node("teleport", 672.0, 384.0, &[]);
            let ok = b.node("send_message", 992.0, 384.0, &[("text", "&aWillkommen zu Hause!")]);
            let no = b.node("send_message", 672.0, 608.0, &[("text", "&cDu hast noch kein Zuhause. Benutze &e/sethome&c.")]);
            b.flow(home, load);
            b.link(load, "found", tp, "in");
            b.link(load, "loc", tp, "loc");
            b.flow(tp, ok);
            b.link(load, "missing", no, "in");
        }
        Starter::Shop => {
            let cmd = b.node("ev_command", 32.0, 32.0, &[("name", "shop"), ("description", "Öffnet den Shop")]);
            let menu = b.node(
                "open_menu",
                352.0,
                32.0,
                &[
                    ("menu", "shop"),
                    ("title", "&8Shop"),
                    ("rows", "3"),
                    ("items", "11\tminecraft:diamond\t&bDiamant kaufen\t&7Kostet &65 Goldbarren\t1\t\t\tdiamant_kaufen\n13\tminecraft:gold_ingot\t&6Dein Gold\t&7Damit bezahlst du\t5\t\t\t\n15\tminecraft:barrier\t&cSchließen\t\t1\t\t\tschliessen"),
                ],
            );
            b.flow(cmd, menu);
            // Click on the diamond: pay 5 gold, get a diamond.
            let buy = b.node("ev_menu_click", 32.0, 480.0, &[("menu", "shop"), ("click_id", "diamant_kaufen")]);
            let take = b.node("take_items", 352.0, 480.0, &[("item", "minecraft:gold_ingot"), ("amount", "5")]);
            let paid = b.node("if", 672.0, 480.0, &[]);
            let give = b.node("give_item", 992.0, 480.0, &[("item", "minecraft:diamond")]);
            let ok = b.node("send_message", 1312.0, 480.0, &[("text", "&aGekauft: &b1 Diamant")]);
            let no = b.node("send_message", 992.0, 736.0, &[("text", "&cDu brauchst 5 Goldbarren.")]);
            b.flow(buy, take);
            b.flow(take, paid);
            b.link(take, "ok", paid, "cond");
            b.link(paid, "then", give, "in");
            b.flow(give, ok);
            b.link(paid, "else", no, "in");
            // Click on the barrier: close.
            let shut = b.node("ev_menu_click", 32.0, 960.0, &[("menu", "shop"), ("click_id", "schliessen")]);
            let close = b.node("close_menu", 352.0, 960.0, &[]);
            b.flow(shut, close);
        }
        Starter::Settings => {
            let s = b.node("ev_start", 32.0, 32.0, &[]);
            let f = b.node("folder_create", 352.0, 32.0, &[("path", "daten")]);
            let d1 = b.node(
                "cfg_default",
                672.0,
                32.0,
                &[
                    ("file", "settings.yml"),
                    ("key", "willkommen"),
                    ("value", "&aWillkommen auf dem Server!"),
                    ("comment", "Nachricht, die Spieler beim Betreten sehen"),
                ],
            );
            let d2 = b.node(
                "cfg_default",
                992.0,
                32.0,
                &[
                    ("file", "settings.yml"),
                    ("key", "begruessung-aktiv"),
                    ("value", "true"),
                    ("comment", "Begrüßung an- oder ausschalten (true/false)"),
                ],
            );
            b.flow(s, f);
            b.flow(f, d1);
            b.flow(d1, d2);
            let j = b.node("ev_join", 32.0, 416.0, &[]);
            let on = b.node("cfg_bool", 32.0, 640.0, &[("file", "settings.yml"), ("key", "begruessung-aktiv")]);
            let iff = b.node("if", 352.0, 416.0, &[]);
            let txt = b.node("cfg_text", 352.0, 672.0, &[("file", "settings.yml"), ("key", "willkommen")]);
            let m = b.node("send_message", 672.0, 416.0, &[]);
            b.flow(j, iff);
            b.link(on, "value", iff, "cond");
            b.link(iff, "then", m, "in");
            b.link(txt, "value", m, "text");
            let r = b.node("ev_command", 32.0, 960.0, &[("name", "einstellungen"), ("description", "Lädt settings.yml neu"), ("op", "true")]);
            let rl = b.node("cfg_reload", 352.0, 960.0, &[("file", "settings.yml")]);
            let rm = b.node("send_message", 672.0, 960.0, &[("text", "&aEinstellungen neu geladen.")]);
            b.flow(r, rl);
            b.flow(rl, rm);
        }
        Starter::Wand => {
            let c = b.node("ev_command", 32.0, 32.0, &[("name", "zauberstab"), ("description", "Gibt dir den Zauberstab"), ("op", "true")]);
            let g = b.node("custom_item", 352.0, 32.0, &[("item", "minecraft:stick"), ("name", "&6Zauberstab"), ("lore", "&7Rechtsklick: Blitz")]);
            b.flow(c, g);
            let u = b.node("ev_custom_use", 32.0, 448.0, &[]);
            let cd = b.node("cooldown", 352.0, 448.0, &[("seconds", "3"), ("key", "zauberstab")]);
            let look = b.node("look_loc", 352.0, 800.0, &[("distance", "40")]);
            let l = b.node("lightning", 672.0, 448.0, &[]);
            let w = b.node("actionbar", 672.0, 672.0, &[("text", "&cDer Stab lädt noch auf …")]);
            b.flow(u, cd);
            b.link(cd, "ready", l, "in");
            b.link(look, "loc", l, "loc");
            b.link(cd, "wait", w, "in");
            let id = b.p.alloc_id();
            b.p.workspaces[0].notes.push(crate::model::Note {
                id,
                pos: [8.0, -80.0],
                size: [940.0, 80.0],
                title: "So geht es weiter".into(),
                text: "Füge links unter „Texturen & Modelle“ ein Resourcepack hinzu und wähle in beiden Blöcken das Modell des Stabs.".into(),
                color: 2,
            });
        }
        Starter::Rewards => {
            let br = b.node("ev_break", 32.0, 32.0, &[]);
            let eq = b.node("text_equals", 32.0, 352.0, &[("b", "minecraft:diamond_ore")]);
            let iff = b.node("if", 352.0, 32.0, &[]);
            let add = b.node("data_add", 672.0, 32.0, &[("key", "diamanten")]);
            let fmt = b.node("format", 672.0, 352.0, &[("format", "&bDiamant Nr. &e{0}&b – weiter so!")]);
            let msg = b.node("actionbar", 992.0, 32.0, &[]);
            b.link(br, "block", eq, "a");
            b.link(eq, "res", iff, "cond");
            b.flow(br, iff);
            b.link(iff, "then", add, "in");
            b.link(add, "value", fmt, "p0");
            b.flow(add, msg);
            b.link(fmt, "res", msg, "text");
            let k = b.node("ev_kill", 32.0, 736.0, &[]);
            let ch = b.node("chance", 352.0, 736.0, &[]);
            let g = b.node("give_item", 672.0, 736.0, &[("item", "minecraft:emerald")]);
            let m = b.node("send_message", 992.0, 736.0, &[("text", "&aGlück gehabt: +1 Smaragd!")]);
            b.flow(k, ch);
            b.link(ch, "then", g, "in");
            b.flow(g, m);
        }
        Starter::Protect => {
            let br = b.node("ev_break", 32.0, 32.0, &[]);
            let a = b.node("make_loc", 32.0, 352.0, &[("world", "world"), ("x", "-20"), ("y", "-64"), ("z", "-20")]);
            let c = b.node("make_loc", 32.0, 640.0, &[("world", "world"), ("x", "20"), ("y", "320"), ("z", "20")]);
            let area = b.node("loc_in_area", 352.0, 352.0, &[]);
            let op = b.node("is_op", 352.0, 640.0, &[]);
            let not = b.node("not", 672.0, 640.0, &[]);
            let and = b.node("and", 672.0, 352.0, &[]);
            let iff = b.node("if", 992.0, 32.0, &[]);
            let cancel = b.node("cancel_event", 1312.0, 32.0, &[]);
            let msg = b.node("actionbar", 1632.0, 32.0, &[("text", "&cHier darfst du nichts abbauen.")]);
            b.link(br, "loc", area, "loc");
            b.link(a, "loc", area, "a");
            b.link(c, "loc", area, "b");
            b.link(br, "player", op, "player");
            b.link(op, "op", not, "a");
            b.link(area, "res", and, "a");
            b.link(not, "res", and, "b");
            b.link(and, "res", iff, "cond");
            b.flow(br, iff);
            b.link(iff, "then", cancel, "in");
            b.flow(cancel, msg);
        }
        Starter::Homes => {
            use crate::model::OptType;
            let set = b.command("sethome", "Speichert ein Home", false, &[("name", OptType::Word)], 32.0, 32.0);
            let save = b.node("loc_save", 352.0, 32.0, &[]);
            let fm = b.node("format", 352.0, 288.0, &[("format", "&aHome &e{0}&a gespeichert.")]);
            let m = b.node("send_message", 672.0, 32.0, &[]);
            b.link(set, "opt0", save, "key");
            b.link(set, "opt0", fm, "p0");
            b.flow(set, save);
            b.flow(save, m);
            b.link(fm, "res", m, "text");

            let home = b.command("home", "Teleportiert zu einem Home", false, &[("name", OptType::Word)], 32.0, 560.0);
            let load = b.node("loc_load", 352.0, 560.0, &[]);
            let tp = b.node("teleport", 672.0, 560.0, &[]);
            let ok = b.node("send_message", 992.0, 560.0, &[("text", "&aWillkommen zu Hause!")]);
            let no = b.node("send_message", 672.0, 800.0, &[("text", "&cDieses Home gibt es nicht. &7/homes zeigt deine Homes.")]);
            b.link(home, "opt0", load, "key");
            b.flow(home, load);
            b.link(load, "found", tp, "in");
            b.link(load, "loc", tp, "loc");
            b.flow(tp, ok);
            b.link(load, "missing", no, "in");

            let del = b.command("delhome", "Löscht ein Home", false, &[("name", OptType::Word)], 32.0, 1060.0);
            let rm = b.node("loc_delete", 352.0, 1060.0, &[]);
            let dm = b.node("send_message", 672.0, 1060.0, &[("text", "&7Home gelöscht.")]);
            b.link(del, "opt0", rm, "key");
            b.flow(del, rm);
            b.flow(rm, dm);

            let list = b.command("homes", "Zeigt deine Homes", false, &[], 32.0, 1340.0);
            let names = b.node("loc_list", 32.0, 1560.0, &[]);
            let lf = b.node("format", 352.0, 1560.0, &[("format", "&aDeine Homes &7({1})&a: &f{0}")]);
            let lm = b.node("send_message", 352.0, 1340.0, &[]);
            b.link(names, "names", lf, "p0");
            b.link(names, "count", lf, "p1");
            b.link(lf, "res", lm, "text");
            b.flow(list, lm);
        }
        Starter::HomeMenu => {
            let cmd = b.command("homemenu", "Öffnet dein Home-Menü", false, &[], 32.0, 32.0);
            let mut items = vec![];
            for (i, (slot_set, slot_tp)) in [(2, 20), (4, 22), (6, 24)].iter().enumerate() {
                let n = i + 1;
                items.push(format!(
                    "{slot_set}\tminecraft:red_bed\t&aHome {n} setzen\t&7Speichert, wo du gerade stehst\t1\t\t\t\tsave_loc\thome{n}"
                ));
                items.push(format!(
                    "{slot_tp}\tminecraft:ender_pearl\t&bZu Home {n}\t&7Teleportiert dich zu Home {n}\t1\t\t\t\ttp_loc\thome{n}"
                ));
            }
            items.push("13\tminecraft:compass\t&eDeine Homes\t&7Oben setzen|&7Unten hinteleportieren\t1\t\t\t\t\t".into());
            items.push("26\tminecraft:barrier\t&cSchließen\t\t1\t\t\t\tclose\t".into());
            let items = items.join("\n");
            let menu = b.node(
                "open_menu",
                352.0,
                32.0,
                &[("menu", "homes"), ("title", "&8Deine Homes"), ("rows", "3"), ("items", items.as_str())],
            );
            b.flow(cmd, menu);
            let id = b.p.alloc_id();
            b.p.workspaces[0].notes.push(crate::model::Note {
                id,
                pos: [8.0, -80.0],
                size: [700.0, 80.0],
                title: "Ganz ohne Kabel".into(),
                text: "Was ein Klick macht, steht im Menü-Editor bei jedem Platz unter „Beim Klick“.".into(),
                color: 1,
            });
        }
        Starter::WarpMenu => {
            use crate::model::OptType;
            let set = b.command("setwarp", "Setzt einen Warp (nur OPs)", true, &[("name", OptType::Word)], 32.0, 32.0);
            let save = b.node("loc_save", 352.0, 32.0, &[("scope", "global")]);
            let fm = b.node("format", 352.0, 288.0, &[("format", "&aWarp &e{0}&a gesetzt.")]);
            let m = b.node("send_message", 672.0, 32.0, &[]);
            b.link(set, "opt0", save, "key");
            b.link(set, "opt0", fm, "p0");
            b.flow(set, save);
            b.flow(save, m);
            b.link(fm, "res", m, "text");
            let open = b.command("warps", "Öffnet das Warp-Menü", false, &[], 32.0, 560.0);
            let items = [
                "11\tminecraft:compass\t&bSpawn\t&7/setwarp spawn\t1\t\t\t\ttp_warp\tspawn",
                "13\tminecraft:diamond_sword\t&cArena\t&7/setwarp arena\t1\t\t\t\ttp_warp\tarena",
                "15\tminecraft:emerald\t&aShop\t&7/setwarp shop\t1\t\t\t\ttp_warp\tshop",
            ]
            .join("\n");
            let menu = b.node(
                "open_menu",
                352.0,
                560.0,
                &[("menu", "warps"), ("title", "&8Warps"), ("rows", "3"), ("items", items.as_str())],
            );
            b.flow(open, menu);
        }
        Starter::Sidebar => {
            let j = b.node("ev_join", 32.0, 32.0, &[]);
            let cnt = b.node("online_count", 32.0, 320.0, &[]);
            let fmt = b.node("format", 352.0, 384.0, &[("format", "&7Willkommen!|&7Online: &a{0}|&7Viel Spaß")]);
            let sb = b.node("sidebar", 352.0, 32.0, &[("title", "&6&lMein Server")]);
            let tab = b.node("tablist", 672.0, 32.0, &[]);
            b.link(cnt, "count", fmt, "p0");
            b.link(fmt, "res", sb, "lines");
            b.flow(j, sb);
            b.flow(sb, tab);
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
    p.settings.pack.packs = vec![crate::model::PackEntry {
        path: "testpack.zip".into(),
        enabled: true,
    }];
    p.settings.pack.give_command = "testitems".into();
    // Debug messages on, so CI compiles that code too.
    p.settings.debug = true;
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
            // Branching blocks ("if", "chance", "load location" …) end the chain;
            // continue from a log in their first branch.
            let first = d
                .outputs
                .iter()
                .find(|o| o.ty == PinType::Exec)
                .map(|o| o.id.to_string())
                .unwrap_or_else(|| "then".into());
            let l = b.node("log", x, 700.0, &[]);
            b.link(n, &first, l, "in");
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
