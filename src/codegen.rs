//! Turns a project (block graphs) into a complete Gradle project with Java code.

use crate::blocks::{self, BlockDef, FieldKind, Kind, PinType, Special};
use crate::model::{Node, NodeId, Platform, Project, Workspace};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub ws: usize,
    pub node: Option<NodeId>,
    pub msg: String,
    pub error: bool,
}

pub struct Output {
    /// Relative path -> file content.
    pub files: Vec<(String, Vec<u8>)>,
    pub problems: Vec<Problem>,
    /// The generated main class, for the code preview.
    pub main_java: String,
}

impl Output {
    pub fn has_errors(&self) -> bool {
        self.problems.iter().any(|p| p.error)
    }
}

// ---------------------------------------------------------------------------
// Embedded templates
// ---------------------------------------------------------------------------

const WRAPPER_JAR: &[u8] = include_bytes!("../assets/wrapper/gradle-wrapper.jar");
const GRADLEW: &str = include_str!("../assets/wrapper/gradlew");
const GRADLEW_BAT: &str = include_str!("../assets/wrapper/gradlew.bat");
const DAEMON_JVM_25: &str = include_str!("../assets/wrapper/daemon-jvm-25.properties");
const DAEMON_JVM_21: &str = include_str!("../assets/wrapper/daemon-jvm-21.properties");
const STORAGE_JAVA: &str = include_str!("../assets/templates/common/Storage.java");
const UTIL_COMMON: &str = include_str!("../assets/templates/common/UtilCommon.java.part");
const PAPER_UTIL: &str = include_str!("../assets/templates/paper/Util.java");
const FABRIC_UTIL: &str = include_str!("../assets/templates/fabric/Util.java");
const FABRIC_LOC: &str = include_str!("../assets/templates/fabric/Loc.java");
const FABRIC_SCHEDULER: &str = include_str!("../assets/templates/fabric/Scheduler.java");

const GRADLE_VERSION: &str = "9.8.2";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Java string literal with escaping.
pub fn java_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => {}
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                o.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn yaml_str(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''").replace('\n', " "))
}

fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// Number literal as Java double.
fn java_num(s: &str) -> String {
    let v: f64 = s.trim().replace(',', ".").parse().unwrap_or(0.0);
    let v = if v.is_finite() { v } else { 0.0 };
    let mut t = format!("{v}");
    if !t.contains('.') && !t.contains('e') {
        t.push_str(".0");
    }
    t
}

fn indent(lines: &[String], n: usize) -> Vec<String> {
    let pad = " ".repeat(n);
    lines
        .iter()
        .map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("{pad}{l}")
            }
        })
        .collect()
}

pub fn value_of<'a>(node: &'a Node, id: &str, default: &'a str) -> &'a str {
    node.values.get(id).map(|s| s.as_str()).unwrap_or(default)
}

/// Valid command name: lowercase letters, digits, `_` and `-`.
pub fn clean_command(s: &str) -> String {
    s.trim()
        .trim_start_matches('/')
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

const EXEC_MARK: char = '\u{1}';

// ---------------------------------------------------------------------------
// Graph -> Java
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Scope {
    /// (node, output pin) -> (variable name, type), values available here.
    vars: HashMap<(NodeId, String), (String, PinType)>,
    /// Variable holding the event's player, used for unconnected player inputs.
    player: Option<String>,
    cancellable: bool,
    /// Statement used by the "Stop" block.
    ret: String,
}

struct Gen<'a> {
    p: Platform,
    main: String,
    problems: Vec<Problem>,
    imports: BTreeSet<String>,
    ws_idx: usize,
    ws: &'a Workspace,
    /// Nodes that ended up in generated code.
    reached: HashSet<NodeId>,
    reported: HashSet<(NodeId, String)>,
}

impl<'a> Gen<'a> {
    fn err(&mut self, node: NodeId, msg: String) {
        if self.reported.insert((node, msg.clone())) {
            self.problems.push(Problem {
                ws: self.ws_idx,
                node: Some(node),
                msg,
                error: true,
            });
        }
    }
    fn warn(&mut self, node: NodeId, msg: String) {
        if self.reported.insert((node, msg.clone())) {
            self.problems.push(Problem {
                ws: self.ws_idx,
                node: Some(node),
                msg,
                error: false,
            });
        }
    }

