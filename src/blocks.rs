//! The block library. Every block knows its pins, its settings and the Java
//! code it produces for each platform (Paper plugin / Fabric mod).
//!
//! Code templates use `${...}` placeholders:
//! * `${in:pin}`    expression of a data input (connected value or inline default)
//! * `${f:field}`   raw field value (for enum names / numbers)
//! * `${s:field}`   field value as Java string literal
//! * `${out:pin}`   local variable name of an output of this block
//! * `${exec:pin}`  generated code of an exec branch
//! * `${server}` `${plugin}` `${main}` `${storekey}` `${id}` context values

use crate::i18n::L;
use crate::model::Platform;
use egui::Color32;
use std::borrow::Cow;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum PinType {
    Exec,
    Player,
    Entity,
    Text,
    Number,
    Bool,
    Location,
    Item,
}

impl PinType {
    pub fn color(self) -> Color32 {
        match self {
            PinType::Exec => Color32::from_rgb(0x3D, 0xFF, 0x6E),
            PinType::Player => Color32::from_rgb(0x72, 0x7C, 0xFF),
            PinType::Entity => Color32::from_rgb(0xB0, 0x7C, 0xFF),
            PinType::Text => Color32::from_rgb(0xFF, 0x7A, 0xC6),
            PinType::Number => Color32::from_rgb(0x45, 0xC8, 0xFF),
            PinType::Bool => Color32::from_rgb(0xFF, 0x5C, 0x5C),
            PinType::Location => Color32::from_rgb(0xFF, 0xC8, 0x4D),
            PinType::Item => Color32::from_rgb(0xFF, 0x9A, 0x3D),
        }
    }

    pub fn label(self) -> L {
        match self {
            PinType::Exec => ("Ablauf", "Flow"),
            PinType::Player => ("Spieler", "Player"),
            PinType::Entity => ("Entity", "Entity"),
            PinType::Text => ("Text", "Text"),
            PinType::Number => ("Zahl", "Number"),
            PinType::Bool => ("Ja/Nein", "Yes/No"),
            PinType::Location => ("Position", "Location"),
            PinType::Item => ("Item", "Item"),
        }
    }

    pub fn java(self, p: Platform) -> &'static str {
        match (self, p) {
            (PinType::Exec, _) => "void",
            (PinType::Text, _) => "String",
            (PinType::Number, _) => "double",
            (PinType::Bool, _) => "boolean",
            (PinType::Player, Platform::Paper) => "org.bukkit.entity.Player",
            (PinType::Player, Platform::Fabric) => "net.minecraft.server.level.ServerPlayer",
            (PinType::Entity, Platform::Paper) => "org.bukkit.entity.Entity",
            (PinType::Entity, Platform::Fabric) => "net.minecraft.world.entity.Entity",
            (PinType::Location, Platform::Paper) => "org.bukkit.Location",
            (PinType::Location, Platform::Fabric) => "Loc",
            (PinType::Item, Platform::Paper) => "org.bukkit.inventory.ItemStack",
            (PinType::Item, Platform::Fabric) => "net.minecraft.world.item.ItemStack",
        }
    }

    /// Does this type get an inline editor when the pin is not connected?
    pub fn has_inline(self) -> bool {
        matches!(self, PinType::Text | PinType::Number | PinType::Bool)
    }
}

/// Can a value of type `from` be plugged into a pin of type `to`?
pub fn compatible(from: PinType, to: PinType) -> bool {
    use PinType::*;
    if from == to {
        return true;
    }
    matches!(
        (from, to),
        (Number, Text)
            | (Bool, Text)
            | (Player, Text)
            | (Entity, Text)
            | (Text, Number)
            | (Player, Entity)
            | (Player, Location)
            | (Entity, Location)
            | (Location, Text)
            | (Item, Text)
    )
}

