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
}

pub const CATEGORIES: [Cat; 11] = [
    Cat::Events,
    Cat::Messages,
    Cat::Player,
    Cat::World,
    Cat::Flow,
    Cat::Logic,
    Cat::Math,
    Cat::Text,
    Cat::Data,
    Cat::Resources,
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

struct B(BlockDef);

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
    B(d)
}

impl B {
    fn i(mut self, id: &'static str, label: L, ty: PinType, default: &'static str) -> Self {
        self.0.inputs.push(PinDef {
            id: Cow::Borrowed(id),
            label: Label::S(label),
            ty,
            default,
            advanced: false,
        });
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
        });
        self
    }
    fn o(mut self, id: &'static str, label: L, ty: PinType) -> Self {
        self.0.outputs.push(PinDef {
            id: Cow::Borrowed(id),
            label: Label::S(label),
            ty,
            default: "",
            advanced: false,
        });
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
            ("Öffnet ein Kisten-Menü mit Items, z. B. für Shops oder Teleporter. Eine Zeile pro Item: „Platz: item | Name | Beschreibung“, z. B. „13: minecraft:diamond | &bDiamant | &7Kostet 5 Gold“. Klicks fängst du mit „Klick im Menü“ ab. Nur Paper.",
             "Opens a chest menu with items, e.g. for shops or teleporters. One line per item: “slot: item | name | lore”, e.g. “13: minecraft:diamond | &bDiamond | &7Costs 5 gold”. Catch clicks with “Menu click”. Paper only."))
            .f("menu", ("Menü-Name", "Menu name"), F::Text, "shop")
            .f("items", ("Items", "Items"), F::Multiline, "11: minecraft:diamond | &bDiamant\n15: minecraft:emerald | &aSmaragd")
            .i("player", p_player, T::Player, "")
            .i("title", ("Titel", "Title"), T::Text, "&8Shop")
            .i("rows", ("Reihen", "Rows"), T::Number, "3")
            .paper("Util.openMenu(${in:player}, ${s:menu}, ${in:title}, ${in:rows}, ${s:items});"),
    );
    v.push(
        b("ev_menu_click", C::Events, K::Event,
            ("Klick im Menü", "Menu click"),
            ("Wird ausgelöst, wenn ein Spieler in einem Menü von „Menü öffnen“ klickt. Items können dort nicht herausgenommen werden. Nur Paper.",
             "Fires when a player clicks in a menu from “Open menu”. Items can't be taken out. Paper only."))
            .f("menu", ("Menü-Name", "Menu name"), F::Text, "shop")
            .o("player", p_player, T::Player)
            .o("slot", ("Platz", "Slot"), T::Number)
            .o("item", ("Item-ID", "Item id"), T::Text)
            .listener("org.bukkit.event.inventory.InventoryClickEvent", false)
            .paper("if (!(event.getInventory().getHolder() instanceof Util.MenuHolder __mh) || !__mh.id.equals(${s:menu})) return;\n    event.setCancelled(true);\n    if (event.getClickedInventory() != event.getView().getTopInventory()) return;")
            .pout("player", "(org.bukkit.entity.Player) event.getWhoClicked()")
            .pout("slot", "event.getSlot()")
            .pout("item", "Util.itemId(event.getCurrentItem())"),
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