    fn def_of(&mut self, node: &Node) -> Option<&'static BlockDef> {
        let d = blocks::get(&node.kind);
        if d.is_none() {
            self.err(node.id, format!("Unbekannter Block „{}“", node.kind));
        }
        d
    }

    fn check_platform(&mut self, node: &Node, def: &BlockDef) -> bool {
        if !def.supports(self.p) {
            self.err(
                node.id,
                format!("„{}“ gibt es für {} nicht", def.name.0, self.p.short()),
            );
            return false;
        }
        true
    }

    /// Generates the code for everything connected to the exec output `pin` of `node`.
    fn chain(
        &mut self,
        node: NodeId,
        pin: &str,
        scope: &Scope,
        stack: &mut Vec<NodeId>,
    ) -> Vec<String> {
        let ws = self.ws;
        let mut targets: Vec<&Node> = ws
            .links_from(node, pin)
            .filter_map(|l| ws.node(l.to.node))
            .collect();
        targets.sort_by(|a, b| a.pos[1].total_cmp(&b.pos[1]));
        let mut out = vec![];
        let mut scope = scope.clone();
        for t in targets {
            out.extend(self.statement(t, &mut scope, stack));
        }
        out
    }

    fn statement(
        &mut self,
        node: &'a Node,
        scope: &mut Scope,
        stack: &mut Vec<NodeId>,
    ) -> Vec<String> {
        let Some(def) = self.def_of(node) else {
            return vec![];
        };
        if stack.contains(&node.id) {
            self.err(
                node.id,
                "Endlosschleife: der Ablauf führt zu sich selbst zurück".into(),
            );
            return vec![];
        }
        if !matches!(def.kind, Kind::Action | Kind::Flow) {
            return vec![];
        }
        self.reached.insert(node.id);
        if !self.check_platform(node, def) {
            return vec![];
        }
        let code = def.code(self.p).unwrap().code;
        if node.kind == "cancel_event" && !scope.cancellable {
            self.err(node.id, "Dieses Event kann nicht abgebrochen werden".into());
            return vec![];
        }
        stack.push(node.id);

        // Outputs of this block.
        let mut inner = scope.clone();
        for o in def.outputs.iter().filter(|o| o.ty != PinType::Exec) {
            inner.vars.insert(
                (node.id, o.id.to_string()),
                (format!("v{}_{}", node.id, o.id), o.ty),
            );
        }

        let (text, exec_slots) = self.subst(code, node, def, &inner, true);

        let mut lines = vec![];
        for line in text.lines() {
            let tl = line.trim();
            if let Some(slot) = tl
                .strip_prefix(EXEC_MARK)
                .and_then(|s| s.strip_suffix(EXEC_MARK))
            {
                let mut branch = inner.clone();
                if node.kind == "wait" {
                    branch.ret = "return;".into();
                }
                let body = self.chain(node.id, slot, &branch, stack);
                lines.extend(indent(&body, 4));
            } else {
                lines.push(line.to_string());
            }
        }

        let has_next = def
            .outputs
            .iter()
            .any(|o| o.id == "next" && o.ty == PinType::Exec);
        if has_next && !exec_slots.contains("next") {
            // Actions keep their outputs available afterwards, loops do not.
            if def.kind == Kind::Action {
                *scope = inner;
            }
            stack.pop();
            let rest = self.chain(node.id, "next", scope, stack);
            lines.extend(rest);
            return lines;
        }
        stack.pop();
        lines
    }

    /// Replaces `${...}` placeholders. Exec slots become marker lines.
    fn subst(
        &mut self,
        tpl: &str,
        node: &Node,
        def: &BlockDef,
        scope: &Scope,
        allow_exec: bool,
    ) -> (String, HashSet<String>) {
        let mut out = String::with_capacity(tpl.len() + 32);
        let mut slots = HashSet::new();
        let mut rest = tpl;
        while let Some(start) = rest.find("${") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let Some(end) = after.find('}') else {
                out.push_str(&rest[start..]);
                rest = "";
                break;
            };
            let token = &after[..end];
            rest = &after[end + 1..];
            let (kind, arg) = token.split_once(':').unwrap_or((token, ""));
            match kind {
                "in" => {
                    let e = self.input_expr(node, def, arg, scope);
                    out.push_str(&e);
                }
                "f" => out.push_str(&self.field_raw(node, def, arg)),
                "s" => {
                    let default = def.field(arg).map(|f| f.default).unwrap_or("");
                    out.push_str(&java_str(value_of(node, arg, default)));
                }
                "out" => out.push_str(&format!("v{}_{}", node.id, arg)),
                "exec" if allow_exec => {
                    slots.insert(arg.to_string());
                    out.push(EXEC_MARK);
                    out.push_str(arg);
                    out.push(EXEC_MARK);
                }
                "server" => out.push_str(match self.p {
                    Platform::Paper => "org.bukkit.Bukkit.getServer()",
                    Platform::Fabric => "Util.SERVER",
                }),
                "plugin" => out.push_str(&format!("{}.INSTANCE", self.main)),
                "main" => out.push_str(&self.main),
                "id" => out.push_str(&node.id.to_string()),
                "return" => out.push_str(&scope.ret),
                "storekey" => {
                    let key = self.input_expr(node, def, "key", scope);
                    if value_of(node, "scope", "player") == "player" {
                        let pl = self.input_expr(node, def, "player", scope);
                        out.push_str(&format!("Util.uuid({pl}) + \".\" + {key}"));
                    } else {
                        out.push_str(&format!("\"global.\" + {key}"));
                    }
                }
                _ => {
                    out.push_str("${");
                    out.push_str(token);
                    out.push('}');
                }
            }
        }
        out.push_str(rest);
        (out, slots)
    }

    fn field_raw(&mut self, node: &Node, def: &BlockDef, id: &str) -> String {
        let Some(f) = def.field(id) else {
            return String::new();
        };
        let v = value_of(node, id, f.default);
        match &f.kind {
            FieldKind::Number => java_num(v),
            FieldKind::Bool => {
                if v == "true" {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            FieldKind::Choice(opts) => {
                if opts.iter().any(|(k, _)| *k == v) {
                    v.to_string()
                } else {
                    f.default.to_string()
                }
            }
            FieldKind::Text | FieldKind::Multiline => v.to_string(),
        }
    }

    /// Java expression for the data input `pin` of `node`, converted to the pin's type.
    fn input_expr(&mut self, node: &Node, def: &BlockDef, pin: &str, scope: &Scope) -> String {
        let Some(pd) = def.input(pin) else {
            return "null".into();
        };
        let ws = self.ws;
        if let Some(link) = ws.link_into(node.id, pin) {
            if let Some(src) = ws.node(link.from.node) {
                if let Some(sdef) = blocks::get(&src.kind) {
                    if let Some(sp) = sdef.output(&link.from.pin) {
                        let from_ty = sp.ty;
                        let raw = self.output_expr(src, sdef, &link.from.pin, scope, 0);
                        return match raw {
                            Some(e) => blocks::convert(&e, from_ty, pd.ty, self.p),
                            None => self.fallback(node, pd),
                        };
                    }
                }
            }
        }
        // Not connected.
        match pd.ty {
            PinType::Text => java_str(value_of(node, pin, pd.default)),
            PinType::Number => java_num(value_of(node, pin, pd.default)),
            PinType::Bool => {
                if value_of(node, pin, pd.default) == "true" {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            PinType::Player | PinType::Entity | PinType::Location if scope.player.is_some() => {
                let v = scope.player.clone().unwrap();
                blocks::convert(&v, PinType::Player, pd.ty, self.p)
            }
            _ => {
                if !(pin == "player"
                    && node.values.get("scope").map(|s| s.as_str()) == Some("global"))
                {
                    self.err(
                        node.id,
                        format!("Eingang „{}“ ist nicht verbunden", pd.label.0),
                    );
                }
                "null".into()
            }
        }
    }

    fn fallback(&self, _node: &Node, pd: &blocks::PinDef) -> String {
        match pd.ty {
            PinType::Text => "\"\"".into(),
            PinType::Number => "0.0".into(),
            PinType::Bool => "false".into(),
            _ => "null".into(),
        }
    }

    /// Expression for an output of `src`. Value blocks are inlined, everything else must be in scope.
    fn output_expr(
        &mut self,
        src: &'a Node,
        sdef: &'static BlockDef,
        pin: &str,
        scope: &Scope,
        depth: usize,
    ) -> Option<String> {
        if let Some((v, _)) = scope.vars.get(&(src.id, pin.to_string())) {
            return Some(v.clone());
        }
        if sdef.kind != Kind::Value {
            self.err(
                src.id,
                format!(
                    "Der Wert „{}“ wird an einer Stelle benutzt, an der „{}“ noch nicht gelaufen ist (anderer Ablauf?)",
                    sdef.output(pin).map(|p| p.label.0).unwrap_or(pin),
                    sdef.name.0
                ),
            );
            return None;
        }
        if depth > 40 {
            self.err(src.id, "Werte sind im Kreis verbunden".into());
            return None;
        }
        self.reached.insert(src.id);
        if !self.check_platform(src, sdef) {
            return None;
        }
        let code = sdef.code(self.p).unwrap();
        let tpl = code.outs.iter().find(|(p, _)| *p == pin).map(|(_, e)| *e)?;
        // Inputs of value blocks are resolved recursively through input_expr.
        let _ = depth;
        let (text, _) = self.subst(tpl, src, sdef, scope, false);
        Some(text)
    }

    /// Prelude lines that declare event outputs as variables.
    fn event_vars(
        &mut self,
        node: &Node,
        def: &BlockDef,
        scope: &mut Scope,
        exprs: &[(&str, String)],
    ) -> Vec<String> {
        let mut lines = vec![];
        for o in def.outputs.iter().filter(|o| o.ty != PinType::Exec) {
            let Some((_, e)) = exprs.iter().find(|(p, _)| *p == o.id) else {
                continue;
            };
            let var = format!("v{}_{}", node.id, o.id);
            lines.push(format!("final {} {} = {};", o.ty.java(self.p), var, e));
            if o.ty == PinType::Player {
                scope.player = Some(var.clone());
            }
            scope.vars.insert((node.id, o.id.to_string()), (var, o.ty));
        }
        lines
    }
}

struct Parts {
    /// Code inside onEnable / onInitialize.
    init: Vec<String>,
    /// Extra methods / fields of the main class.
    members: Vec<String>,
    /// Paper: plugin.yml command entries (name, description, op).
    commands: Vec<(String, String, bool)>,
}

fn new_scope(ret: &str) -> Scope {
    Scope {
        vars: HashMap::new(),
        player: None,
        cancellable: false,
        ret: ret.into(),
    }
}

fn gen_workspace(g: &mut Gen, parts: &mut Parts, seen_cmds: &mut HashMap<String, NodeId>) {
    let ws = g.ws;
    let mut events: Vec<&Node> = ws
        .nodes
        .iter()
        .filter(|n| {
            blocks::get(&n.kind)
                .map(|d| d.kind == Kind::Event)
                .unwrap_or(false)
        })
        .collect();
    events.sort_by(|a, b| {
        a.pos[1]
            .total_cmp(&b.pos[1])
            .then(a.pos[0].total_cmp(&b.pos[0]))
    });

    for ev in events {
        let def = blocks::get(&ev.kind).unwrap();
        g.reached.insert(ev.id);
        if !g.check_platform(ev, def) {
            continue;
        }
        let code = def.code(g.p).unwrap().clone();
        let title = format!("// {} (#{})", def.name.0, ev.id);
        match def.special {
            Special::Start => {
                let mut sc = new_scope("return;");
                let body = g.chain(ev.id, "next", &sc.clone(), &mut vec![]);
                sc.vars.clear();
                if body.is_empty() {
                    continue;
                }
                match g.p {
                    Platform::Paper => {
                        parts.init.push(title);
                        parts.init.push("{".into());
                        parts.init.extend(indent(&body, 4));
                        parts.init.push("}".into());
                    }
                    Platform::Fabric => {
                        for i in &code.imports {
                            g.imports.insert(i.to_string());
                        }
                        parts.init.push(title);
                        let tpl = code.code;
                        for l in tpl.lines() {
                            if l.trim() == "${body}" {
                                parts.init.extend(indent(&body, 4));
                            } else {
                                parts.init.push(l.to_string());
                            }
                        }
                    }
                }
            }
            Special::Timer => {
                let secs: f64 = value_of(ev, "seconds", "60")
                    .replace(',', ".")
                    .parse()
                    .unwrap_or(60.0);
                let ticks = ((secs * 20.0).round() as i64).max(1);
                let sc = new_scope("return;");
                let body = g.chain(ev.id, "next", &sc, &mut vec![]);
                if body.is_empty() {
                    continue;
                }
                parts.init.push(title);
                match g.p {
                    Platform::Paper => {
                        parts.init.push(
                            "org.bukkit.Bukkit.getScheduler().runTaskTimer(this, () -> {".into(),
                        );
                        parts.init.extend(indent(&body, 4));
                        parts.init.push(format!("}}, {ticks}L, {ticks}L);"));
                    }
                    Platform::Fabric => {
                        g.imports.insert(
                            "net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents".into(),
                        );
                        parts
                            .members
                            .push(format!("private static int timer{} = 0;", ev.id));
                        parts
                            .init
                            .push("ServerTickEvents.END_SERVER_TICK.register(__server -> {".into());
                        parts
                            .init
                            .push(format!("    if (++timer{} < {ticks}) return;", ev.id));
                        parts.init.push(format!("    timer{} = 0;", ev.id));
                        parts.init.extend(indent(&body, 4));
                        parts.init.push("});".into());
                    }
                }
            }
            Special::Command => {
                let name = clean_command(value_of(ev, "name", "hallo"));
                if name.is_empty() {
                    g.err(ev.id, "Der Befehl braucht einen Namen".into());
                    continue;
                }
                if let Some(other) = seen_cmds.get(&name) {
                    if *other != ev.id {
                        g.err(ev.id, format!("Den Befehl /{name} gibt es doppelt"));
                        continue;
                    }
                }
                seen_cmds.insert(name.clone(), ev.id);
                let desc = value_of(ev, "description", "").to_string();
                let op = value_of(ev, "op", "false") == "true";
                let players_only = value_of(ev, "players_only", "true") == "true";
                let ret = match g.p {
                    Platform::Paper => "return true;",
                    Platform::Fabric => "return 1;",
                };
                let mut sc = new_scope(ret);
                let player_expr = match g.p {
                    Platform::Paper => {
                        "sender instanceof org.bukkit.entity.Player __pl ? __pl : null".to_string()
                    }
                    Platform::Fabric => "source.getPlayer()".to_string(),
                };
                let exprs = vec![
                    ("player", player_expr),
                    ("arg1", "Util.arg(args, 0)".to_string()),
                    ("arg2", "Util.arg(args, 1)".to_string()),
                    ("args", "String.join(\" \", args)".to_string()),
                    ("argc", "args.length".to_string()),
                ];
                let mut pre = g.event_vars(ev, def, &mut sc, &exprs);
                let pvar = format!("v{}_player", ev.id);
                if players_only {
                    match g.p {
                        Platform::Paper => pre.push(format!(
                            "if ({pvar} == null) {{ sender.sendMessage(\"Only players can use this command.\"); return true; }}"
                        )),
                        Platform::Fabric => pre.push(format!(
                            "if ({pvar} == null) {{ source.sendFailure(Component.literal(\"Only players can use this command.\")); return 0; }}"
                        )),
                    }
                } else {
                    // The player may be null: do not use it automatically.
                    sc.player = None;
                }
                let body = g.chain(ev.id, "next", &sc, &mut vec![]);
                let mut all = pre;
                all.extend(body);
                parts.init.push(title);
                match g.p {
                    Platform::Paper => {
                        parts.commands.push((name.clone(), desc, op));
                        parts.init.push(format!(
                            "java.util.Objects.requireNonNull(getCommand({})).setExecutor((sender, command, label, args) -> {{",
                            java_str(&name)
                        ));
                        parts.init.extend(indent(&all, 4));
                        parts.init.push("    return true;".into());
                        parts.init.push("});".into());
                    }
                    Platform::Fabric => {
                        for i in [
                            "net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback",
                            "net.minecraft.commands.Commands",
                            "net.minecraft.commands.CommandSourceStack",
                            "com.mojang.brigadier.arguments.StringArgumentType",
                        ] {
                            g.imports.insert(i.into());
                        }
                        let m = format!("cmd{}", ev.id);
                        parts.init.push("CommandRegistrationCallback.EVENT.register((dispatcher, registryAccess, environment) -> {".into());
                        parts.init.push(format!(
                            "    dispatcher.register(Commands.literal({})",
                            java_str(&name)
                        ));
                        if op {
                            parts.init.push("        .requires(Util::isOp)".into());
                        }
                        parts.init.push(format!(
                            "        .executes(ctx -> {m}(ctx.getSource(), new String[0]))"
                        ));
                        parts.init.push("        .then(Commands.argument(\"args\", StringArgumentType.greedyString())".into());
                        parts.init.push(format!(
                            "            .executes(ctx -> {m}(ctx.getSource(), StringArgumentType.getString(ctx, \"args\").trim().split(\"\\\\s+\")))));"
                        ));
                        parts.init.push("});".into());
                        let mut mlines = vec![
                            format!("// /{name} (#{})", ev.id),
                            format!(
                                "private static int {m}(final CommandSourceStack source, final String[] args) {{"
                            ),
                        ];
                        mlines.extend(indent(&all, 4));
                        mlines.push("    return 1;".into());
                        mlines.push("}".into());
                        parts.members.push(mlines.join("\n"));
                    }
                }
            }
            Special::None => match g.p {
                Platform::Paper => {
                    let mut sc = new_scope("return;");
                    sc.cancellable = code.cancellable;
                    let exprs: Vec<(&str, String)> =
                        code.outs.iter().map(|(p, e)| (*p, e.to_string())).collect();
                    let pre = g.event_vars(ev, def, &mut sc, &exprs);
                    let body = g.chain(ev.id, "next", &sc, &mut vec![]);
                    if body.is_empty() {
                        continue;
                    }
                    let mut m = vec![
                        title,
                        "@org.bukkit.event.EventHandler".to_string(),
                        format!("public void on{}(final {} event) {{", ev.id, code.listener),
                    ];
                    if !code.code.is_empty() {
                        m.push(format!("    {}", code.code));
                    }
                    m.extend(indent(&pre, 4));
                    m.extend(indent(&body, 4));
                    m.push("}".into());
                    parts.members.push(m.join("\n"));
                }
                Platform::Fabric => {
                    let ret = if code.ret.is_empty() {
                        "return;"
                    } else {
                        code.ret
                    };
                    let mut sc = new_scope(ret);
                    sc.cancellable = code.cancellable;
                    let exprs: Vec<(&str, String)> =
                        code.outs.iter().map(|(p, e)| (*p, e.to_string())).collect();
                    let mut pre = vec![];
                    if code.cancellable {
                        pre.push("final boolean[] __cancel = { false };".to_string());
                    }
                    pre.extend(g.event_vars(ev, def, &mut sc, &exprs));
                    let body = g.chain(ev.id, "next", &sc, &mut vec![]);
                    if body.is_empty() {
                        continue;
                    }
                    for i in &code.imports {
                        g.imports.insert(i.to_string());
                    }
                    let mut all = pre;
                    all.extend(body);
                    parts.init.push(title);
                    for l in code.code.lines() {
                        if l.trim() == "${body}" {
                            parts.init.extend(indent(&all, 4));
                        } else {
                            parts.init.push(l.to_string());
                        }
                    }
                }
            },
        }
    }

    // Blocks that never run.
    for n in &ws.nodes {
        if g.reached.contains(&n.id) {
            continue;
        }
        if let Some(d) = blocks::get(&n.kind) {
            if matches!(d.kind, Kind::Action | Kind::Flow) {
                g.warn(
                    n.id,
                    "Wird nie ausgeführt: nicht mit einem Event verbunden".into(),
                );
            }
            if !d.supports(g.p) {
                g.err(
                    n.id,
                    format!("„{}“ gibt es für {} nicht", d.name.0, g.p.short()),
                );
            }
        } else {
            g.err(n.id, format!("Unbekannter Block „{}“", n.kind));
        }
    }
}

// ---------------------------------------------------------------------------
// Project files
// ---------------------------------------------------------------------------

pub fn generate(proj: &Project) -> Output {
    let s = &proj.settings;
    let p = s.platform;
    let main = s.main_class();
    let mut problems = vec![];
    let mut imports = BTreeSet::new();
    let mut parts = Parts {
        init: vec![],
        members: vec![],
        commands: vec![],
    };
    let mut seen_cmds = HashMap::new();

    if !valid_package(&s.package) {
        problems.push(Problem {
            ws: 0,
            node: None,
            msg: format!(
                "Ungültiger Java-Paketname „{}“ (Beispiel: de.meinname.plugin)",
                s.package
            ),
            error: true,
        });
    }

    for (i, ws) in proj.workspaces.iter().enumerate() {
        let mut g = Gen {
            p,
            main: main.clone(),
            problems: vec![],
            imports: BTreeSet::new(),
            ws_idx: i,
            ws,
            reached: HashSet::new(),
            reported: HashSet::new(),
        };
        gen_workspace(&mut g, &mut parts, &mut seen_cmds);
        problems.extend(g.problems);
        imports.extend(g.imports);
    }

    let pkg = s.package.clone();
    let pkg_path = pkg.replace('.', "/");
    let mut files: Vec<(String, Vec<u8>)> = vec![];
    let mut add = |path: String, content: String| files.push((path, content.into_bytes()));

    let fill = |t: &str| {
        t.replace("__PKG__", &pkg)
            .replace("__MAIN__", &main)
            .replace("__COMMON__", UTIL_COMMON)
    };

    let main_java = match p {
        Platform::Paper => paper_main(&pkg, &main, &parts),
        Platform::Fabric => fabric_main(&pkg, &main, &s.id, &parts, &imports),
    };

    // Gradle wrapper
    add("gradlew".into(), GRADLEW.into());
    add("gradlew.bat".into(), GRADLEW_BAT.into());
    add(
        "gradle/wrapper/gradle-wrapper.properties".into(),
        format!(
            "distributionBase=GRADLE_USER_HOME\ndistributionPath=wrapper/dists\ndistributionUrl=https\\://services.gradle.org/distributions/gradle-{GRADLE_VERSION}-bin.zip\nnetworkTimeout=10000\nvalidateDistributionUrl=true\nzipStoreBase=GRADLE_USER_HOME\nzipStorePath=wrapper/dists\n"
        ),
    );
    add(
        "gradle/gradle-daemon-jvm.properties".into(),
        if s.versions.java >= 25 {
            DAEMON_JVM_25.into()
        } else {
            DAEMON_JVM_21.into()
        },
    );
    add(
        ".gitignore".into(),
        ".gradle/\nbuild/\nrun/\n*.iml\n.idea/\n".into(),
    );

    match p {
        Platform::Paper => {
            add(
                "settings.gradle.kts".into(),
                format!(
                    "plugins {{\n    id(\"org.gradle.toolchains.foojay-resolver-convention\") version \"1.0.0\"\n}}\n\nrootProject.name = {}\n",
                    json_str(&s.id)
                ),
            );
            add(
                "build.gradle.kts".into(),
                format!(
                    r#"// Generated by Minecraft Builder
plugins {{
    java
}}

group = {group}
version = {version}

repositories {{
    mavenCentral()
    maven("https://repo.papermc.io/repository/maven-public/")
}}

dependencies {{
    compileOnly("io.papermc.paper:paper-api:{api}")
}}

java {{
    toolchain.languageVersion.set(JavaLanguageVersion.of({java}))
}}

tasks.withType<JavaCompile>().configureEach {{
    options.encoding = "UTF-8"
    options.release.set({java})
}}

tasks.jar {{
    archiveFileName.set("{id}-${{project.version}}.jar")
}}
"#,
                    group = json_str(&pkg),
                    version = json_str(&s.version),
                    api = s.versions.api.trim(),
                    java = s.versions.java,
                    id = s.id,
                ),
            );
            add(
                "gradle.properties".into(),
                "org.gradle.jvmargs=-Xmx1G\n".into(),
            );
            let mut yml = format!(
                "# Generated by Minecraft Builder\nname: {}\nversion: {}\nmain: {}.{}\napi-version: {}\nauthors: [{}]\ndescription: {}\n",
                yaml_str(&s.name.replace(' ', "_")),
                yaml_str(&s.version),
                pkg,
                main,
                yaml_str(&api_version(&s.versions.minecraft)),
                yaml_str(&s.author),
                yaml_str(&s.description),
            );
            if !parts.commands.is_empty() {
                yml.push_str("commands:\n");
                for (name, desc, op) in &parts.commands {
                    yml.push_str(&format!("  {name}:\n    description: {}\n", yaml_str(desc)));
                    if *op {
                        yml.push_str(&format!("    permission: {}.command.{name}\n", s.id));
                    }
                }
                let ops: Vec<_> = parts.commands.iter().filter(|c| c.2).collect();
                if !ops.is_empty() {
                    yml.push_str("permissions:\n");
                    for (name, _, _) in ops {
                        yml.push_str(&format!("  {}.command.{name}:\n    default: op\n", s.id));
                    }
                }
            }
            add("src/main/resources/plugin.yml".into(), yml);
            add(
                format!("src/main/java/{pkg_path}/{main}.java"),
                main_java.clone(),
            );
            add(
                format!("src/main/java/{pkg_path}/Util.java"),
                fill(PAPER_UTIL),
            );
            add(
                format!("src/main/java/{pkg_path}/Storage.java"),
                fill(STORAGE_JAVA),
            );
        }
        Platform::Fabric => {
            add(
                "settings.gradle".into(),
                format!(
                    "pluginManagement {{\n    repositories {{\n        maven {{\n            name = 'Fabric'\n            url = 'https://maven.fabricmc.net/'\n        }}\n        mavenCentral()\n        gradlePluginPortal()\n    }}\n}}\n\nplugins {{\n    id 'org.gradle.toolchains.foojay-resolver-convention' version '1.0.0'\n}}\n\nrootProject.name = {}\n",
                    json_str(&s.id).replace('"', "'")
                ),
            );
            add(
                "gradle.properties".into(),
                format!(
                    "# Generated by Minecraft Builder\norg.gradle.jvmargs=-Xmx2G\norg.gradle.parallel=true\n\nminecraft_version={}\nloader_version={}\nloom_version={}\nfabric_api_version={}\n\nmod_version={}\nmaven_group={}\narchives_base_name={}\n",
                    s.versions.minecraft.trim(),
                    s.versions.loader.trim(),
                    s.versions.loom.trim(),
                    s.versions.api.trim(),
                    s.version.trim(),
                    pkg,
                    s.id
                ),
            );
            add(
                "build.gradle".into(),
                format!(
                    r#"// Generated by Minecraft Builder
plugins {{
    id 'net.fabricmc.fabric-loom' version "${{loom_version}}"
}}

version = project.mod_version
group = project.maven_group

base {{
    archivesName = project.archives_base_name
}}

dependencies {{
    minecraft "com.mojang:minecraft:${{project.minecraft_version}}"
    implementation "net.fabricmc:fabric-loader:${{project.loader_version}}"
    implementation "net.fabricmc.fabric-api:fabric-api:${{project.fabric_api_version}}"
}}

processResources {{
    def version = project.version
    inputs.property "version", version
    filesMatching("fabric.mod.json") {{
        expand "version": version
    }}
}}

java {{
    toolchain {{
        languageVersion = JavaLanguageVersion.of({java})
    }}
}}

tasks.withType(JavaCompile).configureEach {{
    it.options.release = {java}
    it.options.encoding = "UTF-8"
}}
"#,
                    java = s.versions.java
                ),
            );
            add(
                "src/main/resources/fabric.mod.json".into(),
                format!(
                    r#"{{
  "schemaVersion": 1,
  "id": {id},
  "version": "${{version}}",
  "name": {name},
  "description": {desc},
  "authors": [{author}],
  "license": "All-Rights-Reserved",
  "environment": "*",
  "entrypoints": {{
    "main": [{entry}]
  }},
  "depends": {{
    "fabricloader": ">={loader}",
    "minecraft": "~{mc}",
    "java": ">={java}",
    "fabric-api": "*"
  }}
}}
"#,
                    id = json_str(&s.id),
                    name = json_str(&s.name),
                    desc = json_str(&s.description),
                    author = json_str(&s.author),
                    entry = json_str(&format!("{pkg}.{main}")),
                    loader = s.versions.loader.trim(),
                    mc = s.versions.minecraft.trim(),
                    java = s.versions.java,
                ),
            );
            add(
                format!("src/main/java/{pkg_path}/{main}.java"),
                main_java.clone(),
            );
            add(
                format!("src/main/java/{pkg_path}/Util.java"),
                fill(FABRIC_UTIL),
            );
            add(
                format!("src/main/java/{pkg_path}/Storage.java"),
                fill(STORAGE_JAVA),
            );
            add(
                format!("src/main/java/{pkg_path}/Loc.java"),
                fill(FABRIC_LOC),
            );
            add(
                format!("src/main/java/{pkg_path}/Scheduler.java"),
                fill(FABRIC_SCHEDULER),
            );
        }
    }
    files.push((
        "gradle/wrapper/gradle-wrapper.jar".into(),
        WRAPPER_JAR.to_vec(),
    ));
    add_readme(&mut files, proj);

    Output {
        files,
        problems,
        main_java,
    }
}

fn add_readme(files: &mut Vec<(String, Vec<u8>)>, proj: &Project) {
    let s = &proj.settings;
    let text = format!(
        "# {}\n\nGenerated by Minecraft Builder ({} for Minecraft {}).\n\nBuild the jar with:\n\n    gradlew build        (Windows)\n    ./gradlew build      (Linux/macOS)\n\nThe finished jar is in build/libs/.\n",
        s.name,
        s.platform.label(),
        s.versions.minecraft
    );
    files.push(("README.md".into(), text.into_bytes()));
}

fn valid_package(p: &str) -> bool {
    !p.is_empty()
        && p.split('.').all(|part| {
            let mut c = part.chars();
            matches!(c.next(), Some(f) if f.is_ascii_lowercase() || f == '_')
                && c.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        })
}

/// `api-version` for plugin.yml: "1.21.11" -> "1.21.11", "26.3" -> "26.3".
fn api_version(mc: &str) -> String {
    mc.trim().to_string()
}

fn paper_main(pkg: &str, main: &str, parts: &Parts) -> String {
    let mut o = format!(
        "package {pkg};\n\n// Generated by Minecraft Builder. Changes here are overwritten on the next build.\n\npublic final class {main} extends org.bukkit.plugin.java.JavaPlugin implements org.bukkit.event.Listener {{\n    public static {main} INSTANCE;\n\n    @Override\n    public void onEnable() {{\n        INSTANCE = this;\n        Storage.init(getDataFolder().toPath());\n        getServer().getPluginManager().registerEvents(this, this);\n"
    );
    for l in indent(&parts.init, 8) {
        o.push_str(&l);
        o.push('\n');
    }
    o.push_str("    }\n");
    for m in &parts.members {
        o.push('\n');
        for l in m.lines() {
            if l.is_empty() {
                o.push('\n');
            } else {
                o.push_str("    ");
                o.push_str(l);
                o.push('\n');
            }
        }
    }
    o.push_str("}\n");
    o
}

fn fabric_main(
    pkg: &str,
    main: &str,
    id: &str,
    parts: &Parts,
    imports: &BTreeSet<String>,
) -> String {
    let mut imps: BTreeSet<String> = imports.clone();
    for i in [
        "net.fabricmc.api.ModInitializer",
        "net.fabricmc.loader.api.FabricLoader",
        "net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents",
        "net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents",
        "net.minecraft.core.registries.BuiltInRegistries",
        "net.minecraft.network.chat.Component",
        "net.minecraft.server.level.ServerLevel",
        "net.minecraft.server.level.ServerPlayer",
        "org.slf4j.Logger",
        "org.slf4j.LoggerFactory",
    ] {
        imps.insert(i.into());
    }
    let mut o = format!("package {pkg};\n\n");
    for i in &imps {
        o.push_str(&format!("import {i};\n"));
    }
    o.push_str(&format!(
        "\n// Generated by Minecraft Builder. Changes here are overwritten on the next build.\n\npublic final class {main} implements ModInitializer {{\n    public static final String MOD_ID = {};\n    public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);\n",
        java_str(id)
    ));
    for m in parts
        .members
        .iter()
        .filter(|m| m.starts_with("private static int timer"))
    {
        o.push_str(&format!("    {m}\n"));
    }
    o.push_str(
        "\n    @Override\n    public void onInitialize() {\n        ServerLifecycleEvents.SERVER_STARTING.register(server -> {\n            Util.SERVER = server;\n            Storage.init(FabricLoader.getInstance().getConfigDir().resolve(MOD_ID));\n        });\n        ServerLifecycleEvents.SERVER_STOPPED.register(server -> Util.SERVER = null);\n        ServerTickEvents.END_SERVER_TICK.register(server -> Scheduler.tick());\n",
    );
    for l in indent(&parts.init, 8) {
        o.push_str(&l);
        o.push('\n');
    }
    o.push_str("    }\n");
    for m in parts
        .members
        .iter()
        .filter(|m| !m.starts_with("private static int timer"))
    {
        o.push('\n');
        for l in m.lines() {
            if l.is_empty() {
                o.push('\n');
            } else {
                o.push_str("    ");
                o.push_str(l);
                o.push('\n');
            }
        }
    }
    o.push_str("}\n");
    o
}

/// Writes all generated files into `dir`, removing stale generated sources first.
pub fn write_to(out: &Output, dir: &std::path::Path, package: &str) -> std::io::Result<()> {
    let src = dir.join("src/main/java").join(package.replace('.', "/"));
    if src.exists() {
        std::fs::remove_dir_all(&src)?;
    }
    for (rel, data) in &out.files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, data)?;
        #[cfg(unix)]
        if rel == "gradlew" {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples::{self, Starter};

    #[test]
    fn all_blocks_generate_without_errors() {
        for p in [Platform::Paper, Platform::Fabric] {
            let out = generate(&examples::all_blocks_project(p));
            let errors: Vec<_> = out.problems.iter().filter(|x| x.error).collect();
            assert!(errors.is_empty(), "{p:?}: {errors:?}");
            assert!(out.main_java.contains("class AlleBloecke"));
        }
    }

    #[test]
    fn starters_generate_without_problems() {
        for p in [Platform::Paper, Platform::Fabric] {
            for s in Starter::ALL {
                let out = generate(&examples::new_project("Test Plugin", p, s));
                assert!(out.problems.is_empty(), "{p:?} {s:?}: {:?}", out.problems);
            }
        }
    }

    #[test]
    fn java_strings_are_escaped() {
        assert_eq!(java_str("a\"b\\c\nä"), "\"a\\\"b\\\\c\\n\\u00e4\"");
        assert_eq!(java_num("1,5"), "1.5");
        assert_eq!(java_num("abc"), "0.0");
    }

    #[test]
    fn unconnected_value_from_other_flow_is_an_error() {
        let mut p = examples::new_project("X", Platform::Paper, Starter::Empty);
        let ws = &mut p.workspaces[0];
        ws.nodes.clear();
        ws.links.clear();
        let mk = |id, kind: &str| crate::model::Node {
            id,
            kind: kind.into(),
            pos: [0.0, 0.0],
            values: Default::default(),
        };
        ws.nodes.push(mk(1, "ev_join"));
        ws.nodes.push(mk(2, "ev_quit"));
        ws.nodes.push(mk(3, "send_message"));
        let link = |a: u64, ap: &str, b: u64, bp: &str| crate::model::Link {
            from: crate::model::PinRef {
                node: a,
                pin: ap.into(),
            },
            to: crate::model::PinRef {
                node: b,
                pin: bp.into(),
            },
        };
        ws.links.push(link(2, "next", 3, "in"));
        ws.links.push(link(1, "player", 3, "player"));
        let out = generate(&p);
        assert!(out.has_errors());
    }

    #[test]
    fn command_names_are_cleaned() {
        assert_eq!(clean_command("/Heal Me!"), "healme");
    }
}