/// Wraps an expression to convert between pin types.
pub fn convert(expr: &str, from: PinType, to: PinType, p: Platform) -> String {
    use PinType::*;
    if from == to {
        return expr.to_string();
    }
    match (from, to, p) {
        (Number, Text, _) => format!("Util.fmt({expr})"),
        (Bool, Text, _) => format!("String.valueOf({expr})"),
        (Text, Number, _) => format!("Util.num({expr})"),
        (Player, Entity, _) => expr.to_string(),
        (Player | Entity, Text, Platform::Paper) => format!("Util.name({expr})"),
        (Player | Entity, Text, Platform::Fabric) => format!("Util.name({expr})"),
        (Player | Entity, Location, Platform::Paper) => format!("({expr}).getLocation()"),
        (Player | Entity, Location, Platform::Fabric) => format!("Loc.of({expr})"),
        (Location, Text, _) => format!("Util.locText({expr})"),
        (Item, Text, _) => format!("Util.itemId({expr})"),
        _ => expr.to_string(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Event,
    Action,
    Flow,
    Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cat {
    Events,
    Messages,
    Player,
    World,
    Flow,
    Logic,
    Math,
    Text,
    Data,
    Resources,
    Server,
    Location,
    Files,
    Recipes,
    Moderation,
    Vars,
}

pub const CATEGORIES: [Cat; 16] = [
    Cat::Events,
    Cat::Messages,
    Cat::Player,
    Cat::World,
    Cat::Location,
    Cat::Flow,
    Cat::Logic,
    Cat::Math,
    Cat::Text,
    Cat::Vars,
    Cat::Data,
    Cat::Files,
    Cat::Recipes,
    Cat::Resources,
    Cat::Moderation,
    Cat::Server,
];

impl Cat {
    pub fn label(self) -> L {
        match self {
            Cat::Events => ("Events", "Events"),
            Cat::Messages => ("Nachrichten", "Messages"),
            Cat::Player => ("Spieler", "Player"),
            Cat::World => ("Welt", "World"),
            Cat::Flow => ("Ablauf", "Flow"),
            Cat::Logic => ("Logik", "Logic"),
            Cat::Math => ("Mathe", "Math"),
            Cat::Text => ("Text", "Text"),
            Cat::Data => ("Gespeicherte Daten", "Stored data"),
            Cat::Resources => ("Texturen & Modelle", "Textures & models"),
            Cat::Server => ("Server", "Server"),
            Cat::Location => ("Positionen", "Locations"),
            Cat::Files => ("Dateien & Einstellungen", "Files & settings"),
            Cat::Recipes => ("Rezepte", "Recipes"),
            Cat::Moderation => ("Moderation (Bann & Kick)", "Moderation (ban & kick)"),
            Cat::Vars => ("Variablen & Listen", "Variables & lists"),
        }
    }
    pub fn color(self) -> Color32 {
        match self {
            Cat::Events => Color32::from_rgb(0x3D, 0xFF, 0x6E),
            Cat::Messages => Color32::from_rgb(0x4D, 0xA3, 0xFF),
            Cat::Player => Color32::from_rgb(0x00, 0xE5, 0xFF),
            Cat::World => Color32::from_rgb(0x3D, 0xD6, 0x8C),
            Cat::Flow => Color32::from_rgb(0xB0, 0x7C, 0xFF),
            Cat::Logic => Color32::from_rgb(0xFF, 0x5C, 0x7A),
            Cat::Math => Color32::from_rgb(0x45, 0xC8, 0xFF),
            Cat::Text => Color32::from_rgb(0xF2, 0xC1, 0x4E),
            Cat::Data => Color32::from_rgb(0xFF, 0x7A, 0xC6),
            Cat::Resources => Color32::from_rgb(0xC0, 0x8B, 0xFF),
            Cat::Server => Color32::from_rgb(0x9A, 0xA4, 0xB2),
            Cat::Location => Color32::from_rgb(0xFF, 0xC8, 0x4D),
            Cat::Files => Color32::from_rgb(0xFF, 0x9A, 0x3D),
            Cat::Recipes => Color32::from_rgb(0xD9, 0xA0, 0x66),
            Cat::Moderation => Color32::from_rgb(0xFF, 0x55, 0x55),
            Cat::Vars => Color32::from_rgb(0x7A, 0xE0, 0xC0),
        }
    }
    pub fn icon_kind(self) -> crate::ui::icons::Icon {
        use crate::ui::icons::Icon as I;
        match self {
            Cat::Events => I::Bolt,
            Cat::Messages => I::Message,
            Cat::Player => I::User,
            Cat::World => I::Globe,
            Cat::Flow => I::Branch,
            Cat::Logic => I::Scale,
            Cat::Math => I::Sigma,
            Cat::Text => I::Text,
            Cat::Data => I::Database,
            Cat::Resources => I::Image,
            Cat::Server => I::Server,
            Cat::Location => I::Pin,
            Cat::Files => I::Folder,
            Cat::Recipes => I::Grid,
            Cat::Moderation => I::Warning,
            Cat::Vars => I::Box,
        }
    }
    #[allow(dead_code)]
    pub fn icon(self) -> &'static str {
        match self {
            Cat::Events => "⚡",
            Cat::Messages => "✉",
            Cat::Player => "☺",
            Cat::World => "🌍",
            Cat::Flow => "⤴",
            Cat::Logic => "⚖",
            Cat::Math => "∑",
            Cat::Text => "✏",
            Cat::Data => "💾",
            Cat::Resources => "🎨",
            Cat::Server => "🖧",
            Cat::Location => "📍",
            Cat::Files => "📁",
            Cat::Recipes => "⚒",
            Cat::Moderation => "⛔",
            Cat::Vars => "📦",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PinDef {
    pub id: Cow<'static, str>,
    pub label: Label,
    pub ty: PinType,
    /// Inline default value for unconnected Text/Number/Bool inputs.
    pub default: &'static str,
    /// Optional input that is folded away under "more options" on the block.
    pub advanced: bool,
    /// What this socket does (shown when hovering it). Falls back to `pin_help`.
    pub help: Option<L>,
}

/// Pin label: fixed bilingual text, or a dynamic one (command options).
#[derive(Clone, Debug)]
pub enum Label {
    S(L),
    D(String),
}

impl Label {
    pub fn text(&self) -> &str {
        match self {
            Label::S(l) => crate::i18n::tl(*l),
            Label::D(s) => s,
        }
    }
    /// German text, used in problem messages.
    pub fn de(&self) -> &str {
        match self {
            Label::S(l) => l.0,
            Label::D(s) => s,
        }
    }
}

impl PinDef {
    pub fn exec(id: &'static str) -> Self {
        PinDef {
            id: Cow::Borrowed(id),
            label: Label::S(("", "")),
            ty: PinType::Exec,
            default: "",
            advanced: false,
            help: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum FieldKind {
    Text,
    Multiline,
    Number,
    Bool,
    Choice(&'static [(&'static str, L)]),
    /// Item model id, with a picker listing the models of the project's resource pack.
    ItemModel,
    /// Chest menu layout, edited in the menu editor window.
    Menu,
    /// Crafting recipe (grid + result), edited in the recipe editor.
    Recipe,
    /// Text of the disconnect screen (ban / kick), edited in the screen editor.
    Screen,
    /// ID of a slot of a menu (picked from the menu editor's IDs, empty = any).
    MenuItemId,
    /// Name of a menu, picked from the "Open menu" blocks of the project.
    MenuName,
}

#[derive(Clone, Debug)]
pub struct FieldDef {
    pub id: &'static str,
    pub label: L,
    pub kind: FieldKind,
    pub default: &'static str,
    /// Only shown in the inspector / commands window, not on the block itself.
    pub advanced: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Special {
    None,
    /// Runs once when the plugin / server starts.
    Start,
    /// Registers a chat command.
    Command,
    /// Runs repeatedly every N seconds.
    Timer,
    /// Adds a crafting recipe when the plugin / mod starts (no flow needed).
    Recipe,
    /// A named flow that "Run own block" starts.
    Function,
}

#[derive(Clone, Debug, Default)]
pub struct PlatCode {
    /// Action/flow: statement template.
    /// Paper event: guard code at the start of the listener method.
    /// Fabric event: the full `register(...)` call with a `${body}` slot.
    pub code: &'static str,
    /// Output pin -> Java expression.
    pub outs: Vec<(&'static str, &'static str)>,
    pub imports: Vec<&'static str>,
    /// Paper: fully qualified event class.
    pub listener: &'static str,
    pub cancellable: bool,
    /// Fabric event: statement that ends the handler early (e.g. `return true;`).
    pub ret: &'static str,
}

#[derive(Clone, Debug)]
pub struct BlockDef {
    pub id: &'static str,
    pub cat: Cat,
    pub kind: Kind,
    pub name: L,
    pub desc: L,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
    pub fields: Vec<FieldDef>,
    pub paper: Option<PlatCode>,
    pub fabric: Option<PlatCode>,
    pub special: Special,
}

impl BlockDef {
    pub fn code(&self, p: Platform) -> Option<&PlatCode> {
        match p {
            Platform::Paper => self.paper.as_ref(),
            Platform::Fabric => self.fabric.as_ref(),
        }
    }
    pub fn supports(&self, p: Platform) -> bool {
        self.code(p).is_some()
    }
    pub fn input(&self, id: &str) -> Option<&PinDef> {
        self.inputs.iter().find(|p| p.id == id)
    }
    pub fn output(&self, id: &str) -> Option<&PinDef> {
        self.outputs.iter().find(|p| p.id == id)
    }
    pub fn field(&self, id: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.id == id)
    }
    pub fn has_exec_in(&self) -> bool {
        self.inputs.iter().any(|p| p.ty == PinType::Exec)
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Block being built; the flag remembers whether the last pin added was an output.
struct B(BlockDef, bool);

fn b(id: &'static str, cat: Cat, kind: Kind, name: L, desc: L) -> B {
    let mut d = BlockDef {
        id,
        cat,
        kind,
        name,
        desc,
        inputs: vec![],
        outputs: vec![],
        fields: vec![],
        paper: None,
        fabric: None,
        special: Special::None,
    };
    match kind {
        Kind::Action => {
            d.inputs.push(PinDef::exec("in"));
            d.outputs.push(PinDef::exec("next"));
        }
        Kind::Flow => {
            d.inputs.push(PinDef::exec("in"));
        }
        Kind::Event => {
            d.outputs.push(PinDef::exec("next"));
        }
        Kind::Value => {}
    }
    B(d, false)
}

impl B {
    fn i(mut self, id: &'static str, label: L, ty: PinType, default: &'static str) -> Self {
        self.0.inputs.push(PinDef {
            id: Cow::Borrowed(id),
            label: Label::S(label),
            ty,
            default,
            advanced: false,
            help: None,
        });
        self.1 = false;
        self
    }
    /// Optional input, folded away on the block until opened or connected.
    fn ia(mut self, id: &'static str, label: L, ty: PinType, default: &'static str) -> Self {
        self.0.inputs.push(PinDef {
            id: Cow::Borrowed(id),
            label: Label::S(label),
            ty,
            default,
            advanced: true,
            help: None,
        });
        self.1 = false;
        self
    }
    fn o(mut self, id: &'static str, label: L, ty: PinType) -> Self {
        self.0.outputs.push(PinDef {
            id: Cow::Borrowed(id),
            label: Label::S(label),
            ty,
            default: "",
            advanced: false,
            help: None,
        });
        self.1 = true;
        self
    }
    /// Describes the socket added last (shown on hover).
    fn h(mut self, help: L) -> Self {
        let pin = if self.1 {
            self.0.outputs.last_mut()
        } else {
            self.0.inputs.last_mut()
        };
        if let Some(p) = pin {
            p.help = Some(help);
        }
        self
    }
    fn f(mut self, id: &'static str, label: L, kind: FieldKind, default: &'static str) -> Self {
        self.0.fields.push(FieldDef {
            id,
            label,
            kind,
            default,
            advanced: false,
        });
        self
    }
    /// Field that is only shown in the inspector.
    #[allow(dead_code)]
    fn fa(mut self, id: &'static str, label: L, kind: FieldKind, default: &'static str) -> Self {
        self.0.fields.push(FieldDef {
            id,
            label,
            kind,
            default,
            advanced: true,
        });
        self
    }
    fn special(mut self, s: Special) -> Self {
        self.0.special = s;
        self
    }
    fn pc(&mut self) -> &mut PlatCode {
        self.0.paper.get_or_insert_with(PlatCode::default)
    }
    fn fc(&mut self) -> &mut PlatCode {
        self.0.fabric.get_or_insert_with(PlatCode::default)
    }
    /// Paper statement / guard template.
    fn paper(mut self, code: &'static str) -> Self {
        self.pc().code = code;
        self
    }
    /// Fabric statement / register template.
    fn fabric(mut self, code: &'static str) -> Self {
        self.fc().code = code;
        self
    }
    /// Same template for both platforms.
    fn both(self, code: &'static str) -> Self {
        self.paper(code).fabric(code)
    }
    fn pout(mut self, pin: &'static str, expr: &'static str) -> Self {
        self.pc().outs.push((pin, expr));
        self
    }
    fn fout(mut self, pin: &'static str, expr: &'static str) -> Self {
        self.fc().outs.push((pin, expr));
        self
    }
    fn bout(self, pin: &'static str, expr: &'static str) -> Self {
        self.pout(pin, expr).fout(pin, expr)
    }
    fn fimp(mut self, imp: &'static str) -> Self {
        self.fc().imports.push(imp);
        self
    }
    fn listener(mut self, class: &'static str, cancellable: bool) -> Self {
        let c = self.pc();
        c.listener = class;
        c.cancellable = cancellable;
        self
    }
    fn fcancel(mut self, ret: &'static str) -> Self {
        let c = self.fc();
        c.cancellable = true;
        c.ret = ret;
        self
    }
    /// Platform supported but without any code (e.g. value-only definitions).
    fn paper_empty(mut self) -> Self {
        self.pc();
        self
    }
    fn fabric_empty(mut self) -> Self {
        self.fc();
        self
    }
}

use FieldKind as F;
use PinType as T;

const GAMEMODES: &[(&str, L)] = &[
    ("SURVIVAL", ("Überleben", "Survival")),
    ("CREATIVE", ("Kreativ", "Creative")),
    ("ADVENTURE", ("Abenteuer", "Adventure")),
    ("SPECTATOR", ("Zuschauer", "Spectator")),
];

const SCOPES: &[(&str, L)] = &[
    ("player", ("Pro Spieler", "Per player")),
    ("global", ("Global", "Global")),
];

const MATH_OPS: &[(&str, L)] = &[
    ("+", ("+ Plus", "+ Add")),
    ("-", ("− Minus", "− Subtract")),
    ("*", ("× Mal", "× Multiply")),
    ("/", ("÷ Geteilt", "÷ Divide")),
    ("%", ("% Rest", "% Remainder")),
    ("pow", ("^ Hoch", "^ Power")),
    ("min", ("Minimum", "Minimum")),
    ("max", ("Maximum", "Maximum")),
];

const CMP_OPS: &[(&str, L)] = &[
    ("==", ("= gleich", "= equal")),
    ("!=", ("≠ ungleich", "≠ not equal")),
    ("<", ("< kleiner", "< less")),
    ("<=", ("≤ kleiner gleich", "≤ less or equal")),
    (">", ("> größer", "> greater")),
    (">=", ("≥ größer gleich", "≥ greater or equal")),
];

const ROUND_OPS: &[(&str, L)] = &[
    ("round", ("Runden", "Round")),
    ("floor", ("Abrunden", "Floor")),
    ("ceil", ("Aufrunden", "Ceil")),
];

const BOSS_COLORS: &[(&str, L)] = &[
    ("PINK", ("Rosa", "Pink")),
    ("BLUE", ("Blau", "Blue")),
    ("RED", ("Rot", "Red")),
    ("GREEN", ("Grün", "Green")),
    ("YELLOW", ("Gelb", "Yellow")),
    ("PURPLE", ("Lila", "Purple")),
    ("WHITE", ("Weiß", "White")),
];

const WEATHER: &[(&str, L)] = &[
    ("clear", ("Sonne", "Clear")),
    ("rain", ("Regen", "Rain")),
    ("thunder", ("Gewitter", "Thunder")),
];

const BAN_CATEGORIES: &[(&str, L)] = &[
    ("Hacking", ("Hacking / Cheats", "Hacking / cheats")),
    ("Beleidigung", ("Beleidigung", "Insults")),
    ("Spam", ("Spam", "Spam")),
    ("Werbung", ("Werbung", "Advertising")),
    ("Griefing", ("Griefing", "Griefing")),
    ("Bugusing", ("Bugusing", "Bug abuse")),
    ("Teaming", ("Teaming", "Teaming")),
    ("Skin / Name", ("Unangemessener Skin / Name", "Inappropriate skin / name")),
    ("Sicherheit", ("Account-Sicherheit", "Account security")),
    ("Sonstiges", ("Sonstiges", "Other")),
];

const BAN_UNITS: &[(&str, L)] = &[
    ("60", ("Minuten", "Minutes")),
    ("3600", ("Stunden", "Hours")),
    ("86400", ("Tage", "Days")),
    ("-1", ("Für immer", "Forever")),
];

const AREA_MODES: &[(&str, L)] = &[
    ("enter", ("Beim Betreten", "On entering")),
    ("leave", ("Beim Verlassen", "On leaving")),
];

const MENU_SCOPES: &[(&str, L)] = &[
    ("player", ("Diesen Spieler", "This player")),
    ("global", ("Alle Spieler", "All players")),
];

const MENU_MODES: &[(&str, L)] = &[
    ("all", ("Ganzes Menü", "Whole menu")),
    ("slots", ("Nur diese Plätze", "Only these slots")),
];

const EQUIP_SLOTS: &[(&str, L)] = &[
    ("HEAD", ("Kopf", "Head")),
    ("CHEST", ("Brust", "Chest")),
    ("LEGS", ("Beine", "Legs")),
    ("FEET", ("Füße", "Feet")),
    ("OFFHAND", ("Zweithand", "Off hand")),
    ("MAINHAND", ("Haupthand", "Main hand")),
];

const CASES: &[(&str, L)] = &[
    ("upper", ("GROSSBUCHSTABEN", "UPPERCASE")),
    ("lower", ("kleinbuchstaben", "lowercase")),
];

fn build_all() -> Vec<BlockDef> {
    use Cat as C;
    use Kind as K;
    let p_player = ("Spieler", "Player");
    let mut v: Vec<B> = vec![];

    // ------------------------------------------------------------------ Events
    v.push(
        b(
            "ev_start",
            C::Events,
            K::Event,
            ("Beim Start", "On start"),
            (
                "Läuft einmal, wenn das Plugin geladen wird bzw. der Server gestartet ist.",
                "Runs once when the plugin is enabled / the server has started.",
            ),
        )
        .special(Special::Start)
        .paper_empty()
        .fabric("ServerLifecycleEvents.SERVER_STARTED.register(__server -> {\n${body}\n});")
        .fimp("net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents"),
    );
    v.push(
        b("ev_join", C::Events, K::Event,
            ("Spieler betritt Server", "Player joins"),
            ("Wird ausgelöst, wenn ein Spieler den Server betritt.", "Fires when a player joins the server."))
            .o("player", p_player, T::Player)
            .listener("org.bukkit.event.player.PlayerJoinEvent", false)
            .pout("player", "event.getPlayer()")
            .fabric("ServerPlayConnectionEvents.JOIN.register((handler, sender, __server) -> {\n${body}\n});")
            .fout("player", "handler.getPlayer()")
            .fimp("net.fabricmc.fabric.api.networking.v1.ServerPlayConnectionEvents"),
    );
    v.push(
        b(
            "ev_quit",
            C::Events,
            K::Event,
            ("Spieler verlässt Server", "Player leaves"),
            (
                "Wird ausgelöst, wenn ein Spieler den Server verlässt.",
                "Fires when a player leaves the server.",
            ),
        )
        .o("player", p_player, T::Player)
        .listener("org.bukkit.event.player.PlayerQuitEvent", false)
        .pout("player", "event.getPlayer()")
        .fabric(
            "ServerPlayConnectionEvents.DISCONNECT.register((handler, __server) -> {\n${body}\n});",
        )
        .fout("player", "handler.getPlayer()")
        .fimp("net.fabricmc.fabric.api.networking.v1.ServerPlayConnectionEvents"),
    );
    v.push(
        b("ev_chat", C::Events, K::Event,
            ("Chatnachricht", "Chat message"),
            ("Wird ausgelöst, wenn ein Spieler im Chat schreibt. Kann abgebrochen werden. Paper: läuft asynchron – für Welt-Aktionen erst „Warten“ benutzen.",
             "Fires when a player chats. Can be cancelled. Paper: runs async – use “Wait” before world actions."))
            .o("player", p_player, T::Player)
            .o("message", ("Nachricht", "Message"), T::Text)
            .listener("io.papermc.paper.event.player.AsyncChatEvent", true)
            .pout("player", "event.getPlayer()")
            .pout("message", "net.kyori.adventure.text.serializer.plain.PlainTextComponentSerializer.plainText().serialize(event.message())")
            .fabric("ServerMessageEvents.ALLOW_CHAT_MESSAGE.register((message, sender, params) -> {\n${body}\n    return !__cancel[0];\n});")
            .fcancel("return !__cancel[0];")
            .fout("player", "sender")
            .fout("message", "message.signedContent()")
            .fimp("net.fabricmc.fabric.api.message.v1.ServerMessageEvents"),
    );
    v.push(
        b("ev_break", C::Events, K::Event,
            ("Block abgebaut", "Block broken"),
            ("Wird ausgelöst, wenn ein Spieler einen Block abbaut. Kann abgebrochen werden.",
             "Fires when a player breaks a block. Can be cancelled."))
            .o("player", p_player, T::Player)
            .o("block", ("Block-ID", "Block id"), T::Text)
            .o("loc", ("Position", "Location"), T::Location)
            .listener("org.bukkit.event.block.BlockBreakEvent", true)
            .pout("player", "event.getPlayer()")
            .pout("block", "event.getBlock().getType().getKey().toString()")
            .pout("loc", "event.getBlock().getLocation()")
            .fabric("PlayerBlockBreakEvents.BEFORE.register((level, __p, pos, state, blockEntity) -> {\n    if (!(__p instanceof ServerPlayer)) return true;\n${body}\n    return !__cancel[0];\n});")
            .fcancel("return !__cancel[0];")
            .fout("player", "(ServerPlayer) __p")
            .fout("block", "BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString()")
            .fout("loc", "new Loc((ServerLevel) level, pos.getX(), pos.getY(), pos.getZ())")
            .fimp("net.fabricmc.fabric.api.event.player.PlayerBlockBreakEvents"),
    );
    v.push(
        b("ev_place", C::Events, K::Event,
            ("Block platziert", "Block placed"),
            ("Wird ausgelöst, wenn ein Spieler einen Block setzt. Kann abgebrochen werden. Nur Paper.",
             "Fires when a player places a block. Can be cancelled. Paper only."))
            .o("player", p_player, T::Player)
            .o("block", ("Block-ID", "Block id"), T::Text)
            .o("loc", ("Position", "Location"), T::Location)
            .listener("org.bukkit.event.block.BlockPlaceEvent", true)
            .pout("player", "event.getPlayer()")
            .pout("block", "event.getBlockPlaced().getType().getKey().toString()")
            .pout("loc", "event.getBlockPlaced().getLocation()"),
    );
    v.push(
        b("ev_death", C::Events, K::Event,
            ("Spieler stirbt", "Player dies"),
            ("Wird ausgelöst, wenn ein Spieler stirbt.", "Fires when a player dies."))
            .o("player", p_player, T::Player)
            .listener("org.bukkit.event.entity.PlayerDeathEvent", false)
            .pout("player", "event.getPlayer()")
            .fabric("ServerLivingEntityEvents.AFTER_DEATH.register((__e, damageSource) -> {\n    if (!(__e instanceof ServerPlayer)) return;\n${body}\n});")
            .fout("player", "(ServerPlayer) __e")
            .fimp("net.fabricmc.fabric.api.entity.event.v1.ServerLivingEntityEvents"),
    );
    v.push(
        b("ev_damage", C::Events, K::Event,
            ("Spieler nimmt Schaden", "Player takes damage"),
            ("Wird ausgelöst, bevor ein Spieler Schaden bekommt. Kann abgebrochen werden.",
             "Fires before a player takes damage. Can be cancelled."))
            .o("player", p_player, T::Player)
            .o("damage", ("Schaden", "Damage"), T::Number)
            .listener("org.bukkit.event.entity.EntityDamageEvent", true)
            .paper("if (!(event.getEntity() instanceof org.bukkit.entity.Player)) return;")
            .pout("player", "(org.bukkit.entity.Player) event.getEntity()")
            .pout("damage", "event.getDamage()")
            .fabric("ServerLivingEntityEvents.ALLOW_DAMAGE.register((__e, source, amount) -> {\n    if (!(__e instanceof ServerPlayer)) return true;\n${body}\n    return !__cancel[0];\n});")
            .fcancel("return !__cancel[0];")
            .fout("player", "(ServerPlayer) __e")
            .fout("damage", "amount")
            .fimp("net.fabricmc.fabric.api.entity.event.v1.ServerLivingEntityEvents"),
    );
    v.push(
        b("ev_rightclick", C::Events, K::Event,
            ("Rechtsklick mit Item", "Right click with item"),
            ("Wird ausgelöst, wenn ein Spieler mit der Haupthand rechtsklickt. Kann abgebrochen werden.",
             "Fires when a player right-clicks with the main hand. Can be cancelled."))
            .o("player", p_player, T::Player)
            .o("item", ("Item-ID", "Item id"), T::Text)
            .listener("org.bukkit.event.player.PlayerInteractEvent", true)
            .paper("if (event.getHand() != org.bukkit.inventory.EquipmentSlot.HAND || !event.getAction().isRightClick()) return;")
            .pout("player", "event.getPlayer()")
            .pout("item", "event.getItem() == null ? \"minecraft:air\" : event.getItem().getType().getKey().toString()")
            .fabric("UseItemCallback.EVENT.register((__p, level, hand) -> {\n    if (level.isClientSide() || hand != InteractionHand.MAIN_HAND || !(__p instanceof ServerPlayer)) return InteractionResult.PASS;\n${body}\n    return __cancel[0] ? InteractionResult.FAIL : InteractionResult.PASS;\n});")
            .fcancel("return __cancel[0] ? InteractionResult.FAIL : InteractionResult.PASS;")
            .fout("player", "(ServerPlayer) __p")
            .fout("item", "BuiltInRegistries.ITEM.getKey(__p.getItemInHand(hand).getItem()).toString()")
            .fimp("net.fabricmc.fabric.api.event.player.UseItemCallback")
            .fimp("net.minecraft.world.InteractionHand")
            .fimp("net.minecraft.world.InteractionResult"),
    );
    v.push(
        b("ev_command", C::Events, K::Event,
            ("Befehl", "Command"),
            ("Läuft, wenn jemand den Befehl eingibt. Name, Optionen und Rechte stellst du im Befehle-Fenster ein; die Optionen sind die Ausgänge dieses Blocks.",
             "Runs when someone types the command. Name, options and permissions are set in the commands window; the options are this block's outputs."))
            .special(Special::Command)
            .o("player", p_player, T::Player)
            .paper_empty()
            .fabric_empty(),
    );
    v.push(
        b(
            "ev_timer",
            C::Events,
            K::Event,
            ("Timer", "Timer"),
            (
                "Läuft immer wieder alle X Sekunden, solange der Server läuft.",
                "Runs again and again every X seconds while the server is running.",
            ),
        )
        .special(Special::Timer)
        .f(
            "seconds",
            ("Alle Sekunden", "Every seconds"),
            F::Number,
            "60",
        )
        .paper_empty()
        .fabric_empty(),
    );
    v.push(
        b("cancel_event", C::Events, K::Action,
            ("Event abbrechen", "Cancel event"),
            ("Bricht das auslösende Event ab (z. B. Block wird nicht abgebaut, Nachricht wird nicht gesendet).",
             "Cancels the triggering event (e.g. the block is not broken, the message is not sent)."))
            .paper("event.setCancelled(true);")
            .fabric("__cancel[0] = true;"),
    );

    // --------------------------------------------------------------- Messages
    v.push(
        b("send_message", C::Messages, K::Action,
            ("Nachricht senden", "Send message"),
            ("Schickt einem Spieler eine Chatnachricht. Farbcodes mit & (z. B. &a grün, &l fett).",
             "Sends a chat message to a player. Color codes with & (e.g. &a green, &l bold)."))
            .i("player", p_player, T::Player, "")
            .i("text", ("Text", "Text"), T::Text, "&aHallo!")
            .paper("${in:player}.sendMessage(Util.color(${in:text}));")
            .fabric("${in:player}.sendSystemMessage(Util.color(${in:text}));"),
    );
    v.push(
        b(
            "broadcast",
            C::Messages,
            K::Action,
            ("An alle senden", "Broadcast"),
            (
                "Schickt eine Nachricht an alle Spieler auf dem Server.",
                "Sends a message to every player on the server.",
            ),
        )
        .i("text", ("Text", "Text"), T::Text, "&eHallo an alle!")
        .paper("org.bukkit.Bukkit.broadcast(Util.color(${in:text}));")
        .fabric("${server}.getPlayerList().broadcastSystemMessage(Util.color(${in:text}), false);"),
    );
    v.push(
        b(
            "title",
            C::Messages,
            K::Action,
            ("Titel anzeigen", "Show title"),
            (
                "Zeigt einen großen Titel und Untertitel in der Bildschirmmitte.",
                "Shows a big title and subtitle on screen.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i("title", ("Titel", "Title"), T::Text, "&6Willkommen")
        .ia("subtitle", ("Untertitel", "Subtitle"), T::Text, "")
        .ia("seconds", ("Sekunden", "Seconds"), T::Number, "3")
        .paper("Util.title(${in:player}, ${in:title}, ${in:subtitle}, ${in:seconds});")
        .fabric("Util.title(${in:player}, ${in:title}, ${in:subtitle}, ${in:seconds});"),
    );
    v.push(
        b(
            "actionbar",
            C::Messages,
            K::Action,
            ("Actionbar-Text", "Action bar text"),
            (
                "Zeigt einen kurzen Text über der Hotbar.",
                "Shows a short text above the hotbar.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i("text", ("Text", "Text"), T::Text, "&b+1 Punkt")
        .paper("${in:player}.sendActionBar(Util.color(${in:text}));")
        .fabric("${in:player}.sendSystemMessage(Util.color(${in:text}), true);"),
    );

    // ----------------------------------------------------------------- Player
    v.push(
        b(
            "teleport",
            C::Player,
            K::Action,
            ("Teleportieren", "Teleport"),
            (
                "Teleportiert einen Spieler oder ein Entity zu einer Position.",
                "Teleports a player or entity to a location.",
            ),
        )
        .i("entity", ("Wer", "Who"), T::Entity, "")
        .i("loc", ("Ziel", "Target"), T::Location, "")
        .paper("${in:entity}.teleport(${in:loc});")
        .fabric("Util.teleport(${in:entity}, ${in:loc});"),
    );
    v.push(
        b(
            "heal",
            C::Player,
            K::Action,
            ("Heilen", "Heal"),
            (
                "Setzt Leben und Hunger auf voll.",
                "Restores full health and hunger.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .both("Util.heal(${in:player});"),
    );
    v.push(
        b(
            "set_health",
            C::Player,
            K::Action,
            ("Leben setzen", "Set health"),
            (
                "Setzt die Lebenspunkte (20 = 10 Herzen).",
                "Sets health points (20 = 10 hearts).",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i("health", ("Leben", "Health"), T::Number, "20")
        .both("Util.setHealth(${in:player}, ${in:health});"),
    );
    v.push(
        b(
            "gamemode",
            C::Player,
            K::Action,
            ("Spielmodus setzen", "Set game mode"),
            (
                "Ändert den Spielmodus eines Spielers.",
                "Changes a player's game mode.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .f("mode", ("Modus", "Mode"), F::Choice(GAMEMODES), "CREATIVE")
        .paper("${in:player}.setGameMode(org.bukkit.GameMode.${f:mode});")
        .fabric("${in:player}.setGameMode(net.minecraft.world.level.GameType.${f:mode});"),
    );
    v.push(
        b(
            "give_item",
            C::Player,
            K::Action,
            ("Item geben", "Give item"),
            (
                "Gibt einem Spieler Items, z. B. minecraft:diamond.",
                "Gives a player items, e.g. minecraft:diamond.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i("item", ("Item-ID", "Item id"), T::Text, "minecraft:diamond")
        .i("amount", ("Anzahl", "Amount"), T::Number, "1")
        .both("Util.give(${in:player}, ${in:item}, ${in:amount});"),
    );
    v.push(
        b(
            "clear_inv",
            C::Player,
            K::Action,
            ("Inventar leeren", "Clear inventory"),
            (
                "Entfernt alle Items aus dem Inventar.",
                "Removes every item from the inventory.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .paper("${in:player}.getInventory().clear();")
        .fabric("${in:player}.getInventory().clearContent();"),
    );
    v.push(
        b(
            "effect",
            C::Player,
            K::Action,
            ("Effekt geben", "Give effect"),
            (
                "Gibt einen Trank-Effekt, z. B. speed, regeneration, night_vision.",
                "Gives a potion effect, e.g. speed, regeneration, night_vision.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i("effect", ("Effekt", "Effect"), T::Text, "speed")
        .i("seconds", ("Sekunden", "Seconds"), T::Number, "30")
        .ia("level", ("Stufe", "Level"), T::Number, "1")
        .both("Util.effect(${in:player}, ${in:effect}, ${in:seconds}, ${in:level});"),
    );
    v.push(
        b(
            "set_fire",
            C::Player,
            K::Action,
            ("Anzünden", "Set on fire"),
            (
                "Zündet ein Entity für einige Sekunden an.",
                "Sets an entity on fire for some seconds.",
            ),
        )
        .i("entity", ("Wer", "Who"), T::Entity, "")
        .i("seconds", ("Sekunden", "Seconds"), T::Number, "5")
        .paper("${in:entity}.setFireTicks((int) (${in:seconds} * 20));")
        .fabric("${in:entity}.igniteForSeconds((float) ${in:seconds});"),
    );
    v.push(
        b(
            "kill",
            C::Player,
            K::Action,
            ("Töten", "Kill"),
            ("Tötet einen Spieler sofort.", "Kills a player instantly."),
        )
        .i("player", p_player, T::Player, "")
        .paper("${in:player}.setHealth(0);")
        .fabric("${in:player}.kill(${in:player}.level());"),
    );
    v.push(
        b(
            "kick",
            C::Player,
            K::Action,
            ("Kicken", "Kick"),
            (
                "Wirft einen Spieler mit Begründung vom Server.",
                "Kicks a player from the server with a reason.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i(
            "reason",
            ("Grund", "Reason"),
            T::Text,
            "&cDu wurdest gekickt.",
        )
        .paper("${in:player}.kick(Util.color(${in:reason}));")
        .fabric("${in:player}.connection.disconnect(Util.color(${in:reason}));"),
    );
    v.push(
        b(
            "set_level",
            C::Player,
            K::Action,
            ("XP-Level setzen", "Set XP level"),
            ("Setzt das Erfahrungslevel.", "Sets the experience level."),
        )
        .i("player", p_player, T::Player, "")
        .i("level", ("Level", "Level"), T::Number, "10")
        .paper("${in:player}.setLevel((int) ${in:level});")
        .fabric("${in:player}.setExperienceLevels((int) ${in:level});"),
    );
    v.push(
        b(
            "player_cmd",
            C::Player,
            K::Action,
            ("Befehl als Spieler", "Run command as player"),
            (
                "Lässt den Spieler einen Befehl ausführen (ohne /).",
                "Makes the player run a command (without /).",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i("command", ("Befehl", "Command"), T::Text, "spawn")
        .paper("${in:player}.performCommand(${in:command});")
        .fabric("Util.runAs(${in:player}, ${in:command});"),
    );
    v.push(
        b(
            "sound",
            C::Player,
            K::Action,
            ("Sound abspielen", "Play sound"),
            (
                "Spielt einen Sound beim Spieler ab, z. B. entity.experience_orb.pickup.",
                "Plays a sound at the player, e.g. entity.experience_orb.pickup.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i(
            "sound",
            ("Sound", "Sound"),
            T::Text,
            "entity.experience_orb.pickup",
        )
        .ia("volume", ("Lautstärke", "Volume"), T::Number, "1")
        .ia("pitch", ("Tonhöhe", "Pitch"), T::Number, "1")
        .both("Util.sound(${in:player}, ${in:sound}, ${in:volume}, ${in:pitch});"),
    );
    v.push(
        b(
            "player_name",
            C::Player,
            K::Value,
            ("Spielername", "Player name"),
            ("Der Name des Spielers.", "The player's name."),
        )
        .i("player", p_player, T::Player, "")
        .o("name", ("Name", "Name"), T::Text)
        .bout("name", "Util.name(${in:player})"),
    );
    v.push(
        b(
            "player_info",
            C::Player,
            K::Value,
            ("Spieler-Infos", "Player info"),
            (
                "Leben, Hunger, Level und Position eines Spielers.",
                "Health, food, level and location of a player.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .o("health", ("Leben", "Health"), T::Number)
        .o("food", ("Hunger", "Food"), T::Number)
        .o("level", ("Level", "Level"), T::Number)
        .o("loc", ("Position", "Location"), T::Location)
        .o("world", ("Welt", "World"), T::Text)
        .pout("health", "${in:player}.getHealth()")
        .pout("food", "${in:player}.getFoodLevel()")
        .pout("level", "${in:player}.getLevel()")
        .pout("loc", "${in:player}.getLocation()")
        .pout("world", "${in:player}.getWorld().getName()")
        .fout("health", "${in:player}.getHealth()")
        .fout("food", "${in:player}.getFoodData().getFoodLevel()")
        .fout("level", "${in:player}.experienceLevel")
        .fout("loc", "Loc.of(${in:player})")
        .fout("world", "Util.worldName(${in:player}.level())"),
    );
    v.push(
        b(
            "held_item",
            C::Player,
            K::Value,
            ("Item in der Hand", "Held item"),
            (
                "Die ID des Items in der Haupthand.",
                "The id of the item in the main hand.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .o("item", ("Item-ID", "Item id"), T::Text)
        .pout(
            "item",
            "${in:player}.getInventory().getItemInMainHand().getType().getKey().toString()",
        )
        .fout(
            "item",
            "BuiltInRegistries.ITEM.getKey(${in:player}.getMainHandItem().getItem()).toString()",
        ),
    );
    v.push(
        b(
            "is_op",
            C::Player,
            K::Value,
            ("Ist OP", "Is OP"),
            (
                "Ja, wenn der Spieler Operator ist.",
                "Yes if the player is an operator.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .o("op", ("Ist OP", "Is OP"), T::Bool)
        .pout("op", "${in:player}.isOp()")
        .fout("op", "Util.isOp(${in:player})"),
    );
    v.push(
        b(
            "has_perm",
            C::Player,
            K::Value,
            ("Hat Berechtigung", "Has permission"),
            (
                "Prüft eine Berechtigung (z. B. von LuckPerms). Nur Paper.",
                "Checks a permission (e.g. from LuckPerms). Paper only.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i(
            "perm",
            ("Berechtigung", "Permission"),
            T::Text,
            "meinplugin.vip",
        )
        .o("has", ("Hat sie", "Has it"), T::Bool)
        .pout("has", "${in:player}.hasPermission(${in:perm})"),
    );
    v.push(
        b(
            "player_by_name",
            C::Player,
            K::Value,
            ("Spieler nach Name", "Player by name"),
            (
                "Sucht einen Online-Spieler nach Namen. Mit „Spieler gefunden?“ prüfen.",
                "Finds an online player by name. Check it with “Player found?”.",
            ),
        )
        .i("name", ("Name", "Name"), T::Text, "")
        .o("player", p_player, T::Player)
        .pout("player", "org.bukkit.Bukkit.getPlayerExact(${in:name})")
        .fout(
            "player",
            "${server}.getPlayerList().getPlayerByName(${in:name})",
        ),
    );
    v.push(
        b(
            "player_exists",
            C::Player,
            K::Value,
            ("Spieler gefunden?", "Player found?"),
            (
                "Ja, wenn der Spieler existiert (z. B. bei Befehlen von der Konsole ist er leer).",
                "Yes if the player exists (e.g. empty when a command comes from the console).",
            ),
        )
        .i("player", p_player, T::Player, "")
        .o("ok", ("Gefunden", "Found"), T::Bool)
        .bout("ok", "(${in:player} != null)"),
    );

    // ------------------------------------------------------------------ World
    v.push(
        b(
            "make_loc",
            C::World,
            K::Value,
            ("Position erstellen", "Make location"),
            (
                "Erstellt eine Position aus Welt und Koordinaten.",
                "Builds a location from world and coordinates.",
            ),
        )
        .i("world", ("Welt", "World"), T::Text, "world")
        .i("x", ("X", "X"), T::Number, "0")
        .i("y", ("Y", "Y"), T::Number, "64")
        .i("z", ("Z", "Z"), T::Number, "0")
        .o("loc", ("Position", "Location"), T::Location)
        .bout("loc", "Util.loc(${in:world}, ${in:x}, ${in:y}, ${in:z})"),
    );
    v.push(
        b(
            "loc_parts",
            C::World,
            K::Value,
            ("Position zerlegen", "Split location"),
            (
                "Gibt X, Y, Z und Welt einer Position aus.",
                "Outputs X, Y, Z and world of a location.",
            ),
        )
        .i("loc", ("Position", "Location"), T::Location, "")
        .o("x", ("X", "X"), T::Number)
        .o("y", ("Y", "Y"), T::Number)
        .o("z", ("Z", "Z"), T::Number)
        .o("world", ("Welt", "World"), T::Text)
        .pout("x", "${in:loc}.getX()")
        .pout("y", "${in:loc}.getY()")
        .pout("z", "${in:loc}.getZ()")
        .pout("world", "${in:loc}.getWorld().getName()")
        .fout("x", "${in:loc}.x()")
        .fout("y", "${in:loc}.y()")
        .fout("z", "${in:loc}.z()")
        .fout("world", "Util.worldName(${in:loc}.level())"),
    );
    v.push(
        b(
            "offset_loc",
            C::World,
            K::Value,
            ("Position verschieben", "Offset location"),
            (
                "Addiert X/Y/Z zu einer Position.",
                "Adds X/Y/Z to a location.",
            ),
        )
        .i("loc", ("Position", "Location"), T::Location, "")
        .i("x", ("+X", "+X"), T::Number, "0")
        .i("y", ("+Y", "+Y"), T::Number, "1")
        .i("z", ("+Z", "+Z"), T::Number, "0")
        .o("loc", ("Position", "Location"), T::Location)
        .pout("loc", "${in:loc}.clone().add(${in:x}, ${in:y}, ${in:z})")
        .fout("loc", "${in:loc}.add(${in:x}, ${in:y}, ${in:z})"),
    );
    v.push(
        b(
            "set_block",
            C::World,
            K::Action,
            ("Block setzen", "Set block"),
            (
                "Setzt einen Block an eine Position, z. B. minecraft:gold_block.",
                "Places a block at a location, e.g. minecraft:gold_block.",
            ),
        )
        .i("loc", ("Position", "Location"), T::Location, "")
        .i(
            "block",
            ("Block-ID", "Block id"),
            T::Text,
            "minecraft:gold_block",
        )
        .both("Util.setBlock(${in:loc}, ${in:block});"),
    );
    v.push(
        b(
            "get_block",
            C::World,
            K::Value,
            ("Block an Position", "Block at location"),
            (
                "Die Block-ID an einer Position.",
                "The block id at a location.",
            ),
        )
        .i("loc", ("Position", "Location"), T::Location, "")
        .o("block", ("Block-ID", "Block id"), T::Text)
        .bout("block", "Util.blockAt(${in:loc})"),
    );
    v.push(
        b(
            "lightning",
            C::World,
            K::Action,
            ("Blitz", "Lightning"),
            ("Lässt einen Blitz einschlagen.", "Strikes lightning."),
        )
        .i("loc", ("Position", "Location"), T::Location, "")
        .both("Util.lightning(${in:loc});"),
    );
    v.push(
        b(
            "explosion",
            C::World,
            K::Action,
            ("Explosion", "Explosion"),
            (
                "Erzeugt eine Explosion (4 = TNT).",
                "Creates an explosion (4 = TNT).",
            ),
        )
        .i("loc", ("Position", "Location"), T::Location, "")
        .i("power", ("Stärke", "Power"), T::Number, "4")
        .both("Util.explode(${in:loc}, ${in:power});"),
    );
    v.push(
        b(
            "spawn_entity",
            C::World,
            K::Action,
            ("Entity spawnen", "Spawn entity"),
            (
                "Spawnt ein Mob, z. B. minecraft:zombie oder minecraft:cow.",
                "Spawns a mob, e.g. minecraft:zombie or minecraft:cow.",
            ),
        )
        .i("loc", ("Position", "Location"), T::Location, "")
        .i("type", ("Typ", "Type"), T::Text, "minecraft:zombie")
        .both("Util.spawn(${in:loc}, ${in:type});"),
    );
    v.push(
        b(
            "set_time",
            C::World,
            K::Action,
            ("Uhrzeit setzen", "Set time"),
            (
                "Setzt die Zeit in der Welt (0 = Morgen, 6000 = Mittag, 13000 = Nacht).",
                "Sets the world time (0 = morning, 6000 = noon, 13000 = night).",
            ),
        )
        .i("world", ("Welt", "World"), T::Text, "world")
        .i("time", ("Zeit", "Time"), T::Number, "6000")
        .both("Util.setTime(${in:world}, ${in:time});"),
    );

    // ------------------------------------------------------------------- Flow
    v.push(
        b(
            "if",
            C::Flow,
            K::Flow,
            ("Wenn", "If"),
            (
                "Führt „Dann“ aus, wenn die Bedingung stimmt, sonst „Sonst“.",
                "Runs “Then” if the condition is true, otherwise “Else”.",
            ),
        )
        .i("cond", ("Bedingung", "Condition"), T::Bool, "true")
        .o("then", ("Dann", "Then"), T::Exec)
        .o("else", ("Sonst", "Else"), T::Exec)
        .both("if (${in:cond}) {\n${exec:then}\n} else {\n${exec:else}\n}"),
    );
    v.push(
        b(
            "chance",
            C::Flow,
            K::Flow,
            ("Zufall (Chance)", "Random chance"),
            (
                "Führt „Treffer“ mit X Prozent Wahrscheinlichkeit aus.",
                "Runs “Hit” with X percent probability.",
            ),
        )
        .i("percent", ("Prozent", "Percent"), T::Number, "50")
        .o("then", ("Treffer", "Hit"), T::Exec)
        .o("else", ("Kein Treffer", "Miss"), T::Exec)
        .both(
            "if (Math.random() * 100 < ${in:percent}) {\n${exec:then}\n} else {\n${exec:else}\n}",
        ),
    );
    v.push(
        b("repeat", C::Flow, K::Flow,
            ("Wiederholen", "Repeat"),
            ("Führt „Schleife“ X-mal aus, danach „Fertig“.", "Runs “Loop” X times, then “Done”."))
            .i("times", ("Anzahl", "Times"), T::Number, "3")
            .o("loop", ("Schleife", "Loop"), T::Exec)
            .o("index", ("Durchlauf", "Index"), T::Number)
            .o("next", ("Fertig", "Done"), T::Exec)
            .both("for (int __i${id} = 0; __i${id} < (int) ${in:times}; __i${id}++) {\n    final double ${out:index} = __i${id} + 1;\n${exec:loop}\n}"),
    );
    v.push(
        b("for_players", C::Flow, K::Flow,
            ("Für jeden Spieler", "For each player"),
            ("Führt „Schleife“ für jeden Online-Spieler aus.", "Runs “Loop” for every online player."))
            .o("loop", ("Schleife", "Loop"), T::Exec)
            .o("player", p_player, T::Player)
            .o("next", ("Fertig", "Done"), T::Exec)
            .paper("for (final org.bukkit.entity.Player ${out:player} : org.bukkit.Bukkit.getOnlinePlayers()) {\n${exec:loop}\n}")
            .fabric("for (final ServerPlayer ${out:player} : new java.util.ArrayList<>(${server}.getPlayerList().getPlayers())) {\n${exec:loop}\n}"),
    );
    v.push(
        b("wait", C::Flow, K::Flow,
            ("Warten", "Wait"),
            ("Wartet X Sekunden, bevor es weitergeht (blockiert den Server nicht).", "Waits X seconds before continuing (does not block the server)."))
            .i("seconds", ("Sekunden", "Seconds"), T::Number, "1")
            .o("next", ("Danach", "Then"), T::Exec)
            .paper("org.bukkit.Bukkit.getScheduler().runTaskLater(${plugin}, () -> {\n${exec:next}\n}, Math.max(1L, (long) (${in:seconds} * 20)));")
            .fabric("Scheduler.later(Math.max(1, (int) (${in:seconds} * 20)), () -> {\n${exec:next}\n});"),
    );
    v.push(
        b(
            "stop",
            C::Flow,
            K::Flow,
            ("Stopp", "Stop"),
            (
                "Beendet den aktuellen Ablauf sofort.",
                "Ends the current flow immediately.",
            ),
        )
        .both("if (true) ${return}"),
    );

    // ------------------------------------------------------------------ Logic
    v.push(
        b(
            "compare",
            C::Logic,
            K::Value,
            ("Zahlen vergleichen", "Compare numbers"),
            ("Vergleicht zwei Zahlen.", "Compares two numbers."),
        )
        .i("a", ("A", "A"), T::Number, "0")
        .f("op", ("Vergleich", "Comparison"), F::Choice(CMP_OPS), ">=")
        .i("b", ("B", "B"), T::Number, "10")
        .o("res", ("Ergebnis", "Result"), T::Bool)
        .bout("res", "(${in:a} ${f:op} ${in:b})"),
    );
    v.push(
        b(
            "text_equals",
            C::Logic,
            K::Value,
            ("Text gleich", "Text equals"),
            (
                "Ja, wenn beide Texte gleich sind.",
                "Yes if both texts are equal.",
            ),
        )
        .i("a", ("A", "A"), T::Text, "")
        .i("b", ("B", "B"), T::Text, "")
        .f(
            "ignore_case",
            ("Groß/klein egal", "Ignore case"),
            F::Bool,
            "true",
        )
        .o("res", ("Ergebnis", "Result"), T::Bool)
        .bout("res", "Util.textEquals(${in:a}, ${in:b}, ${f:ignore_case})"),
    );
    v.push(
        b(
            "text_contains",
            C::Logic,
            K::Value,
            ("Text enthält", "Text contains"),
            (
                "Ja, wenn der Text das Stück enthält.",
                "Yes if the text contains the part.",
            ),
        )
        .i("text", ("Text", "Text"), T::Text, "")
        .i("part", ("Stück", "Part"), T::Text, "")
        .o("res", ("Ergebnis", "Result"), T::Bool)
        .bout(
            "res",
            "${in:text}.toLowerCase().contains(${in:part}.toLowerCase())",
        ),
    );
    v.push(
        b(
            "and",
            C::Logic,
            K::Value,
            ("Und", "And"),
            ("Ja, wenn beide stimmen.", "Yes if both are true."),
        )
        .i("a", ("A", "A"), T::Bool, "true")
        .i("b", ("B", "B"), T::Bool, "true")
        .o("res", ("Ergebnis", "Result"), T::Bool)
        .bout("res", "(${in:a} && ${in:b})"),
    );
    v.push(
        b(
            "or",
            C::Logic,
            K::Value,
            ("Oder", "Or"),
            (
                "Ja, wenn mindestens eins stimmt.",
                "Yes if at least one is true.",
            ),
        )
        .i("a", ("A", "A"), T::Bool, "false")
        .i("b", ("B", "B"), T::Bool, "false")
        .o("res", ("Ergebnis", "Result"), T::Bool)
        .bout("res", "(${in:a} || ${in:b})"),
    );
    v.push(
        b(
            "not",
            C::Logic,
            K::Value,
            ("Nicht", "Not"),
            ("Dreht Ja und Nein um.", "Flips yes and no."),
        )
        .i("a", ("Wert", "Value"), T::Bool, "false")
        .o("res", ("Ergebnis", "Result"), T::Bool)
        .bout("res", "(!${in:a})"),
    );

    // ------------------------------------------------------------------- Math
    v.push(
        b(
            "number",
            C::Math,
            K::Value,
            ("Zahl", "Number"),
            ("Eine feste Zahl.", "A fixed number."),
        )
        .f("value", ("Wert", "Value"), F::Number, "1")
        .o("value", ("Zahl", "Number"), T::Number)
        .bout("value", "${f:value}"),
    );
    v.push(
        b(
            "math",
            C::Math,
            K::Value,
            ("Rechnen", "Calculate"),
            ("Rechnet mit zwei Zahlen.", "Calculates with two numbers."),
        )
        .i("a", ("A", "A"), T::Number, "0")
        .f("op", ("Rechenart", "Operation"), F::Choice(MATH_OPS), "+")
        .i("b", ("B", "B"), T::Number, "1")
        .o("res", ("Ergebnis", "Result"), T::Number)
        .bout("res", "Util.math(${in:a}, \"${f:op}\", ${in:b})"),
    );
    v.push(
        b(
            "random",
            C::Math,
            K::Value,
            ("Zufallszahl", "Random number"),
            (
                "Eine zufällige ganze Zahl von Min bis Max.",
                "A random whole number from min to max.",
            ),
        )
        .i("min", ("Min", "Min"), T::Number, "1")
        .i("max", ("Max", "Max"), T::Number, "10")
        .o("res", ("Zahl", "Number"), T::Number)
        .bout("res", "Util.random(${in:min}, ${in:max})"),
    );
    v.push(
        b(
            "round",
            C::Math,
            K::Value,
            ("Runden", "Round"),
            ("Rundet eine Zahl.", "Rounds a number."),
        )
        .i("a", ("Zahl", "Number"), T::Number, "0")
        .f("mode", ("Art", "Mode"), F::Choice(ROUND_OPS), "round")
        .o("res", ("Ergebnis", "Result"), T::Number)
        .bout("res", "(double) Math.${f:mode}(${in:a})"),
    );

    // ------------------------------------------------------------------- Text
    v.push(
        b(
            "text",
            C::Text,
            K::Value,
            ("Text", "Text"),
            ("Ein fester Text.", "A fixed text."),
        )
        .f("value", ("Text", "Text"), F::Multiline, "Hallo")
        .o("value", ("Text", "Text"), T::Text)
        .bout("value", "${s:value}"),
    );
    v.push(
        b(
            "join",
            C::Text,
            K::Value,
            ("Text verbinden", "Join text"),
            (
                "Hängt bis zu drei Texte aneinander.",
                "Joins up to three texts.",
            ),
        )
        .i("a", ("A", "A"), T::Text, "")
        .i("b", ("B", "B"), T::Text, "")
        .i("c", ("C", "C"), T::Text, "")
        .o("res", ("Text", "Text"), T::Text)
        .bout("res", "(${in:a} + ${in:b} + ${in:c})"),
    );
    v.push(
        b(
            "format",
            C::Text,
            K::Value,
            ("Text mit Platzhaltern", "Text with placeholders"),
            (
                "Ersetzt {0}, {1}, {2}, {3} im Text durch die Eingänge.",
                "Replaces {0}, {1}, {2}, {3} in the text with the inputs.",
            ),
        )
        .f(
            "format",
            ("Vorlage", "Template"),
            F::Multiline,
            "&a{0} hat {1} Punkte!",
        )
        .i("p0", ("{0}", "{0}"), T::Text, "")
        .i("p1", ("{1}", "{1}"), T::Text, "")
        .i("p2", ("{2}", "{2}"), T::Text, "")
        .i("p3", ("{3}", "{3}"), T::Text, "")
        .o("res", ("Text", "Text"), T::Text)
        .bout(
            "res",
            "Util.format(${s:format}, ${in:p0}, ${in:p1}, ${in:p2}, ${in:p3})",
        ),
    );
    v.push(
        b(
            "to_number",
            C::Text,
            K::Value,
            ("Text → Zahl", "Text → number"),
            (
                "Wandelt Text in eine Zahl um (ungültig = 0).",
                "Converts text to a number (invalid = 0).",
            ),
        )
        .i("text", ("Text", "Text"), T::Text, "0")
        .o("res", ("Zahl", "Number"), T::Number)
        .bout("res", "Util.num(${in:text})"),
    );
    v.push(
        b(
            "is_number",
            C::Text,
            K::Value,
            ("Ist Zahl?", "Is number?"),
            (
                "Ja, wenn der Text eine gültige Zahl ist.",
                "Yes if the text is a valid number.",
            ),
        )
        .i("text", ("Text", "Text"), T::Text, "")
        .o("res", ("Ergebnis", "Result"), T::Bool)
        .bout("res", "Util.isNum(${in:text})"),
    );

    // ------------------------------------------------------------------- Data
    v.push(
        b(
            "data_set",
            C::Data,
            K::Action,
            ("Zahl speichern", "Store number"),
            (
                "Speichert eine Zahl dauerhaft (bleibt nach Neustart erhalten).",
                "Stores a number permanently (survives restarts).",
            ),
        )
        .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
        .i("player", p_player, T::Player, "")
        .i("key", ("Name", "Key"), T::Text, "punkte")
        .i("value", ("Wert", "Value"), T::Number, "0")
        .both("Storage.setNum(${storekey}, ${in:value});"),
    );
    v.push(
        b(
            "data_add",
            C::Data,
            K::Action,
            ("Gespeicherte Zahl erhöhen", "Increase stored number"),
            (
                "Erhöht eine gespeicherte Zahl und gibt den neuen Wert aus.",
                "Increases a stored number and outputs the new value.",
            ),
        )
        .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
        .i("player", p_player, T::Player, "")
        .i("key", ("Name", "Key"), T::Text, "punkte")
        .i("by", ("Um", "By"), T::Number, "1")
        .o("value", ("Neuer Wert", "New value"), T::Number)
        .both("final double ${out:value} = Storage.addNum(${storekey}, ${in:by});"),
    );
    v.push(
        b(
            "data_get",
            C::Data,
            K::Value,
            ("Gespeicherte Zahl", "Stored number"),
            (
                "Liest eine gespeicherte Zahl (0, wenn noch nichts gespeichert).",
                "Reads a stored number (0 if nothing stored yet).",
            ),
        )
        .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
        .i("player", p_player, T::Player, "")
        .i("key", ("Name", "Key"), T::Text, "punkte")
        .o("value", ("Wert", "Value"), T::Number)
        .bout("value", "Storage.getNum(${storekey})"),
    );
    v.push(
        b(
            "data_set_text",
            C::Data,
            K::Action,
            ("Text speichern", "Store text"),
            (
                "Speichert einen Text dauerhaft.",
                "Stores a text permanently.",
            ),
        )
        .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
        .i("player", p_player, T::Player, "")
        .i("key", ("Name", "Key"), T::Text, "home")
        .i("value", ("Text", "Text"), T::Text, "")
        .both("Storage.setText(${storekey}, ${in:value});"),
    );
    v.push(
        b(
            "data_get_text",
            C::Data,
            K::Value,
            ("Gespeicherter Text", "Stored text"),
            (
                "Liest einen gespeicherten Text (leer, wenn noch nichts gespeichert).",
                "Reads a stored text (empty if nothing stored yet).",
            ),
        )
        .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
        .i("player", p_player, T::Player, "")
        .i("key", ("Name", "Key"), T::Text, "home")
        .o("value", ("Text", "Text"), T::Text)
        .bout("value", "Storage.getText(${storekey})"),
    );

    // ----------------------------------------------------------------- Server
    v.push(
        b("console_cmd", C::Server, K::Action,
            ("Konsolenbefehl", "Console command"),
            ("Führt einen Befehl als Konsole aus (ohne /).", "Runs a command as the console (without /)."))
            .i("command", ("Befehl", "Command"), T::Text, "say Hallo")
            .paper("org.bukkit.Bukkit.dispatchCommand(org.bukkit.Bukkit.getConsoleSender(), ${in:command});")
            .fabric("Util.runConsole(${in:command});"),
    );
    v.push(
        b(
            "log",
            C::Server,
            K::Action,
            ("In Konsole schreiben", "Log to console"),
            (
                "Schreibt eine Zeile ins Server-Log.",
                "Writes a line into the server log.",
            ),
        )
        .i("text", ("Text", "Text"), T::Text, "Hallo Log!")
        .paper("${plugin}.getLogger().info(${in:text});")
        .fabric("${main}.LOGGER.info(${in:text});"),
    );
    v.push(
        b(
            "online_count",
            C::Server,
            K::Value,
            ("Spieler online", "Players online"),
            (
                "Wie viele Spieler gerade online sind.",
                "How many players are online right now.",
            ),
        )
        .o("count", ("Anzahl", "Count"), T::Number)
        .pout("count", "org.bukkit.Bukkit.getOnlinePlayers().size()")
        .fout("count", "${server}.getPlayerCount()"),
    );

    // -------------------------------------------------------------- Resources
    v.push(
        b("custom_item", C::Resources, K::Action,
            ("Eigenes Item geben", "Give custom item"),
            ("Gibt ein Item mit eigenem Modell aus deinem Resourcepack, eigenem Namen und Beschreibung. Mehrere Beschreibungszeilen mit | trennen.",
             "Gives an item with a custom model from your resource pack, custom name and lore. Separate lore lines with |."))
            .f("model", ("Modell", "Model"), F::ItemModel, "")
            .i("player", p_player, T::Player, "")
            .i("item", ("Basis-Item", "Base item"), T::Text, "minecraft:stick")
            .i("name", ("Name", "Name"), T::Text, "&6Zauberstab")
            .ia("lore", ("Beschreibung", "Lore"), T::Text, "&7Ein magischer Stab")
            .ia("amount", ("Anzahl", "Amount"), T::Number, "1")
            .both("Util.giveCustom(${in:player}, ${in:item}, ${in:amount}, ${s:model}, ${in:name}, ${in:lore});"),
    );
    v.push(
        b("held_model", C::Resources, K::Value,
            ("Modell in der Hand", "Model in hand"),
            ("Die Modell-ID des Items in der Haupthand, z. B. meinpack:zauberstab. Gut mit „Rechtsklick mit Item“ und „Text gleich“.",
             "The model id of the item in the main hand, e.g. mypack:wand. Works well with “Right click with item” and “Text equals”."))
            .i("player", p_player, T::Player, "")
            .o("model", ("Modell", "Model"), T::Text)
            .bout("model", "Util.heldModel(${in:player})"),
    );
    v.push(
        b(
            "is_model",
            C::Resources,
            K::Value,
            ("Hält Modell?", "Holds model?"),
            (
                "Ja, wenn der Spieler ein Item mit diesem Modell in der Hand hält.",
                "Yes if the player holds an item with this model.",
            ),
        )
        .f("model", ("Modell", "Model"), F::ItemModel, "")
        .i("player", p_player, T::Player, "")
        .o("res", ("Ergebnis", "Result"), T::Bool)
        .bout(
            "res",
            "Util.heldModel(${in:player}).equalsIgnoreCase(${s:model})",
        ),
    );
    v.push(
        b("send_pack", C::Resources, K::Action,
            ("Resourcepack senden", "Send resource pack"),
            ("Schickt dem Spieler das Resourcepack des Plugins (passiert sonst automatisch beim Betreten). Nur Paper.",
             "Sends the plugin's resource pack to the player (otherwise happens automatically on join). Paper only."))
            .i("player", p_player, T::Player, "")
            .paper("ResourcePack.send(${in:player});"),
    );
    v.push(
        b(
            "particles",
            C::World,
            K::Action,
            ("Partikel", "Particles"),
            (
                "Erzeugt Partikel an einer Position, z. B. heart, flame, happy_villager, cloud.",
                "Spawns particles at a location, e.g. heart, flame, happy_villager, cloud.",
            ),
        )
        .i("loc", ("Position", "Location"), T::Location, "")
        .i("type", ("Typ", "Type"), T::Text, "heart")
        .i("count", ("Anzahl", "Count"), T::Number, "10")
        .both("Util.particles(${in:loc}, ${in:type}, ${in:count});"),
    );

    // ------------------------------------------------------------ More blocks
    v.push(
        b("ev_kill", C::Events, K::Event,
            ("Spieler tötet Mob", "Player kills mob"),
            ("Wird ausgelöst, wenn ein Spieler ein Tier oder Monster tötet.", "Fires when a player kills an animal or monster."))
            .o("player", p_player, T::Player)
            .o("mob", ("Mob-Typ", "Mob type"), T::Text)
            .o("loc", ("Position", "Location"), T::Location)
            .listener("org.bukkit.event.entity.EntityDeathEvent", false)
            .paper("if (event.getEntity().getKiller() == null) return;")
            .pout("player", "event.getEntity().getKiller()")
            .pout("mob", "event.getEntity().getType().getKey().toString()")
            .pout("loc", "event.getEntity().getLocation()")
            .fabric("ServerEntityCombatEvents.AFTER_KILLED_OTHER_ENTITY.register((__level, __killer, __killed, __src) -> {\n    if (!(__killer instanceof ServerPlayer)) return;\n${body}\n});")
            .fout("player", "(ServerPlayer) __killer")
            .fout("mob", "BuiltInRegistries.ENTITY_TYPE.getKey(__killed.getType()).toString()")
            .fout("loc", "Loc.of(__killed)")
            .fimp("net.fabricmc.fabric.api.entity.event.v1.ServerEntityCombatEvents"),
    );
    v.push(
        b("cooldown", C::Flow, K::Flow,
            ("Cooldown", "Cooldown"),
            ("Lässt den Ablauf pro Spieler nur alle X Sekunden durch. Sonst geht es bei „Noch warten“ weiter (z. B. mit einer Nachricht).",
             "Lets the flow through only every X seconds per player. Otherwise it continues at “Still waiting” (e.g. with a message)."))
            .i("player", p_player, T::Player, "")
            .i("seconds", ("Sekunden", "Seconds"), T::Number, "10")
            .ia("key", ("Name", "Key"), T::Text, "standard")
            .o("ready", ("Bereit", "Ready"), T::Exec)
            .o("wait", ("Noch warten", "Still waiting"), T::Exec)
            .o("left", ("Rest-Sekunden", "Seconds left"), T::Number)
            .both("final double ${out:left} = Util.cooldown(Util.uuid(${in:player}) + \":\" + ${in:key}, ${in:seconds});\nif (${out:left} <= 0) {\n${exec:ready}\n} else {\n${exec:wait}\n}"),
    );
    v.push(
        b(
            "bossbar",
            C::Messages,
            K::Action,
            ("Bossbar anzeigen", "Show boss bar"),
            (
                "Zeigt oben eine Leiste mit Text, z. B. für Countdowns oder Fortschritt.",
                "Shows a bar at the top with text, e.g. for countdowns or progress.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i("text", ("Text", "Text"), T::Text, "&6Event startet bald!")
        .i("percent", ("Füllung %", "Fill %"), T::Number, "100")
        .ia("seconds", ("Sekunden", "Seconds"), T::Number, "5")
        .f(
            "color",
            ("Farbe", "Color"),
            F::Choice(BOSS_COLORS),
            "YELLOW",
        )
        .both(
            "Util.bossbar(${in:player}, ${in:text}, ${in:percent}, ${in:seconds}, \"${f:color}\");",
        ),
    );
    v.push(
        b("tablist", C::Messages, K::Action,
            ("Tab-Liste beschriften", "Set tab list text"),
            ("Setzt den Text über und unter der Spielerliste (Tab-Taste).", "Sets the text above and below the player list (Tab key)."))
            .i("player", p_player, T::Player, "")
            .i("header", ("Oben", "Header"), T::Text, "&6Mein Server")
            .i("footer", ("Unten", "Footer"), T::Text, "&7Viel Spaß!")
            .paper("${in:player}.sendPlayerListHeaderAndFooter(Util.color(${in:header}), Util.color(${in:footer}));")
            .fabric("${in:player}.connection.send(new net.minecraft.network.protocol.game.ClientboundTabListPacket(Util.color(${in:header}), Util.color(${in:footer})));"),
    );
    v.push(
        b("sidebar", C::Messages, K::Action,
            ("Seitenleiste (Scoreboard)", "Sidebar (scoreboard)"),
            ("Zeigt rechts eine Liste mit Titel. Zeilen mit | trennen, z. B. „&7Punkte: &e5|&7Online: &a3“. Nur Paper.",
             "Shows a list with a title on the right. Separate lines with |, e.g. “&7Points: &e5|&7Online: &a3”. Paper only."))
            .i("player", p_player, T::Player, "")
            .i("title", ("Titel", "Title"), T::Text, "&6&lMein Server")
            .i("lines", ("Zeilen", "Lines"), T::Text, "&7Willkommen!|&7Online: &a1")
            .paper("Util.sidebar(${in:player}, ${in:title}, ${in:lines});"),
    );
    v.push(
        b("sidebar_hide", C::Messages, K::Action,
            ("Seitenleiste entfernen", "Remove sidebar"),
            ("Entfernt die Seitenleiste wieder. Nur Paper.", "Removes the sidebar again. Paper only."))
            .i("player", p_player, T::Player, "")
            .paper("${in:player}.setScoreboard(org.bukkit.Bukkit.getScoreboardManager().getMainScoreboard());"),
    );
    v.push(
        b(
            "fly",
            C::Player,
            K::Action,
            ("Fliegen erlauben", "Allow flying"),
            (
                "Erlaubt oder verbietet das Fliegen (wie im Kreativmodus).",
                "Allows or forbids flying (like in creative mode).",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i("allow", ("Erlaubt", "Allowed"), T::Bool, "true")
        .both("Util.setFly(${in:player}, ${in:allow});"),
    );
    v.push(
        b(
            "count_items",
            C::Player,
            K::Value,
            ("Anzahl Items", "Item count"),
            (
                "Wie viele Items dieser Art der Spieler im Inventar hat.",
                "How many items of this kind the player has.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .i("item", ("Item-ID", "Item id"), T::Text, "minecraft:diamond")
        .o("count", ("Anzahl", "Count"), T::Number)
        .bout("count", "Util.countItems(${in:player}, ${in:item})"),
    );
    v.push(
        b("take_items", C::Player, K::Action,
            ("Items abziehen", "Take items"),
            ("Nimmt dem Spieler Items weg – nur wenn er genug hat. „Geklappt“ sagt, ob es funktioniert hat (gut für Shops).",
             "Takes items from the player – only if they have enough. “Worked” tells whether it worked (good for shops)."))
            .i("player", p_player, T::Player, "")
            .i("item", ("Item-ID", "Item id"), T::Text, "minecraft:diamond")
            .i("amount", ("Anzahl", "Amount"), T::Number, "1")
            .o("ok", ("Geklappt", "Worked"), T::Bool)
            .both("final boolean ${out:ok} = Util.takeItems(${in:player}, ${in:item}, ${in:amount});"),
    );
    v.push(
        b(
            "weather",
            C::World,
            K::Action,
            ("Wetter setzen", "Set weather"),
            (
                "Ändert das Wetter in einer Welt.",
                "Changes the weather in a world.",
            ),
        )
        .i("world", ("Welt", "World"), T::Text, "world")
        .f("type", ("Wetter", "Weather"), F::Choice(WEATHER), "clear")
        .both("Util.weather(${in:world}, \"${f:type}\");"),
    );
    v.push(
        b(
            "random_player",
            C::Server,
            K::Value,
            ("Zufälliger Spieler", "Random player"),
            (
                "Ein zufälliger Online-Spieler (leer, wenn niemand online ist).",
                "A random online player (empty if nobody is online).",
            ),
        )
        .o("player", p_player, T::Player)
        .bout("player", "Util.randomPlayer()"),
    );
    v.push(
        b(
            "distance",
            C::World,
            K::Value,
            ("Entfernung", "Distance"),
            (
                "Abstand zwischen zwei Positionen in Blöcken (-1 bei verschiedenen Welten).",
                "Distance between two locations in blocks (-1 for different worlds).",
            ),
        )
        .i("a", ("Von", "From"), T::Location, "")
        .i("b", ("Nach", "To"), T::Location, "")
        .o("dist", ("Blöcke", "Blocks"), T::Number)
        .bout("dist", "Util.distance(${in:a}, ${in:b})"),
    );
    v.push(
        b(
            "text_replace",
            C::Text,
            K::Value,
            ("Text ersetzen", "Replace text"),
            (
                "Ersetzt alle Vorkommen eines Stücks im Text.",
                "Replaces every occurrence of a part in the text.",
            ),
        )
        .i("text", ("Text", "Text"), T::Text, "")
        .i("find", ("Suchen", "Find"), T::Text, "")
        .i("with", ("Ersetzen durch", "Replace with"), T::Text, "")
        .o("res", ("Text", "Text"), T::Text)
        .bout("res", "${in:text}.replace(${in:find}, ${in:with})"),
    );
    v.push(
        b("text_case", C::Text, K::Value,
            ("Groß-/Kleinschreibung", "Upper/lower case"),
            ("Wandelt Text in GROSS- oder kleinbuchstaben um.", "Turns text into UPPER or lower case."))
            .i("text", ("Text", "Text"), T::Text, "")
            .f("mode", ("Art", "Mode"), F::Choice(CASES), "upper")
            .o("res", ("Text", "Text"), T::Text)
            .bout("res", "(\"${f:mode}\".equals(\"upper\") ? ${in:text}.toUpperCase() : ${in:text}.toLowerCase())"),
    );
    v.push(
        b("open_menu", C::Player, K::Action,
            ("Menü öffnen", "Open menu"),
            ("Öffnet ein Kisten-Menü mit Items, z. B. für Shops oder Teleporter. Mit „Menü bearbeiten“ legst du Reihen und Items fest – wie in einer echten Kiste. Klicks fängst du mit „Klick im Menü“ ab. Nur Paper.",
             "Opens a chest menu with items, e.g. for shops or teleporters. “Edit menu” lets you set rows and items – like in a real chest. Catch clicks with “Menu click”. Paper only."))
            .f("menu", ("Menü-Name", "Menu name"), F::Text, "shop")
            .f("items", ("Items", "Items"), F::Menu, "11\tminecraft:diamond\t&bDiamant\t\t1\t\t\n15\tminecraft:emerald\t&aSmaragd\t\t1\t\t")
            .i("player", p_player, T::Player, "")
            .h(("Wem das Menü geöffnet wird. Nicht verbunden = der Spieler aus dem Event.", "Who the menu is opened for. Not connected = the player from the event."))
            .i("title", ("Titel", "Title"), T::Text, "&8Shop")
            .h(("Überschrift oben im Menü (mit &-Farbcodes).", "Heading at the top of the menu (with & color codes)."))
            .i("rows", ("Reihen", "Rows"), T::Number, "3")
            .h(("Wie viele Reihen das Menü hat (1–6, je 9 Plätze).", "How many rows the menu has (1–6, 9 slots each)."))
            .paper("Util.openMenu(${in:player}, ${s:menu}, ${in:title}, ${in:rows}, ${s:items});"),
    );
    v.push(
        b("ev_menu_click", C::Events, K::Event,
            ("Klick im Menü", "Menu click"),
            ("Wird ausgelöst, wenn ein Spieler in einem Menü von „Menü öffnen“ klickt. Mit „Item-ID“ reagiert der Block nur auf das Item, dem du im Menü-Editor diese ID gegeben hast. Items bleiben im Menü, außer du erlaubst das Herausnehmen. Nur Paper.",
             "Fires when a player clicks in a menu from “Open menu”. With “Item ID” the block only reacts to the item you gave this ID in the menu editor. Items stay in the menu unless you allow taking them. Paper only."))
            .f("menu", ("Menü-Name", "Menu name"), F::MenuName, "shop")
            .f("click_id", ("Item-ID", "Item ID"), F::MenuItemId, "")
            .o("player", p_player, T::Player)
            .o("slot", ("Platz", "Slot"), T::Number)
            .h(("Nummer des angeklickten Platzes: 0 ist oben links, pro Reihe 9 Plätze.", "Number of the clicked slot: 0 is top left, 9 slots per row."))
            .o("item", ("Item-ID", "Item id"), T::Text)
            .h(("ID des angeklickten Items, z. B. minecraft:diamond (minecraft:air bei leerem Platz).", "Id of the clicked item, e.g. minecraft:diamond (minecraft:air for an empty slot)."))
            .o("model", ("Modell", "Model"), T::Text)
            .h(("Modell-ID des angeklickten Items – bei eigenen Items aus dem Resourcepack z. B. meinpack:zauberstab.", "Model id of the clicked item – for custom items from the resource pack e.g. mypack:wand."))
            .listener("org.bukkit.event.inventory.InventoryClickEvent", false)
            .paper("if (!(event.getInventory().getHolder() instanceof Util.MenuHolder __mh) || !__mh.id.equals(${s:menu})) return;\n    if (event.getClickedInventory() != event.getView().getTopInventory()) return;\n    if (!${s:click_id}.isEmpty() && !Util.menuId(event.getCurrentItem()).equals(${s:click_id})) return;")
            .pout("player", "(org.bukkit.entity.Player) event.getWhoClicked()")
            .pout("slot", "event.getSlot()")
            .pout("item", "Util.itemId(event.getCurrentItem())")
            .o("id", ("ID", "ID"), T::Text)
            .h(("Die ID des angeklickten Platzes aus dem Menü-Editor (leer, wenn er keine hat).", "The ID of the clicked slot from the menu editor (empty if it has none)."))
            .pout("model", "Util.modelOf(event.getCurrentItem())")
            .pout("id", "Util.menuId(event.getCurrentItem())"),
    );
    v.push(
        b("replace_menu", C::Player, K::Action,
            ("Menü ersetzen", "Replace menu"),
            ("Tauscht ein Menü aus – für einen Spieler oder für alle. „Ganzes Menü“ ersetzt alles, „Nur diese Plätze“ nur die Plätze, die du im Editor belegst (der Rest bleibt wie im Original). Wer danach das Menü öffnet, sieht die neue Version (bleibt nach Neustart erhalten); ist es gerade offen, wird es sofort ausgetauscht. „Menü zurücksetzen“ holt das Original zurück. Nur Paper.",
             "Swaps a menu – for one player or for everyone. “Whole menu” replaces everything, “Only these slots” only the slots you fill in the editor (the rest stays as in the original). Whoever opens the menu afterwards sees the new version (survives restarts); if it is open right now, it is swapped at once. “Reset menu” brings the original back. Paper only."))
            .f("scope", ("Für", "For"), F::Choice(MENU_SCOPES), "player")
            .f("mode", ("Ersetzen", "Replace"), F::Choice(MENU_MODES), "all")
            .f("menu", ("Menü-Name", "Menu name"), F::MenuName, "shop")
            .f("items", ("Items", "Items"), F::Menu, "13\tminecraft:emerald\t&aNeues Menü\t&7Nur für dich\t1\t\t\t")
            .i("player", p_player, T::Player, "")
            .h(("Für wen das Menü ausgetauscht wird. Nicht verbunden = der Spieler aus dem Event. Bei „Alle Spieler“ wird er nicht gebraucht.", "Whose menu is swapped. Not connected = the player from the event. Not needed with “All players”."))
            .i("title", ("Titel", "Title"), T::Text, "&8Shop")
            .h(("Überschrift des neuen Menüs (mit &-Farbcodes).", "Heading of the new menu (with & color codes)."))
            .i("rows", ("Reihen", "Rows"), T::Number, "3")
            .h(("Wie viele Reihen das neue Menü hat (1–6).", "How many rows the new menu has (1–6)."))
            .paper("Util.replaceMenu(${in:player}, \"${f:scope}\".equals(\"global\"), ${s:menu}, ${in:title}, ${in:rows}, ${s:items}, \"${f:mode}\".equals(\"slots\"));"),
    );
    v.push(
        b("reset_menu", C::Player, K::Action,
            ("Menü zurücksetzen", "Reset menu"),
            ("Macht „Menü ersetzen“ rückgängig: Der Spieler (oder alle) sieht wieder das ursprüngliche Menü aus „Menü öffnen“. Nur Paper.",
             "Undoes “Replace menu”: the player (or everyone) sees the original menu from “Open menu” again. Paper only."))
            .f("scope", ("Für", "For"), F::Choice(MENU_SCOPES), "player")
            .f("menu", ("Menü-Name", "Menu name"), F::MenuName, "shop")
            .i("player", p_player, T::Player, "")
            .paper("Util.resetMenu(${in:player}, \"${f:scope}\".equals(\"global\"), ${s:menu});"),
    );
    v.push(
        b(
            "close_menu",
            C::Player,
            K::Action,
            ("Menü schließen", "Close menu"),
            (
                "Schließt das offene Menü / Inventar des Spielers.",
                "Closes the player's open menu / inventory.",
            ),
        )
        .i("player", p_player, T::Player, "")
        .paper("${in:player}.closeInventory();")
        .fabric("${in:player}.closeContainer();"),
    );


    // -------------------------------------------------------------- Locations
    v.push(
        b("player_loc", C::Location, K::Value,
            ("Position holen", "Get location"),
            ("Wo der Spieler gerade steht: als Position und einzeln als X, Y, Z und Welt. Die Position kannst du speichern, vergleichen oder zum Teleportieren benutzen.",
             "Where the player is right now: as a location and separately as X, Y, Z and world. You can store the location, compare it or teleport to it."))
            .i("player", p_player, T::Player, "")
            .o("loc", ("Position", "Location"), T::Location)
            .h(("Die komplette Position (Welt + Koordinaten). Verbinde sie z. B. mit „Position speichern“ oder „Teleportieren“.", "The whole location (world + coordinates). Connect it e.g. to “Save location” or “Teleport”."))
            .o("x", ("X", "X"), T::Number)
            .o("y", ("Y (Höhe)", "Y (height)"), T::Number)
            .o("z", ("Z", "Z"), T::Number)
            .o("world", ("Welt", "World"), T::Text)
            .pout("loc", "${in:player}.getLocation()")
            .pout("x", "${in:player}.getLocation().getX()")
            .pout("y", "${in:player}.getLocation().getY()")
            .pout("z", "${in:player}.getLocation().getZ()")
            .pout("world", "${in:player}.getWorld().getName()")
            .fout("loc", "Loc.of(${in:player})")
            .fout("x", "${in:player}.getX()")
            .fout("y", "${in:player}.getY()")
            .fout("z", "${in:player}.getZ()")
            .fout("world", "Util.worldName(${in:player}.level())"),
    );
    v.push(
        b("loc_save", C::Location, K::Action,
            ("Position speichern", "Save location"),
            ("Speichert eine Position dauerhaft unter einem Namen, z. B. „home“ – pro Spieler oder für alle. Bleibt nach einem Neustart erhalten.",
             "Stores a location permanently under a name, e.g. “home” – per player or for everyone. Survives restarts."))
            .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
            .i("player", p_player, T::Player, "")
            .i("key", ("Name", "Name"), T::Text, "home")
            .h(("Unter diesem Namen wird die Position abgelegt. Derselbe Name bei „Position laden“ holt sie zurück.", "The location is stored under this name. The same name in “Load location” brings it back."))
            .i("loc", ("Position", "Location"), T::Location, "")
            .h(("Die Position, die gespeichert wird. Nicht verbunden = die Position des Spielers aus dem Event.", "The location to store. Not connected = the location of the player from the event."))
            .both("Util.saveLoc(${storekey}, ${in:loc});"),
    );
    v.push(
        b("loc_load", C::Location, K::Flow,
            ("Position laden", "Load location"),
            ("Holt eine gespeicherte Position zurück. Gibt es sie, geht es bei „Gefunden“ weiter, sonst bei „Nicht gefunden“ (z. B. für eine Fehlermeldung).",
             "Brings back a stored location. If it exists the flow continues at “Found”, otherwise at “Not found” (e.g. for an error message)."))
            .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
            .i("player", p_player, T::Player, "")
            .i("key", ("Name", "Name"), T::Text, "home")
            .o("found", ("Gefunden", "Found"), T::Exec)
            .h(("Läuft, wenn unter dem Namen eine Position gespeichert ist.", "Runs if a location is stored under the name."))
            .o("missing", ("Nicht gefunden", "Not found"), T::Exec)
            .h(("Läuft, wenn noch nichts gespeichert wurde.", "Runs if nothing has been stored yet."))
            .o("loc", ("Position", "Location"), T::Location)
            .h(("Die geladene Position – nur im Zweig „Gefunden“ benutzbar.", "The loaded location – only usable in the “Found” branch."))
            .paper("final org.bukkit.Location ${out:loc} = Util.loadLoc(${storekey});\nif (${out:loc} != null) {\n${exec:found}\n} else {\n${exec:missing}\n}")
            .fabric("final Loc ${out:loc} = Util.loadLoc(${storekey});\nif (${out:loc} != null) {\n${exec:found}\n} else {\n${exec:missing}\n}"),
    );
    v.push(
        b("loc_exists", C::Location, K::Value,
            ("Position gespeichert?", "Location stored?"),
            ("Ja, wenn unter dem Namen eine Position gespeichert ist.", "Yes if a location is stored under the name."))
            .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
            .i("player", p_player, T::Player, "")
            .i("key", ("Name", "Name"), T::Text, "home")
            .o("res", ("Ergebnis", "Result"), T::Bool)
            .bout("res", "(Util.loadLoc(${storekey}) != null)"),
    );
    v.push(
        b("loc_delete", C::Location, K::Action,
            ("Gespeicherte Position löschen", "Delete stored location"),
            ("Entfernt eine gespeicherte Position wieder.", "Removes a stored location again."))
            .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
            .i("player", p_player, T::Player, "")
            .i("key", ("Name", "Name"), T::Text, "home")
            .both("Storage.removeText(\"loc.\" + ${storekey});"),
    );
    v.push(
        b("loc_list", C::Location, K::Value,
            ("Gespeicherte Positionen", "Stored locations"),
            ("Die Namen aller gespeicherten Positionen eines Spielers (oder aller globalen), z. B. „home, mine, farm“ – praktisch für /homes.",
             "The names of all stored locations of a player (or all global ones), e.g. “home, mine, farm” – handy for /homes."))
            .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
            .i("player", p_player, T::Player, "")
            .o("names", ("Namen", "Names"), T::Text)
            .h(("Alle Namen, mit Komma getrennt (leer, wenn keine).", "All names, separated by commas (empty if none)."))
            .o("count", ("Anzahl", "Count"), T::Number)
            .bout("names", "Storage.names(\"loc.\" + (\"${f:scope}\".equals(\"global\") ? \"global\" : Util.uuid(${in:player})) + \".\")")
            .bout("count", "Storage.count(\"loc.\" + (\"${f:scope}\".equals(\"global\") ? \"global\" : Util.uuid(${in:player})) + \".\")"),
    );
    v.push(
        b("look_loc", C::Location, K::Value,
            ("Position im Blick", "Location in sight"),
            ("Der Block, auf den der Spieler schaut. Ist nichts in Reichweite, kommt die Position des Spielers heraus.",
             "The block the player looks at. If nothing is in range you get the player's own location."))
            .i("player", p_player, T::Player, "")
            .i("distance", ("Reichweite", "Range"), T::Number, "50")
            .h(("Wie viele Blöcke weit gesucht wird (höchstens 120).", "How many blocks far to look (120 at most)."))
            .o("loc", ("Position", "Location"), T::Location)
            .bout("loc", "Util.lookLoc(${in:player}, ${in:distance})"),
    );
    v.push(
        b("loc_block", C::Location, K::Value,
            ("Blockmitte", "Block center"),
            ("Rundet eine Position auf die Mitte ihres Blocks – praktisch, damit Spieler nach dem Teleport nicht in einer Kante stehen.",
             "Rounds a location to the middle of its block – handy so players don't end up on an edge after teleporting."))
            .i("loc", ("Position", "Location"), T::Location, "")
            .o("res", ("Position", "Location"), T::Location)
            .bout("res", "Util.blockLoc(${in:loc})"),
    );
    v.push(
        b("loc_in_area", C::Location, K::Value,
            ("In Bereich?", "In area?"),
            ("Ja, wenn die Position im Quader zwischen den beiden Ecken liegt (z. B. Spawn-Schutz oder Arena).",
             "Yes if the location is inside the box between the two corners (e.g. spawn protection or an arena)."))
            .i("loc", ("Position", "Location"), T::Location, "")
            .h(("Die Position, die geprüft wird. Nicht verbunden = die Position des Spielers aus dem Event.", "The location to check. Not connected = the location of the player from the event."))
            .i("a", ("Ecke 1", "Corner 1"), T::Location, "")
            .i("b", ("Ecke 2", "Corner 2"), T::Location, "")
            .o("res", ("Ergebnis", "Result"), T::Bool)
            .bout("res", "Util.inArea(${in:loc}, ${in:a}, ${in:b})"),
    );
    v.push(
        b("world_spawn", C::Location, K::Value,
            ("Welt-Spawn", "World spawn"),
            ("Der Spawnpunkt einer Welt. Nur Paper.", "The spawn point of a world. Paper only."))
            .i("world", ("Welt", "World"), T::Text, "world")
            .o("loc", ("Position", "Location"), T::Location)
            .pout("loc", "Util.spawnLoc(${in:world})"),
    );
    v.push(
        b("respawn_loc", C::Location, K::Value,
            ("Bett / Respawn-Punkt", "Bed / respawn point"),
            ("Wo der Spieler nach dem Tod wieder erscheint (Bett oder Anker), sonst der Welt-Spawn. Nur Paper.",
             "Where the player reappears after dying (bed or anchor), otherwise the world spawn. Paper only."))
            .i("player", p_player, T::Player, "")
            .o("loc", ("Position", "Location"), T::Location)
            .pout("loc", "Util.respawnLoc(${in:player})"),
    );
    v.push(
        b("loc_surface", C::Location, K::Value,
            ("Oberfläche", "Surface"),
            ("Die erste freie Stelle über dem höchsten Block an dieser Position – gut für sichere Zufalls-Teleports. Nur Paper.",
             "The first free spot above the highest block at this location – good for safe random teleports. Paper only."))
            .i("loc", ("Position", "Location"), T::Location, "")
            .o("res", ("Position", "Location"), T::Location)
            .pout("res", "Util.surface(${in:loc})"),
    );

    // ---------------------------------------------------------------- Recipes
    v.push(
        b("recipe", C::Recipes, K::Value,
            ("Rezept", "Recipe"),
            ("Ein eigenes Rezept für Werkbank, Ofen, Räucherofen, Schmelzofen, Lagerfeuer oder Steinsäge. Mit „Rezept bearbeiten“ legst du Zutaten und Ergebnis fest – auch eigene Items aus deinem Resourcepack. Braucht keinen Ablauf: das Rezept gibt es, sobald der Server läuft.",
             "An own recipe for crafting table, furnace, smoker, blast furnace, campfire or stonecutter. “Edit recipe” sets ingredients and result – custom items from your resource pack too. Needs no flow: the recipe exists as soon as the server runs."))
            .special(Special::Recipe)
            .f("rtype", ("Art", "Kind"), F::Choice(crate::recipe::TYPES), "shaped")
            .f("rid", ("Rezept-Name", "Recipe name"), F::Text, "mein_rezept")
            .f("recipe", ("Rezept", "Recipe"), F::Recipe, crate::recipe::DEFAULT)
            .fa("xp", ("Erfahrung", "Experience"), F::Number, "0.1")
            .fa("seconds", ("Brennzeit (Sek.)", "Cooking time (s)"), F::Number, "10")
            .fa("discover", ("Im Rezeptbuch freischalten (Paper)", "Unlock in recipe book (Paper)"), F::Bool, "true")
            .paper_empty()
            .fabric_empty(),
    );


    // ------------------------------------------------------------- Moderation
    v.push(
        b("reason", C::Moderation, K::Value,
            ("Grund (Bildschirm)", "Reason (screen)"),
            ("Der Text, den ein Spieler beim Bann oder Kick auf dem „Verbindung getrennt“-Bildschirm sieht. Mit „Bildschirm bearbeiten“ gestaltest du ihn mit Farben und Zeilen; {time} wird zur Restzeit, {player} zum Namen, {category} zur Kategorie. Verbinde „Grund“ mit „Bannen“ oder „Kicken“.",
             "The text a player sees on the “Disconnected” screen when banned or kicked. “Edit screen” lets you design it with colors and lines; {time} becomes the time left, {player} the name, {category} the category. Connect “Reason” to “Ban” or “Kick”."))
            .f("category", ("Kategorie", "Category"), F::Choice(BAN_CATEGORIES), "Hacking")
            .f("screen", ("Bildschirm", "Screen"), F::Screen, "&cDu bist für &f{time} &cvon diesem Server gebannt!\n\n&7Grund: &f{category}\n&7Einspruch: &bdiscord.gg/deinserver")
            .o("reason", ("Grund", "Reason"), T::Text)
            .h(("Der fertige Bildschirm-Text. Verbinde ihn mit dem Eingang „Grund“ von „Bannen“ oder „Kicken“.", "The finished screen text. Connect it to the “Reason” input of “Ban” or “Kick”."))
            .pout("reason", "${s:screen}.replace(\"{category}\", \"${f:category}\")")
            .fout("reason", "${s:screen}.replace(\"{category}\", \"${f:category}\")"),
    );
    v.push(
        b("ban", C::Moderation, K::Action,
            ("Bannen", "Ban"),
            ("Sperrt einen Spieler für eine bestimmte Zeit (oder für immer) aus. Er sieht sofort und bei jedem Verbindungsversuch den Grund-Bildschirm mit der Restzeit. Bleibt nach Neustart erhalten. Nur Paper.",
             "Locks a player out for a certain time (or forever). They see the reason screen with the time left right away and on every attempt to join. Survives restarts. Paper only."))
            .f("unit", ("Einheit", "Unit"), F::Choice(BAN_UNITS), "86400")
            .i("player", p_player, T::Player, "")
            .i("duration", ("Dauer", "Duration"), T::Number, "30")
            .h(("Wie lange, in der gewählten Einheit. Bei „Für immer“ egal.", "How long, in the chosen unit. Ignored with “Forever”."))
            .i("reason", ("Grund", "Reason"), T::Text, "&cDu bist für &f{time} &cgebannt.")
            .h(("Text auf dem Bann-Bildschirm. Tippe ihn direkt ein oder verbinde einen „Grund (Bildschirm)“-Block. {time} = Restzeit, {player} = Name.", "Text on the ban screen. Type it in or connect a “Reason (screen)” block. {time} = time left, {player} = name."))
            .paper("Util.ban(${in:player}, \"${f:unit}\".equals(\"-1\") ? -1 : Math.abs(${in:duration}) * ${f:unit}, ${in:reason});"),
    );
    v.push(
        b("unban", C::Moderation, K::Action,
            ("Entbannen", "Unban"),
            ("Hebt den Bann eines Spielers auf (nach Name). „Geklappt“ sagt, ob er gebannt war. Nur Paper.",
             "Lifts a player's ban (by name). “Worked” tells whether they were banned. Paper only."))
            .i("name", ("Spielername", "Player name"), T::Text, "")
            .o("ok", ("Geklappt", "Worked"), T::Bool)
            .paper("final boolean ${out:ok} = Util.unban(${in:name});"),
    );
    v.push(
        b("is_banned", C::Moderation, K::Value,
            ("Ist gebannt?", "Is banned?"),
            ("Ja, wenn der Spieler mit diesem Namen gerade gebannt ist. Nur Paper.", "Yes if the player with this name is banned right now. Paper only."))
            .i("name", ("Spielername", "Player name"), T::Text, "")
            .o("res", ("Ergebnis", "Result"), T::Bool)
            .pout("res", "Util.isBanned(${in:name})"),
    );

    // ------------------------------------------------------ Variables & lists
    v.push(
        b("var_set", C::Vars, K::Action,
            ("Variable setzen", "Set variable"),
            ("Merkt sich einen Wert, bis der Server stoppt – z. B. „im Event“ oder „Runde läuft“. Für Dauerhaftes nimm „Zahl/Text speichern“.",
             "Remembers a value until the server stops – e.g. “in event” or “round running”. For permanent values use “Store number/text”."))
            .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
            .i("player", p_player, T::Player, "")
            .i("key", ("Name", "Name"), T::Text, "im_event")
            .i("value", ("Wert", "Value"), T::Text, "ja")
            .both("Util.varSet(${storekey}, ${in:value});"),
    );
    v.push(
        b("var_get", C::Vars, K::Value,
            ("Variable", "Variable"),
            ("Liest eine Variable (leer bzw. 0, wenn sie nicht gesetzt ist).", "Reads a variable (empty or 0 if it is not set)."))
            .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
            .i("player", p_player, T::Player, "")
            .i("key", ("Name", "Name"), T::Text, "im_event")
            .o("value", ("Wert", "Value"), T::Text)
            .o("set", ("Gesetzt?", "Set?"), T::Bool)
            .bout("value", "Util.varGet(${storekey})")
            .bout("set", "(!Util.varGet(${storekey}).isEmpty())"),
    );
    v.push(
        b("var_add", C::Vars, K::Action,
            ("Variable erhöhen", "Increase variable"),
            ("Zählt eine Zahl-Variable hoch (oder mit Minus herunter) und gibt den neuen Wert aus.", "Counts a number variable up (or down with minus) and outputs the new value."))
            .f("scope", ("Bereich", "Scope"), F::Choice(SCOPES), "player")
            .i("player", p_player, T::Player, "")
            .i("key", ("Name", "Name"), T::Text, "kills")
            .i("by", ("Um", "By"), T::Number, "1")
            .o("value", ("Neuer Wert", "New value"), T::Number)
            .both("final double ${out:value} = Util.varAdd(${storekey}, ${in:by});"),
    );
    v.push(
        b("list_add", C::Vars, K::Action,
            ("Zur Liste hinzufügen", "Add to list"),
            ("Hängt einen Eintrag an eine Liste an, z. B. einen Spielernamen an „arena“. Listen gelten, bis der Server stoppt.",
             "Appends an entry to a list, e.g. a player name to “arena”. Lists last until the server stops."))
            .f("unique", ("Nur einmal", "Only once"), F::Bool, "true")
            .i("key", ("Liste", "List"), T::Text, "arena")
            .i("value", ("Eintrag", "Entry"), T::Text, "")
            .h(("Was hinzugefügt wird. Ein verbundener Spieler wird zu seinem Namen.", "What is added. A connected player turns into their name."))
            .both("Util.listAdd(${in:key}, ${in:value}, ${f:unique});"),
    );
    v.push(
        b("list_remove", C::Vars, K::Action,
            ("Aus Liste entfernen", "Remove from list"),
            ("Entfernt einen Eintrag aus einer Liste.", "Removes an entry from a list."))
            .i("key", ("Liste", "List"), T::Text, "arena")
            .i("value", ("Eintrag", "Entry"), T::Text, "")
            .both("Util.listRemove(${in:key}, ${in:value});"),
    );
    v.push(
        b("list_clear", C::Vars, K::Action,
            ("Liste leeren", "Clear list"),
            ("Entfernt alle Einträge einer Liste.", "Removes all entries of a list."))
            .i("key", ("Liste", "List"), T::Text, "arena")
            .both("Util.listClear(${in:key});"),
    );
    v.push(
        b("list_info", C::Vars, K::Value,
            ("Liste", "List"),
            ("Infos zu einer Liste: ob ein Eintrag drin ist, wie viele es sind, ein bestimmter Eintrag und alle als Text.",
             "Info about a list: whether an entry is in it, how many there are, a certain entry and all of them as text."))
            .i("key", ("Liste", "List"), T::Text, "arena")
            .ia("value", ("Eintrag suchen", "Entry to find"), T::Text, "")
            .ia("index", ("Nummer", "Number"), T::Number, "1")
            .o("has", ("Enthält?", "Contains?"), T::Bool)
            .h(("Ja, wenn „Eintrag suchen“ in der Liste steht.", "Yes if “Entry to find” is in the list."))
            .o("size", ("Anzahl", "Count"), T::Number)
            .o("entry", ("Eintrag Nr.", "Entry no."), T::Text)
            .h(("Der Eintrag an der Stelle „Nummer“ (1 = erster).", "The entry at position “Number” (1 = first)."))
            .o("all", ("Alle", "All"), T::Text)
            .h(("Alle Einträge, mit Komma getrennt.", "All entries, separated by commas."))
            .bout("has", "Util.listHas(${in:key}, ${in:value})")
            .bout("size", "Util.listSize(${in:key})")
            .bout("entry", "Util.listGet(${in:key}, ${in:index})")
            .bout("all", "Util.listJoin(${in:key}, \", \")"),
    );
    v.push(
        b("for_list", C::Vars, K::Flow,
            ("Für jeden Eintrag", "For each entry"),
            ("Führt „Schleife“ für jeden Eintrag einer Liste aus. Für Spielernamen passt „Spieler nach Name“ dahinter.",
             "Runs “Loop” for every entry of a list. For player names, “Player by name” fits behind it."))
            .i("key", ("Liste", "List"), T::Text, "arena")
            .o("loop", ("Schleife", "Loop"), T::Exec)
            .o("item", ("Eintrag", "Entry"), T::Text)
            .o("next", ("Fertig", "Done"), T::Exec)
            .both("for (final String ${out:item} : Util.listCopy(${in:key})) {\n${exec:loop}\n}"),
    );
    v.push(
        b("timer_start", C::Flow, K::Flow,
            ("Countdown starten", "Start countdown"),
            ("Führt „Abgelaufen“ nach X Sekunden aus – pro Spieler und Name. Ein neuer Countdown mit demselben Namen ersetzt den alten; „Countdown abbrechen“ stoppt ihn (z. B. wenn sich der Spieler bewegt).",
             "Runs “Elapsed” after X seconds – per player and name. A new countdown with the same name replaces the old one; “Cancel countdown” stops it (e.g. when the player moves)."))
            .i("player", p_player, T::Player, "")
            .i("key", ("Name", "Name"), T::Text, "teleport")
            .i("seconds", ("Sekunden", "Seconds"), T::Number, "5")
            .o("done", ("Abgelaufen", "Elapsed"), T::Exec)
            .both("Util.timer(Util.uuid(${in:player}) + \":\" + ${in:key}, ${in:seconds}, () -> {\n${exec:done}\n});"),
    );
    v.push(
        b("timer_cancel", C::Flow, K::Action,
            ("Countdown abbrechen", "Cancel countdown"),
            ("Stoppt einen laufenden Countdown. „Lief noch“ sagt, ob es einen gab.", "Stops a running countdown. “Was running” tells whether there was one."))
            .i("player", p_player, T::Player, "")
            .i("key", ("Name", "Name"), T::Text, "teleport")
            .o("ok", ("Lief noch", "Was running"), T::Bool)
            .both("final boolean ${out:ok} = Util.cancelTimer(Util.uuid(${in:player}) + \":\" + ${in:key});"),
    );
    v.push(
        b("ev_function", C::Flow, K::Event,
            ("Eigener Block", "Own block"),
            ("Fasst einen Ablauf unter einem Namen zusammen. Mit „Eigenen Block ausführen“ startest du ihn von überall – so musst du nichts doppelt bauen.",
             "Bundles a flow under a name. “Run own block” starts it from anywhere – so you never build things twice."))
            .special(Special::Function)
            .f("name", ("Name", "Name"), F::Text, "belohnung")
            .o("player", p_player, T::Player)
            .paper_empty()
            .fabric_empty(),
    );
    v.push(
        b("call_function", C::Flow, K::Action,
            ("Eigenen Block ausführen", "Run own block"),
            ("Startet den Ablauf eines „Eigener Block“ mit diesem Namen und macht danach hier weiter.", "Starts the flow of an “Own block” with this name and continues here afterwards."))
            .f("name", ("Name", "Name"), F::Text, "belohnung")
            .i("player", p_player, T::Player, "")
            .both("Util.call(${s:name}, ${in:player});"),
    );
    v.push(
        b("msg", C::Files, K::Value,
            ("Nachricht (messages.yml)", "Message (messages.yml)"),
            ("Ein Text aus messages.yml im Datenordner. Fehlt er dort, wird er mit dem Standard angelegt – so können Server-Besitzer alle Texte ändern oder übersetzen.",
             "A text from messages.yml in the data folder. If it is missing there, it is added with the default – so server owners can change or translate all texts."))
            .i("key", ("Name", "Name"), T::Text, "willkommen")
            .i("default", ("Standard", "Default"), T::Text, "&aWillkommen!")
            .o("text", ("Text", "Text"), T::Text)
            .bout("text", "DataFiles.msg(${in:key}, ${in:default})"),
    );

    // ------------------------------------------------------------ More events
    v.push(
        b("ev_click_block", C::Events, K::Event,
            ("Block angeklickt", "Block clicked"),
            ("Wird ausgelöst, wenn ein Spieler einen Block anklickt (z. B. ein Schild oder einen Knopf). Kann abgebrochen werden. Fabric: nur Rechtsklick.",
             "Fires when a player clicks a block (e.g. a sign or a button). Can be cancelled. Fabric: right click only."))
            .o("player", p_player, T::Player)
            .o("block", ("Block-ID", "Block id"), T::Text)
            .o("loc", ("Position", "Location"), T::Location)
            .o("right", ("Rechtsklick?", "Right click?"), T::Bool)
            .listener("org.bukkit.event.player.PlayerInteractEvent", true)
            .paper("if (event.getHand() != org.bukkit.inventory.EquipmentSlot.HAND || event.getClickedBlock() == null) return;")
            .pout("player", "event.getPlayer()")
            .pout("block", "event.getClickedBlock().getType().getKey().toString()")
            .pout("loc", "event.getClickedBlock().getLocation()")
            .pout("right", "event.getAction().isRightClick()")
            .fabric("UseBlockCallback.EVENT.register((__p, level, hand, __hit) -> {\n    if (level.isClientSide() || hand != InteractionHand.MAIN_HAND || !(__p instanceof ServerPlayer)) return InteractionResult.PASS;\n${body}\n    return __cancel[0] ? InteractionResult.FAIL : InteractionResult.PASS;\n});")
            .fcancel("return __cancel[0] ? InteractionResult.FAIL : InteractionResult.PASS;")
            .fout("player", "(ServerPlayer) __p")
            .fout("block", "BuiltInRegistries.BLOCK.getKey(level.getBlockState(__hit.getBlockPos()).getBlock()).toString()")
            .fout("loc", "new Loc((ServerLevel) level, __hit.getBlockPos().getX(), __hit.getBlockPos().getY(), __hit.getBlockPos().getZ())")
            .fout("right", "true")
            .fimp("net.fabricmc.fabric.api.event.player.UseBlockCallback")
            .fimp("net.minecraft.world.InteractionHand")
            .fimp("net.minecraft.world.InteractionResult"),
    );
    v.push(
        b("ev_move", C::Events, K::Event,
            ("Spieler bewegt sich", "Player moves"),
            ("Wird ausgelöst, sobald ein Spieler auf einen anderen Block wechselt (nicht beim bloßen Umschauen). Kann abgebrochen werden. Nur Paper.",
             "Fires as soon as a player steps onto another block (not when just looking around). Can be cancelled. Paper only."))
            .o("player", p_player, T::Player)
            .o("loc", ("Neue Position", "New location"), T::Location)
            .listener("org.bukkit.event.player.PlayerMoveEvent", true)
            .paper("if (!event.hasChangedBlock()) return;")
            .pout("player", "event.getPlayer()")
            .pout("loc", "event.getTo()"),
    );
    v.push(
        b("ev_enter_area", C::Events, K::Event,
            ("Bereich betreten", "Area entered"),
            ("Wird ausgelöst, wenn ein Spieler den Quader zwischen zwei Ecken betritt (oder verlässt) – z. B. Spawn, Arena oder Shop. Nur Paper.",
             "Fires when a player enters (or leaves) the box between two corners – e.g. spawn, arena or shop. Paper only."))
            .f("mode", ("Wann", "When"), F::Choice(AREA_MODES), "enter")
            .f("world", ("Welt", "World"), F::Text, "world")
            .f("x1", ("Ecke 1 X", "Corner 1 X"), F::Number, "-10")
            .f("y1", ("Ecke 1 Y", "Corner 1 Y"), F::Number, "-64")
            .f("z1", ("Ecke 1 Z", "Corner 1 Z"), F::Number, "-10")
            .f("x2", ("Ecke 2 X", "Corner 2 X"), F::Number, "10")
            .f("y2", ("Ecke 2 Y", "Corner 2 Y"), F::Number, "320")
            .f("z2", ("Ecke 2 Z", "Corner 2 Z"), F::Number, "10")
            .o("player", p_player, T::Player)
            .listener("org.bukkit.event.player.PlayerMoveEvent", false)
            .paper("if (!event.hasChangedBlock()) return;\n    final boolean __in = Util.inBox(event.getTo(), ${s:world}, ${f:x1}, ${f:y1}, ${f:z1}, ${f:x2}, ${f:y2}, ${f:z2});\n    final boolean __was = Util.inBox(event.getFrom(), ${s:world}, ${f:x1}, ${f:y1}, ${f:z1}, ${f:x2}, ${f:y2}, ${f:z2});\n    if (__in == __was || __in != \"${f:mode}\".equals(\"enter\")) return;")
            .pout("player", "event.getPlayer()"),
    );
    v.push(
        b("ev_sneak", C::Events, K::Event,
            ("Spieler schleicht", "Player sneaks"),
            ("Wird ausgelöst, wenn ein Spieler anfängt zu schleichen (Shift). Nur Paper.", "Fires when a player starts sneaking (shift). Paper only."))
            .o("player", p_player, T::Player)
            .listener("org.bukkit.event.player.PlayerToggleSneakEvent", true)
            .paper("if (!event.isSneaking()) return;")
            .pout("player", "event.getPlayer()"),
    );
    v.push(
        b("ev_jump", C::Events, K::Event,
            ("Spieler springt", "Player jumps"),
            ("Wird ausgelöst, wenn ein Spieler springt. Nur Paper.", "Fires when a player jumps. Paper only."))
            .o("player", p_player, T::Player)
            .listener("com.destroystokyo.paper.event.player.PlayerJumpEvent", true)
            .pout("player", "event.getPlayer()"),
    );
    v.push(
        b("ev_world_change", C::Events, K::Event,
            ("Welt gewechselt", "World changed"),
            ("Wird ausgelöst, nachdem ein Spieler in eine andere Welt gewechselt ist (z. B. Nether). Nur Paper.", "Fires after a player changed to another world (e.g. the Nether). Paper only."))
            .o("player", p_player, T::Player)
            .o("world", ("Neue Welt", "New world"), T::Text)
            .listener("org.bukkit.event.player.PlayerChangedWorldEvent", false)
            .pout("player", "event.getPlayer()")
            .pout("world", "event.getPlayer().getWorld().getName()"),
    );
    v.push(
        b("ev_respawn", C::Events, K::Event,
            ("Spieler respawnt", "Player respawns"),
            ("Wird ausgelöst, wenn ein Spieler nach dem Tod wieder erscheint.", "Fires when a player reappears after dying."))
            .o("player", p_player, T::Player)
            .listener("org.bukkit.event.player.PlayerRespawnEvent", false)
            .pout("player", "event.getPlayer()")
            .fabric("ServerPlayerEvents.AFTER_RESPAWN.register((__old, __p, __alive) -> {\n${body}\n});")
            .fout("player", "__p")
            .fimp("net.fabricmc.fabric.api.entity.event.v1.ServerPlayerEvents"),
    );
    v.push(
        b("ev_pickup", C::Events, K::Event,
            ("Item aufgehoben", "Item picked up"),
            ("Wird ausgelöst, wenn ein Spieler ein Item vom Boden aufhebt. Kann abgebrochen werden. Nur Paper.", "Fires when a player picks an item up from the ground. Can be cancelled. Paper only."))
            .o("player", p_player, T::Player)
            .o("item", ("Item-ID", "Item id"), T::Text)
            .o("model", ("Modell", "Model"), T::Text)
            .listener("org.bukkit.event.entity.EntityPickupItemEvent", true)
            .paper("if (!(event.getEntity() instanceof org.bukkit.entity.Player)) return;")
            .pout("player", "(org.bukkit.entity.Player) event.getEntity()")
            .pout("item", "Util.itemId(event.getItem().getItemStack())")
            .pout("model", "Util.modelOf(event.getItem().getItemStack())"),
    );
    v.push(
        b("ev_drop", C::Events, K::Event,
            ("Item weggeworfen", "Item dropped"),
            ("Wird ausgelöst, wenn ein Spieler ein Item wegwirft. Kann abgebrochen werden. Nur Paper.", "Fires when a player drops an item. Can be cancelled. Paper only."))
            .o("player", p_player, T::Player)
            .o("item", ("Item-ID", "Item id"), T::Text)
            .o("model", ("Modell", "Model"), T::Text)
            .listener("org.bukkit.event.player.PlayerDropItemEvent", true)
            .pout("player", "event.getPlayer()")
            .pout("item", "Util.itemId(event.getItemDrop().getItemStack())")
            .pout("model", "Util.modelOf(event.getItemDrop().getItemStack())"),
    );

    // ---------------------------------------------- Holograms, NPCs, money, ranks
    v.push(
        b("hologram", C::World, K::Action,
            ("Hologramm", "Hologram"),
            ("Setzt schwebenden Text an eine Position. Mehrere Zeilen mit | trennen. Derselbe Name ersetzt das alte Hologramm. Nur Paper.",
             "Puts floating text at a location. Separate several lines with |. The same name replaces the old hologram. Paper only."))
            .i("loc", ("Position", "Location"), T::Location, "")
            .i("key", ("Name", "Name"), T::Text, "willkommen")
            .i("text", ("Text", "Text"), T::Text, "&6&lWillkommen!|&7Viel Spaß")
            .paper("Util.hologram(${in:loc}, ${in:key}, ${in:text});"),
    );
    v.push(
        b("npc_spawn", C::World, K::Action,
            ("NPC hinstellen", "Place NPC"),
            ("Stellt eine Figur hin, die sich nicht bewegt und keinen Schaden nimmt – z. B. einen Händler. Klicks fängst du mit „NPC angeklickt“ ab. Nur Paper.",
             "Places a figure that does not move and takes no damage – e.g. a trader. Catch clicks with “NPC clicked”. Paper only."))
            .i("loc", ("Position", "Location"), T::Location, "")
            .i("key", ("Name (intern)", "Name (internal)"), T::Text, "haendler")
            .i("name", ("Anzeigename", "Display name"), T::Text, "&6Händler")
            .ia("type", ("Art", "Type"), T::Text, "minecraft:villager")
            .paper("Util.npc(${in:loc}, ${in:key}, ${in:name}, ${in:type});"),
    );
    v.push(
        b("entity_remove", C::World, K::Action,
            ("Hologramm / NPC entfernen", "Remove hologram / NPC"),
            ("Entfernt ein Hologramm oder einen NPC mit diesem Namen. Nur Paper.", "Removes a hologram or NPC with this name. Paper only."))
            .i("key", ("Name", "Name"), T::Text, "haendler")
            .paper("Util.removeTagged(${in:key});"),
    );
    v.push(
        b("ev_npc_click", C::Events, K::Event,
            ("NPC angeklickt", "NPC clicked"),
            ("Wird ausgelöst, wenn ein Spieler einen NPC aus „NPC hinstellen“ rechtsklickt – z. B. um ein Menü zu öffnen. Nur Paper.",
             "Fires when a player right-clicks an NPC from “Place NPC” – e.g. to open a menu. Paper only."))
            .f("key", ("NPC-Name", "NPC name"), F::Text, "haendler")
            .o("player", p_player, T::Player)
            .listener("org.bukkit.event.player.PlayerInteractEntityEvent", false)
            .paper("if (event.getHand() != org.bukkit.inventory.EquipmentSlot.HAND || !Util.tagOf(event.getRightClicked()).equals(${s:key})) return;\n    event.setCancelled(true);")
            .pout("player", "event.getPlayer()"),
    );
    v.push(
        b("money_get", C::Data, K::Value,
            ("Kontostand", "Balance"),
            ("Wie viel Geld der Spieler hat.", "How much money the player has."))
            .i("player", p_player, T::Player, "")
            .o("amount", ("Geld", "Money"), T::Number)
            .bout("amount", "Util.money(${in:player})"),
    );
    v.push(
        b("money_add", C::Data, K::Action,
            ("Geld geben", "Give money"),
            ("Gibt dem Spieler Geld (bleibt nach Neustart erhalten).", "Gives the player money (survives restarts)."))
            .i("player", p_player, T::Player, "")
            .i("amount", ("Betrag", "Amount"), T::Number, "100")
            .both("Util.moneyAdd(${in:player}, ${in:amount});"),
    );
    v.push(
        b("money_take", C::Data, K::Action,
            ("Geld abziehen", "Take money"),
            ("Zieht Geld ab – nur wenn der Spieler genug hat. „Geklappt“ sagt, ob es funktioniert hat (gut für Shops).",
             "Takes money – only if the player has enough. “Worked” tells whether it worked (good for shops)."))
            .i("player", p_player, T::Player, "")
            .i("amount", ("Betrag", "Amount"), T::Number, "50")
            .o("ok", ("Geklappt", "Worked"), T::Bool)
            .both("final boolean ${out:ok} = Util.moneyTake(${in:player}, ${in:amount});"),
    );
    v.push(
        b("set_prefix", C::Player, K::Action,
            ("Rang-Präfix setzen", "Set rank prefix"),
            ("Setzt einen Rang vor den Namen in Chat und Tab-Liste, z. B. „&c[Admin]“. Leer entfernt ihn. Bleibt nach Neustart erhalten. Nur Paper.",
             "Puts a rank before the name in chat and tab list, e.g. “&c[Admin]”. Empty removes it. Survives restarts. Paper only."))
            .i("player", p_player, T::Player, "")
            .i("prefix", ("Präfix", "Prefix"), T::Text, "&c[Admin]")
            .paper("Util.setPrefix(${in:player}, ${in:prefix});"),
    );
    v.push(
        b("set_perm", C::Player, K::Action,
            ("Berechtigung geben", "Give permission"),
            ("Gibt oder nimmt einem Spieler eine Berechtigung, z. B. für einen deiner Befehle. Bleibt nach Neustart erhalten. Nur Paper.",
             "Gives or takes a permission from a player, e.g. for one of your commands. Survives restarts. Paper only."))
            .i("player", p_player, T::Player, "")
            .i("perm", ("Berechtigung", "Permission"), T::Text, "meinplugin.vip")
            .i("on", ("Geben", "Give"), T::Bool, "true")
            .paper("Util.setPerm(${in:player}, ${in:perm}, ${in:on});"),
    );

    // ------------------------------------------------------ Files & settings
    let p_file = ("Datei", "File");
    let h_file: L = (
        "Pfad im Ordner deines Plugins (plugins/<Name>/) bzw. Mods (config/<id>/). Unterordner mit / trennen.",
        "Path inside your plugin's folder (plugins/<Name>/) or mod's folder (config/<id>/). Separate subfolders with /.",
    );
    v.push(
        b("folder_create", C::Files, K::Action,
            ("Ordner erstellen", "Create folder"),
            ("Legt einen Ordner im Datenordner an – bei Plugins plugins/<Name>/…, bei Mods config/<id>/…. Der Hauptordner selbst entsteht automatisch. Am besten gleich „Beim Start“.",
             "Creates a folder in the data folder – plugins/<Name>/… for plugins, config/<id>/… for mods. The main folder itself is created automatically. Best done right “On start”."))
            .i("path", ("Ordner", "Folder"), T::Text, "daten")
            .h(("Name des Ordners, auch verschachtelt wie „daten/spieler“.", "Name of the folder, also nested like “data/players”."))
            .both("DataFiles.mkdir(${in:path});"),
    );
    v.push(
        b("cfg_default", C::Files, K::Action,
            ("Einstellung anlegen", "Add setting"),
            ("Schreibt eine Einstellung mit Standardwert in eine Einstellungsdatei, falls sie dort noch fehlt. So entsteht z. B. plugins/<Name>/settings.yml, die Server-Besitzer bearbeiten können.",
             "Writes a setting with a default value into a settings file if it is missing there. That creates e.g. plugins/<Name>/settings.yml for server owners to edit."))
            .i("file", p_file, T::Text, "settings.yml")
            .h(h_file)
            .i("key", ("Einstellung", "Setting"), T::Text, "willkommen")
            .h(("Name der Einstellung, z. B. „willkommen“ oder „shop.preis“.", "Name of the setting, e.g. “welcome” or “shop.price”."))
            .i("value", ("Standardwert", "Default value"), T::Text, "&aWillkommen!")
            .ia("comment", ("Kommentar", "Comment"), T::Text, "")
            .h(("Erklärung, die als #-Zeile über der Einstellung in der Datei steht.", "Explanation written as a # line above the setting in the file."))
            .both("DataFiles.cfgDefault(${in:file}, ${in:key}, ${in:value}, ${in:comment});"),
    );
    v.push(
        b("cfg_text", C::Files, K::Value,
            ("Einstellung (Text)", "Setting (text)"),
            ("Liest einen Text aus einer Einstellungsdatei.", "Reads a text from a settings file."))
            .i("file", p_file, T::Text, "settings.yml")
            .h(h_file)
            .i("key", ("Einstellung", "Setting"), T::Text, "willkommen")
            .ia("default", ("Sonst", "Otherwise"), T::Text, "")
            .h(("Dieser Wert kommt heraus, wenn die Einstellung fehlt.", "This value is used if the setting is missing."))
            .o("value", ("Text", "Text"), T::Text)
            .bout("value", "DataFiles.cfgText(${in:file}, ${in:key}, ${in:default})"),
    );
    v.push(
        b("cfg_num", C::Files, K::Value,
            ("Einstellung (Zahl)", "Setting (number)"),
            ("Liest eine Zahl aus einer Einstellungsdatei.", "Reads a number from a settings file."))
            .i("file", p_file, T::Text, "settings.yml")
            .h(h_file)
            .i("key", ("Einstellung", "Setting"), T::Text, "preis")
            .ia("default", ("Sonst", "Otherwise"), T::Number, "0")
            .o("value", ("Zahl", "Number"), T::Number)
            .bout("value", "DataFiles.cfgNum(${in:file}, ${in:key}, ${in:default})"),
    );
    v.push(
        b("cfg_bool", C::Files, K::Value,
            ("Einstellung (Ja/Nein)", "Setting (yes/no)"),
            ("Liest einen Schalter aus einer Einstellungsdatei (true/false, ja/nein, an/aus).", "Reads a switch from a settings file (true/false, yes/no, on/off)."))
            .i("file", p_file, T::Text, "settings.yml")
            .h(h_file)
            .i("key", ("Einstellung", "Setting"), T::Text, "aktiv")
            .ia("default", ("Sonst", "Otherwise"), T::Bool, "true")
            .o("value", ("Ja/Nein", "Yes/No"), T::Bool)
            .bout("value", "DataFiles.cfgBool(${in:file}, ${in:key}, ${in:default})"),
    );
    v.push(
        b("cfg_set", C::Files, K::Action,
            ("Einstellung ändern", "Change setting"),
            ("Schreibt einen neuen Wert in eine Einstellungsdatei. Kommentare in der Datei bleiben erhalten.", "Writes a new value into a settings file. Comments in the file are kept."))
            .i("file", p_file, T::Text, "settings.yml")
            .h(h_file)
            .i("key", ("Einstellung", "Setting"), T::Text, "willkommen")
            .i("value", ("Wert", "Value"), T::Text, "")
            .both("DataFiles.cfgSet(${in:file}, ${in:key}, ${in:value});"),
    );
    v.push(
        b("cfg_reload", C::Files, K::Action,
            ("Einstellungen neu laden", "Reload settings"),
            ("Liest eine Einstellungsdatei neu von der Festplatte – z. B. in einem /reload-Befehl, nachdem jemand die Datei bearbeitet hat. Leer = alle Dateien.",
             "Reads a settings file from disk again – e.g. in a /reload command after someone edited the file. Empty = all files."))
            .i("file", p_file, T::Text, "settings.yml")
            .both("DataFiles.cfgReload(${in:file});"),
    );
    v.push(
        b("file_write", C::Files, K::Action,
            ("Datei schreiben", "Write file"),
            ("Schreibt Text in eine Datei im Datenordner (ersetzt den alten Inhalt). Fehlende Ordner werden angelegt.", "Writes text into a file in the data folder (replaces the old content). Missing folders are created."))
            .i("file", p_file, T::Text, "notiz.txt")
            .h(h_file)
            .i("text", ("Inhalt", "Content"), T::Text, "")
            .both("DataFiles.write(${in:file}, ${in:text});"),
    );
    v.push(
        b("file_append", C::Files, K::Action,
            ("Zeile anhängen", "Append line"),
            ("Hängt eine Zeile ans Ende einer Datei an – gut für eigene Logs.", "Appends a line to the end of a file – good for your own logs."))
            .i("file", p_file, T::Text, "log.txt")
            .h(h_file)
            .i("text", ("Zeile", "Line"), T::Text, "")
            .both("DataFiles.append(${in:file}, ${in:text});"),
    );
    v.push(
        b("file_read", C::Files, K::Value,
            ("Datei lesen", "Read file"),
            ("Der ganze Inhalt einer Datei als Text (leer, wenn es sie nicht gibt).", "The whole content of a file as text (empty if it does not exist)."))
            .i("file", p_file, T::Text, "notiz.txt")
            .h(h_file)
            .o("text", ("Inhalt", "Content"), T::Text)
            .bout("text", "DataFiles.read(${in:file})"),
    );
    v.push(
        b("file_exists", C::Files, K::Value,
            ("Datei vorhanden?", "File exists?"),
            ("Ja, wenn es die Datei oder den Ordner gibt.", "Yes if the file or folder exists."))
            .i("file", p_file, T::Text, "settings.yml")
            .h(h_file)
            .o("res", ("Ergebnis", "Result"), T::Bool)
            .bout("res", "DataFiles.exists(${in:file})"),
    );
    v.push(
        b("file_delete", C::Files, K::Action,
            ("Datei löschen", "Delete file"),
            ("Löscht eine Datei oder einen leeren Ordner im Datenordner.", "Deletes a file or an empty folder in the data folder."))
            .i("file", p_file, T::Text, "notiz.txt")
            .h(h_file)
            .both("DataFiles.delete(${in:file});"),
    );
    v.push(
        b("file_list", C::Files, K::Value,
            ("Dateien im Ordner", "Files in folder"),
            ("Die Namen aller Dateien eines Ordners, mit Komma getrennt.", "The names of all files in a folder, separated by commas."))
            .i("path", ("Ordner", "Folder"), T::Text, "daten")
            .o("names", ("Namen", "Names"), T::Text)
            .bout("names", "DataFiles.list(${in:path})"),
    );

    // --------------------------------------------------- More resource pack
    let h_model: L = (
        "Ein Modell aus deinem Resourcepack (Seitenleiste „Texturen & Modelle“).",
        "A model from your resource pack (sidebar “Textures & models”).",
    );
    let _ = h_model;
    v.push(
        b("ev_custom_use", C::Resources, K::Event,
            ("Rechtsklick mit eigenem Item", "Right click with custom item"),
            ("Wird ausgelöst, wenn ein Spieler mit einem Item rechtsklickt, das dieses Modell hat – z. B. für Zauberstäbe. Kann abgebrochen werden.",
             "Fires when a player right-clicks with an item that has this model – e.g. for magic wands. Can be cancelled."))
            .f("model", ("Modell", "Model"), F::ItemModel, "")
            .o("player", p_player, T::Player)
            .listener("org.bukkit.event.player.PlayerInteractEvent", true)
            .paper("if (event.getHand() != org.bukkit.inventory.EquipmentSlot.HAND || !event.getAction().isRightClick() || !Util.heldModel(event.getPlayer()).equalsIgnoreCase(${s:model})) return;")
            .pout("player", "event.getPlayer()")
            .fabric("UseItemCallback.EVENT.register((__p, level, hand) -> {\n    if (level.isClientSide() || hand != InteractionHand.MAIN_HAND || !(__p instanceof ServerPlayer) || !Util.heldModel((ServerPlayer) __p).equalsIgnoreCase(${s:model})) return InteractionResult.PASS;\n${body}\n    return __cancel[0] ? InteractionResult.FAIL : InteractionResult.PASS;\n});")
            .fcancel("return __cancel[0] ? InteractionResult.FAIL : InteractionResult.PASS;")
            .fout("player", "(ServerPlayer) __p")
            .fimp("net.fabricmc.fabric.api.event.player.UseItemCallback")
            .fimp("net.minecraft.world.InteractionHand")
            .fimp("net.minecraft.world.InteractionResult"),
    );
    v.push(
        b("set_held_model", C::Resources, K::Action,
            ("Modell des Items ändern", "Change item model"),
            ("Gibt dem Item in der Haupthand ein anderes Aussehen aus deinem Resourcepack. Leer = wieder normales Aussehen.",
             "Gives the item in the main hand another look from your resource pack. Empty = normal look again."))
            .f("model", ("Modell", "Model"), F::ItemModel, "")
            .i("player", p_player, T::Player, "")
            .both("Util.setHeldModel(${in:player}, ${s:model});"),
    );
    v.push(
        b("count_custom", C::Resources, K::Value,
            ("Anzahl eigener Items", "Custom item count"),
            ("Wie viele Items mit diesem Modell der Spieler im Inventar hat.", "How many items with this model the player has."))
            .f("model", ("Modell", "Model"), F::ItemModel, "")
            .i("player", p_player, T::Player, "")
            .o("count", ("Anzahl", "Count"), T::Number)
            .bout("count", "Util.countModel(${in:player}, ${s:model})"),
    );
    v.push(
        b("take_custom", C::Resources, K::Action,
            ("Eigene Items abziehen", "Take custom items"),
            ("Nimmt dem Spieler Items mit diesem Modell weg – nur wenn er genug hat. „Geklappt“ sagt, ob es funktioniert hat.",
             "Takes items with this model from the player – only if they have enough. “Worked” tells whether it worked."))
            .f("model", ("Modell", "Model"), F::ItemModel, "")
            .i("player", p_player, T::Player, "")
            .i("amount", ("Anzahl", "Amount"), T::Number, "1")
            .o("ok", ("Geklappt", "Worked"), T::Bool)
            .both("final boolean ${out:ok} = Util.takeModel(${in:player}, ${s:model}, ${in:amount});"),
    );
    v.push(
        b("equip_custom", C::Resources, K::Action,
            ("Eigenes Item anziehen", "Equip custom item"),
            ("Legt ein Item mit eigenem Modell direkt in einen Ausrüstungsplatz – z. B. einen Hut auf den Kopf.",
             "Puts an item with a custom model straight into an equipment slot – e.g. a hat on the head."))
            .f("model", ("Modell", "Model"), F::ItemModel, "")
            .f("slot", ("Platz", "Slot"), F::Choice(EQUIP_SLOTS), "HEAD")
            .i("player", p_player, T::Player, "")
            .i("item", ("Basis-Item", "Base item"), T::Text, "minecraft:paper")
            .h(("Das Vanilla-Item, das das Modell trägt.", "The vanilla item that carries the model."))
            .ia("name", ("Name", "Name"), T::Text, "")
            .both("Util.equipCustom(${in:player}, \"${f:slot}\", ${in:item}, ${s:model}, ${in:name});"),
    );
    v.push(
        b("drop_custom", C::Resources, K::Action,
            ("Eigenes Item fallen lassen", "Drop custom item"),
            ("Lässt ein Item mit eigenem Modell an einer Position auf den Boden fallen – z. B. als Belohnung.",
             "Drops an item with a custom model on the ground at a location – e.g. as a reward."))
            .f("model", ("Modell", "Model"), F::ItemModel, "")
            .i("loc", ("Position", "Location"), T::Location, "")
            .i("item", ("Basis-Item", "Base item"), T::Text, "minecraft:paper")
            .i("name", ("Name", "Name"), T::Text, "")
            .ia("amount", ("Anzahl", "Amount"), T::Number, "1")
            .both("Util.dropCustom(${in:loc}, ${in:item}, ${in:amount}, ${s:model}, ${in:name});"),
    );
    v.push(
        b("ev_pack_status", C::Resources, K::Event,
            ("Resourcepack-Status", "Resource pack status"),
            ("Wird ausgelöst, wenn ein Spieler das Resourcepack annimmt, ablehnt oder fertig geladen hat. Nur Paper.",
             "Fires when a player accepts, declines or has finished loading the resource pack. Paper only."))
            .o("player", p_player, T::Player)
            .o("loaded", ("Geladen", "Loaded"), T::Bool)
            .h(("Ja, sobald das Pack beim Spieler fertig geladen ist.", "Yes once the pack has finished loading for the player."))
            .o("declined", ("Abgelehnt", "Declined"), T::Bool)
            .h(("Ja, wenn der Spieler das Pack abgelehnt hat oder der Download fehlschlug.", "Yes if the player declined the pack or the download failed."))
            .o("status", ("Status", "Status"), T::Text)
            .h(("Der genaue Status: ACCEPTED, DECLINED, SUCCESSFULLY_LOADED, FAILED_DOWNLOAD …", "The exact status: ACCEPTED, DECLINED, SUCCESSFULLY_LOADED, FAILED_DOWNLOAD …"))
            .listener("org.bukkit.event.player.PlayerResourcePackStatusEvent", false)
            .pout("player", "event.getPlayer()")
            .pout("loaded", "(event.getStatus() == org.bukkit.event.player.PlayerResourcePackStatusEvent.Status.SUCCESSFULLY_LOADED)")
            .pout("declined", "(event.getStatus() == org.bukkit.event.player.PlayerResourcePackStatusEvent.Status.DECLINED || event.getStatus() == org.bukkit.event.player.PlayerResourcePackStatusEvent.Status.FAILED_DOWNLOAD)")
            .pout("status", "event.getStatus().name()"),
    );
    v.push(
        b("remove_pack", C::Resources, K::Action,
            ("Resourcepack entfernen", "Remove resource pack"),
            ("Nimmt dem Spieler die vom Server geschickten Resourcepacks wieder weg. Nur Paper.",
             "Removes the resource packs sent by the server from the player again. Paper only."))
            .i("player", p_player, T::Player, "")
            .paper("${in:player}.removeResourcePacks();"),
    );

    v.into_iter().map(|b| b.0).collect()
}

pub fn all() -> &'static [BlockDef] {
    static ALL: OnceLock<Vec<BlockDef>> = OnceLock::new();
    ALL.get_or_init(build_all)
}

/// Pins of a placed block. Command blocks get one output per command option.
pub struct Shape {
    pub def: &'static BlockDef,
    #[allow(dead_code)]
    pub inputs: Cow<'static, [PinDef]>,
    pub outputs: Cow<'static, [PinDef]>,
}

impl Shape {
    pub fn input(&self, id: &str) -> Option<&PinDef> {
        self.inputs.iter().find(|p| p.id == id)
    }
    pub fn output(&self, id: &str) -> Option<&PinDef> {
        self.outputs.iter().find(|p| p.id == id)
    }
}

pub fn opt_type(t: crate::model::OptType) -> PinType {
    use crate::model::OptType as O;
    match t {
        O::Word | O::Text | O::Choice => PinType::Text,
        O::Number | O::Integer => PinType::Number,
        O::Player => PinType::Player,
        O::Bool => PinType::Bool,
    }
}

pub fn shape(node: &crate::model::Node, project: &crate::model::Project) -> Option<Shape> {
    let def = get(&node.kind)?;
    if def.special == Special::Command {
        let mut outs: Vec<PinDef> = def.outputs.to_vec();
        if let Some(c) = project.node_command(node) {
            for (i, o) in c.options.iter().enumerate() {
                outs.push(PinDef {
                    id: Cow::Owned(crate::model::opt_pin(i)),
                    label: Label::D(o.name.clone()),
                    ty: opt_type(o.ty),
                    default: "",
                    advanced: false,
                    help: None,
                });
            }
        }
        return Some(Shape {
            def,
            inputs: Cow::Borrowed(&def.inputs),
            outputs: Cow::Owned(outs),
        });
    }
    Some(Shape {
        def,
        inputs: Cow::Borrowed(&def.inputs),
        outputs: Cow::Borrowed(&def.outputs),
    })
}

pub fn get(id: &str) -> Option<&'static BlockDef> {
    all().iter().find(|b| b.id == id)
}


/// One short sentence about a block's kind, shown on the block.
pub fn kind_label(k: Kind) -> L {
    match k {
        Kind::Event => ("Event", "Event"),
        Kind::Action => ("Aktion", "Action"),
        Kind::Flow => ("Ablauf", "Flow"),
        Kind::Value => ("Wert", "Value"),
    }
}

/// Explains what a socket does. Uses the pin's own text if it has one,
/// otherwise a description derived from the pin id and type.
pub fn pin_help(def: &BlockDef, pin: &PinDef, is_output: bool) -> String {
    use crate::i18n::t;
    if let Some(h) = pin.help {
        return crate::i18n::tl(h).to_string();
    }
    let id: &str = &pin.id;
    if pin.ty == PinType::Exec {
        return match (is_output, id, def.kind) {
            (false, _, _) => t(
                "Ablauf-Eingang: Der Block läuft, sobald der Ablauf hier ankommt. Verbinde ihn mit dem grünen Pfeil des Blocks davor.",
                "Flow input: the block runs as soon as the flow arrives here. Connect it to the green arrow of the block before.",
            ),
            (true, "next", Kind::Event) => t(
                "Ablauf-Ausgang: Hier startet alles, was passieren soll, wenn das Event ausgelöst wird.",
                "Flow output: everything that should happen when the event fires starts here.",
            ),
            (true, "next", _) => t(
                "Ablauf-Ausgang: Danach geht es mit dem hier verbundenen Block weiter.",
                "Flow output: afterwards the flow continues with the block connected here.",
            ),
            (true, "then", _) => t("Läuft, wenn die Bedingung stimmt.", "Runs if the condition is true."),
            (true, "else", _) => t("Läuft, wenn die Bedingung nicht stimmt.", "Runs if the condition is false."),
            (true, "body" | "loop" | "each", _) => t(
                "Läuft bei jedem Durchgang der Wiederholung.",
                "Runs on every round of the loop.",
            ),
            (true, "done", _) => t("Läuft, wenn alles fertig ist.", "Runs when everything is done."),
            (true, _, _) => t(
                "Ablauf-Ausgang: Diesen Zweig nimmt der Ablauf in diesem Fall.",
                "Flow output: the flow takes this branch in that case.",
            ),
        }
        .to_string();
    }
    if def.special == Special::Command && is_output && id.starts_with("opt") {
        return t(
            "Was der Spieler bei dieser Option des Befehls eingegeben hat. Optionen bearbeitest du im Befehle-Fenster.",
            "What the player typed for this option of the command. Edit options in the commands window.",
        )
        .to_string();
    }
    let s = match (is_output, id, pin.ty) {
        (true, "player", _) if def.kind == Kind::Event => t(
            "Der Spieler, der das Event ausgelöst hat. Verbinde ihn mit den Spieler-Eingängen anderer Blöcke – oder lass sie frei, dann wird er automatisch benutzt.",
            "The player who triggered the event. Connect it to player inputs of other blocks – or leave those empty, then it is used automatically.",
        ),
        (true, _, PinType::Player) => t(
            "Ein Spieler als Ergebnis dieses Blocks. Kann leer sein, wenn keiner gefunden wurde.",
            "A player as the result of this block. Can be empty if none was found.",
        ),
        (false, "player", _) => t(
            "Der Spieler, auf den dieser Block wirkt. Nicht verbunden = der Spieler aus dem Event.",
            "The player this block acts on. Not connected = the player from the event.",
        ),
        (false, "entity", _) => t(
            "Das Lebewesen, auf das dieser Block wirkt – auch ein Spieler passt hier.",
            "The entity this block acts on – a player fits here too.",
        ),
        (false, "loc", _) => t(
            "Die Position (Welt + Koordinaten), an der es passiert. Ein Spieler passt auch – dann zählt, wo er steht.",
            "The location (world + coordinates) where it happens. A player fits too – then where they stand counts.",
        ),
        (true, "loc", _) => t(
            "Die Position (Welt + Koordinaten). Verbinde sie mit Positions-Eingängen, z. B. „Teleportieren“ oder „Position speichern“.",
            "The location (world + coordinates). Connect it to location inputs, e.g. “Teleport” or “Save location”.",
        ),
        (false, "item", PinType::Text) => t(
            "Die ID eines Items, z. B. minecraft:diamond. Mit der Lupe daneben kannst du alle Items durchsuchen.",
            "The id of an item, e.g. minecraft:diamond. The magnifier next to it lets you search all items.",
        ),
        (false, "block", PinType::Text) => t(
            "Die ID eines Blocks, z. B. minecraft:stone. Mit der Lupe daneben kannst du alle Blöcke durchsuchen.",
            "The id of a block, e.g. minecraft:stone. The magnifier next to it lets you search all blocks.",
        ),
        (false, "amount" | "count", _) => t("Wie viele es sein sollen.", "How many it should be."),
        (false, "seconds", _) => t("Dauer in Sekunden (Kommazahlen wie 0.5 gehen auch).", "Duration in seconds (decimals like 0.5 work too)."),
        (false, "world", _) => t(
            "Name der Welt, z. B. world, world_nether oder world_the_end.",
            "Name of the world, e.g. world, world_nether or world_the_end.",
        ),
        (false, "key", _) => t(
            "Der Name, unter dem der Wert gespeichert ist. Derselbe Name liest ihn später wieder.",
            "The name the value is stored under. The same name reads it again later.",
        ),
        (false, "command", _) => t("Der Befehl ohne / am Anfang.", "The command without the leading /."),
        (false, "text" | "message", _) => t(
            "Der Text. Farben mit &-Codes: &a grün, &c rot, &e gelb, &l fett.",
            "The text. Colors with & codes: &a green, &c red, &e yellow, &l bold.",
        ),
        (false, "name" | "title" | "subtitle" | "lore" | "header" | "footer", PinType::Text) => t(
            "Text mit &-Farbcodes: &a grün, &c rot, &e gelb, &l fett.",
            "Text with & color codes: &a green, &c red, &e yellow, &l bold.",
        ),
        (false, "cond" | "condition", _) => t(
            "Die Bedingung: Ja oder Nein. Verbinde hier z. B. einen Vergleich.",
            "The condition: yes or no. Connect e.g. a comparison here.",
        ),
        (true, "res" | "value" | "result", PinType::Bool) => t(
            "Das Ergebnis: Ja oder Nein. Passt in „Wenn“ oder andere Ja/Nein-Eingänge.",
            "The result: yes or no. Fits into “If” or other yes/no inputs.",
        ),
        (true, _, PinType::Number) => t(
            "Eine Zahl als Ergebnis. Passt in Zahl- und Text-Eingänge.",
            "A number as the result. Fits into number and text inputs.",
        ),
        (true, _, PinType::Text) => t(
            "Ein Text als Ergebnis. Passt in Text-Eingänge (und in Zahl-Eingänge, wenn er eine Zahl ist).",
            "A text as the result. Fits into text inputs (and number inputs if it is a number).",
        ),
        (true, _, PinType::Bool) => t("Ja oder Nein als Ergebnis.", "Yes or no as the result."),
        (true, _, PinType::Location) => t("Eine Position als Ergebnis.", "A location as the result."),
        (true, _, _) => t("Ein Wert als Ergebnis dieses Blocks.", "A value produced by this block."),
        (false, _, PinType::Number) => t(
            "Eine Zahl. Tippe sie direkt ein oder verbinde einen Zahl-Ausgang.",
            "A number. Type it in or connect a number output.",
        ),
        (false, _, PinType::Text) => t(
            "Ein Text. Tippe ihn direkt ein oder verbinde einen Ausgang (Zahlen, Spieler und Positionen werden automatisch zu Text).",
            "A text. Type it in or connect an output (numbers, players and locations turn into text automatically).",
        ),
        (false, _, PinType::Bool) => t(
            "Ja oder Nein. Stelle den Schalter um oder verbinde einen Ja/Nein-Ausgang.",
            "Yes or no. Flip the switch or connect a yes/no output.",
        ),
        (false, _, PinType::Location) => t(
            "Eine Position. Verbinde einen Positions-Ausgang – oder einen Spieler, dann zählt, wo er steht.",
            "A location. Connect a location output – or a player, then where they stand counts.",
        ),
        (false, _, PinType::Player) => t(
            "Ein Spieler. Nicht verbunden = der Spieler aus dem Event.",
            "A player. Not connected = the player from the event.",
        ),
        (false, _, _) => t("Ein Wert, den dieser Block braucht.", "A value this block needs."),
    };
    s.to_string()
}

/// What can be plugged into / out of a pin type, for tooltips.
pub fn accepts(ty: PinType, is_output: bool) -> Vec<PinType> {
    use PinType::*;
    [Player, Entity, Text, Number, Bool, Location, Item]
        .into_iter()
        .filter(|o| {
            *o != ty
                && if is_output {
                    compatible(ty, *o)
                } else {
                    compatible(*o, ty)
                }
        })
        .collect()
}
