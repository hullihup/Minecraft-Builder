//! Test mode ("Try it out"): shows step by step which blocks run when an event
//! fires, and lights them up on the canvas. Nothing runs for real.

use crate::app::App;
use crate::blocks::{self, Kind, PinType};
use crate::i18n::{t, tl};
use crate::model::{NodeId, Project};
use crate::theme;
use egui::{Color32, RichText};
use std::collections::{HashMap, HashSet};

/// Seconds between two steps lighting up.
const STEP: f64 = 0.55;

pub struct Step {
    pub node: NodeId,
    pub depth: usize,
    /// Branch it runs in, e.g. "Gefunden".
    pub via: String,
    pub text: String,
}

pub struct Trace {
    pub ws: usize,
    pub start: NodeId,
    pub steps: Vec<Step>,
    pub t0: f64,
    /// Action blocks of the workspace that no event reaches.
    pub never: Vec<String>,
}

/// One line about a block: its name and the most telling value.
fn describe(project: &Project, ws: usize, id: NodeId) -> String {
    let w = &project.workspaces[ws];
    let Some(n) = w.node(id) else {
        return format!("#{id}");
    };
    let Some(def) = blocks::get(&n.kind) else {
        return n.kind.clone();
    };
    if let Some(c) = project.node_command(n) {
        return format!("{} /{}", tl(def.name), c.name);
    }
    let mut s = tl(def.name).to_string();
    for key in [
        "menu", "text", "key", "command", "item", "rid", "seconds", "title", "file",
    ] {
        let linked = w.link_into(id, key).is_some();
        let val = n
            .values
            .get(key)
            .cloned()
            .or_else(|| def.field(key).map(|f| f.default.to_string()))
            .or_else(|| def.input(key).map(|p| p.default.to_string()));
        if linked {
            s.push_str(&format!(" · {key}: {}", t("(verbunden)", "(connected)")));
            break;
        }
        if let Some(v) = val.filter(|v| !v.is_empty()) {
            let short: String = v.lines().next().unwrap_or("").chars().take(40).collect();
            s.push_str(&format!(" · „{short}“"));
            break;
        }
    }
    s
}

pub fn compute(project: &Project, ws: usize, start: NodeId) -> Vec<Step> {
    fn visit(
        project: &Project,
        ws: usize,
        id: NodeId,
        depth: usize,
        via: String,
        seen: &mut HashSet<NodeId>,
        out: &mut Vec<Step>,
    ) {
        if !seen.insert(id) || out.len() > 200 {
            return;
        }
        out.push(Step {
            node: id,
            depth,
            via,
            text: describe(project, ws, id),
        });
        let w = &project.workspaces[ws];
        let Some(n) = w.node(id) else { return };
        let Some(shape) = blocks::shape(n, project) else {
            return;
        };
        // Branches first (like the generated code), "next" last.
        let mut pins: Vec<&blocks::PinDef> = shape
            .outputs
            .iter()
            .filter(|o| o.ty == PinType::Exec && o.id != "next")
            .collect();
        pins.extend(
            shape
                .outputs
                .iter()
                .filter(|o| o.ty == PinType::Exec && o.id == "next"),
        );
        for pin in pins {
            let mut targets: Vec<&crate::model::Node> = w
                .links_from(id, &pin.id)
                .filter_map(|l| w.node(l.to.node))
                .collect();
            targets.sort_by(|a, b| a.pos[1].total_cmp(&b.pos[1]));
            let branch = pin.id != "next";
            for tgt in targets {
                visit(
                    project,
                    ws,
                    tgt.id,
                    depth + usize::from(branch),
                    if branch {
                        pin.label.text().to_string()
                    } else {
                        String::new()
                    },
                    seen,
                    out,
                );
            }
        }
    }
    let mut out = vec![];
    visit(
        project,
        ws,
        start,
        0,
        String::new(),
        &mut HashSet::new(),
        &mut out,
    );
    out
}

/// Starts the test for an event block (or the selected / first event).
pub fn start(app: &mut App, ws: usize, node: Option<NodeId>, now: f64) {
    let w = &app.project.workspaces[ws];
    let is_event = |id: NodeId| {
        w.node(id)
            .and_then(|n| blocks::get(&n.kind))
            .map(|d| d.kind == Kind::Event)
            .unwrap_or(false)
    };
    let start = node
        .filter(|id| is_event(*id))
        .or_else(|| app.selection.iter().copied().find(|id| is_event(*id)))
        .or_else(|| {
            let mut evs: Vec<&crate::model::Node> =
                w.nodes.iter().filter(|n| is_event(n.id)).collect();
            evs.sort_by(|a, b| a.pos[1].total_cmp(&b.pos[1]));
            evs.first().map(|n| n.id)
        });
    let Some(start) = start else {
        app.log(
            crate::build::Level::Warn,
            t(
                "Zum Ausprobieren brauchst du einen Event-Block (z. B. „Befehl“ oder „Spieler betritt Server“).",
                "To try it out you need an event block (e.g. “Command” or “Player joins”).",
            ),
        );
        return;
    };
    // Blocks that no event reaches.
    let mut reached = HashSet::new();
    for n in w.nodes.iter().filter(|n| is_event(n.id)) {
        for s in compute(&app.project, ws, n.id) {
            reached.insert(s.node);
        }
    }
    let opened = crate::ui::menu_editor::opened_by_actions(&app.project);
    let never: Vec<String> = w
        .nodes
        .iter()
        .filter(|n| {
            !(n.kind == "open_menu"
                && opened.contains(n.values.get("menu").map(|s| s.as_str()).unwrap_or("shop")))
        })
        .filter(|n| {
            blocks::get(&n.kind)
                .map(|d| matches!(d.kind, Kind::Action | Kind::Flow))
                .unwrap_or(false)
                && !reached.contains(&n.id)
        })
        .map(|n| describe(&app.project, ws, n.id))
        .collect();
    let steps = compute(&app.project, ws, start);
    app.trace = Some(Trace {
        ws,
        start,
        steps,
        t0: now,
        never,
    });
}

/// Blocks to light up now: node -> is the current step.
pub fn lit(app: &App, now: f64) -> HashMap<NodeId, bool> {
    let mut out = HashMap::new();
    if let Some(tr) = &app.trace {
        if tr.ws != app.active_ws {
            return out;
        }
        let shown = (((now - tr.t0) / STEP) as usize + 1).min(tr.steps.len());
        for (i, s) in tr.steps.iter().take(shown).enumerate() {
            out.insert(s.node, i + 1 == shown);
        }
    }
    out
}

pub fn window(app: &mut App, ctx: &egui::Context) {
    let Some(tr) = &app.trace else { return };
    let now = ctx.input(|i| i.time);
    let shown = (((now - tr.t0) / STEP) as usize + 1).min(tr.steps.len());
    let done = shown >= tr.steps.len();
    if !done {
        ctx.request_repaint_after(std::time::Duration::from_millis(60));
    }
    let mut close = false;
    let mut again = false;
    let mut jump: Option<NodeId> = None;
    let title = tr.steps.first().map(|s| s.text.clone()).unwrap_or_default();
    egui::Window::new(t("Ablauf ausprobieren", "Try out the flow"))
        .id(egui::Id::new("trace_window"))
        .default_pos(ctx.content_rect().right_top() + egui::vec2(-460.0, 80.0))
        .default_width(420.0)
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(
                RichText::new(format!("{} {title}", t("Wenn das passiert:", "When this happens:")))
                    .strong(),
            );
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                for (i, s) in tr.steps.iter().take(shown).enumerate().skip(1) {
                    ui.horizontal(|ui| {
                        ui.add_space(s.depth as f32 * 18.0);
                        let cur = i + 1 == shown && !done;
                        ui.label(
                            RichText::new(format!("{i}."))
                                .monospace()
                                .color(if cur { theme::GREEN } else { theme::TEXT_WEAK }),
                        );
                        if !s.via.is_empty() {
                            ui.label(RichText::new(format!("↳ {}:", s.via)).color(theme::YELLOW).size(12.0));
                        }
                        let r = ui.add(
                            egui::Label::new(RichText::new(&s.text).color(if cur { theme::GREEN } else { theme::TEXT }))
                                .sense(egui::Sense::click()),
                        );
                        if r.on_hover_text(t("Zum Block springen", "Jump to the block")).clicked() {
                            jump = Some(s.node);
                        }
                    });
                }
            });
            if tr.steps.len() <= 1 {
                ui.label(
                    RichText::new(t(
                        "Dieses Event startet noch nichts: Zieh vom grünen Pfeil ▷ des Events zu einem Aktions-Block.",
                        "This event does not start anything yet: drag from the event's green arrow ▷ to an action block.",
                    ))
                    .color(theme::YELLOW),
                );
            } else if done {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "✔ {} {}",
                        tr.steps.len() - 1,
                        t("Blöcke laufen in dieser Reihenfolge.", "blocks run in this order.")
                    ))
                    .color(theme::GREEN),
                );
            }
            if done && !tr.never.is_empty() {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(t(
                        "Diese Blöcke laufen bei keinem Event (grüner Pfeil fehlt):",
                        "These blocks run for no event (green arrow missing):",
                    ))
                    .color(theme::YELLOW),
                );
                for n in &tr.never {
                    ui.label(RichText::new(format!("• {n}")).size(12.0).color(Color32::from_rgb(0xFF, 0xD8, 0x80)));
                }
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(t(
                    "Nur eine Vorschau – im Spiel passiert dabei nichts. Werte wie Spieler oder Position kommen erst im Spiel dazu.",
                    "Only a preview – nothing happens in the game. Values like player or location only exist in the game.",
                ))
                .size(11.0)
                .color(theme::TEXT_WEAK),
            );
            ui.horizontal(|ui| {
                if ui.button(t("Nochmal", "Again")).clicked() {
                    again = true;
                }
                if ui.button(t("Schließen", "Close")).clicked() {
                    close = true;
                }
            });
        });
    if again {
        let (ws, st) = (tr.ws, tr.start);
        start(app, ws, Some(st), now);
    }
    if let Some(id) = jump {
        app.focus_node = Some((app.active_ws, id));
        app.selection.clear();
        app.selection.insert(id);
    }
    if close {
        app.trace = None;
    }
}
