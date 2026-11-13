//! The node editor canvas: pan/zoom, nodes with inline editors, wires, notes.

use crate::app::{App, Clip, Palette};
use crate::blocks::{self, BlockDef, Kind, PinType};
use crate::i18n::{t, tl};
use crate::model::{Link, NodeId, Note, PinRef, Workspace};
use crate::theme;
use crate::ui::widgets;
use egui::emath::TSTransform;
use egui::{
    Color32, CornerRadius, FontId, Id, LayerId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2, pos2, vec2,
};
use std::collections::{HashMap, HashSet};

pub const GRID: f32 = 16.0;
const HEADER_H: f32 = 30.0;
const ROW_H: f32 = 24.0;
const PAD_X: f32 = 12.0;

pub fn snap_pos(p: Pos2) -> Pos2 {
    pos2((p.x / GRID).round() * GRID, (p.y / GRID).round() * GRID)
}

pub fn node_width(def: &BlockDef) -> f32 {
    match def.kind {
        Kind::Value => 220.0,
        _ => 260.0,
    }
}

#[derive(Clone)]
pub struct LinkDrag {
    pub node: NodeId,
    pub pin: String,
    pub is_output: bool,
    pub ty: PinType,
}

pub struct CanvasState {
    /// Node sizes from the last frame (scene units).
    pub sizes: HashMap<NodeId, Vec2>,
    /// Pin positions of this frame: (node, pin, is_output) -> (scene pos, type).
    pub pins: HashMap<(NodeId, String, bool), (Pos2, PinType)>,
    pub link_drag: Option<LinkDrag>,
    pub box_start: Option<Pos2>,
    /// Note being moved, with the nodes inside it.
    pub note_drag: Option<(NodeId, Vec<NodeId>)>,
    pub focus_top_search: bool,
    pub rect: Rect,
    pub fit_request: bool,
    /// The current link was started by a click (finish with a second click).
    pub click_link: bool,
    /// Selected wire (click on it; Del removes it).
    pub selected_link: Option<Link>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            sizes: HashMap::new(),
            pins: HashMap::new(),
            link_drag: None,
            box_start: None,
            note_drag: None,
            focus_top_search: false,
            rect: Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)),
            fit_request: false,
            click_link: false,
            selected_link: None,
        }
    }
}

enum NodeEvent {
    Select {
        id: NodeId,
        add: bool,
    },
    Drag {
        id: NodeId,
        delta: Vec2,
    },
    DragStop,
    PinDrag(LinkDrag),
    /// A pin was clicked: start a click-connection or finish one.
    PinClick(LinkDrag),
    PinClear {
        node: NodeId,
        pin: String,
        is_output: bool,
    },
    Delete(NodeId),
    Duplicate(NodeId),
    Disconnect(NodeId),
}

fn bezier(a: Pos2, b: Pos2, color: Color32, width: f32) -> Shape {
    let dx = ((b.x - a.x).abs() * 0.5).clamp(40.0, 220.0);
    Shape::CubicBezier(egui::epaint::CubicBezierShape::from_points_stroke(
        [a, a + vec2(dx, 0.0), b - vec2(dx, 0.0), b],
        false,
        Color32::TRANSPARENT,
        Stroke::new(width, color),
    ))
}

fn bezier_point(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    let dx = ((b.x - a.x).abs() * 0.5).clamp(40.0, 220.0);
    let (p0, p1, p2, p3) = (a, a + vec2(dx, 0.0), b - vec2(dx, 0.0), b);
    let u = 1.0 - t;
    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    pos2(
        w[0] * p0.x + w[1] * p1.x + w[2] * p2.x + w[3] * p3.x,
        w[0] * p0.y + w[1] * p1.y + w[2] * p2.y + w[3] * p3.y,
    )
}

/// Approximate distance from `p` to the wire curve.
fn bezier_distance(a: Pos2, b: Pos2, p: Pos2) -> f32 {
    let bounds = Rect::from_two_pos(a, b).expand2(vec2(230.0, 12.0));
    if !bounds.contains(p) {
        return f32::MAX;
    }
    let mut best = f32::MAX;
    let mut prev = a;
    for i in 1..=32 {
        let q = bezier_point(a, b, i as f32 / 32.0);
        let seg = q - prev;
        let len2 = seg.length_sq().max(1e-4);
        let tt = ((p - prev).dot(seg) / len2).clamp(0.0, 1.0);
        best = best.min((prev + seg * tt).distance(p));
        prev = q;
    }
    best
}

fn exec_triangle(p: Pos2, filled: bool, color: Color32) -> Shape {
    let pts = vec![
        p + vec2(-5.0, -6.0),
        p + vec2(6.0, 0.0),
        p + vec2(-5.0, 6.0),
    ];
    if filled {
        Shape::convex_polygon(pts, color, Stroke::NONE)
    } else {
        Shape::closed_line(pts, Stroke::new(1.5, color))
    }
}

fn pin_shape(painter: &egui::Painter, p: Pos2, ty: PinType, connected: bool, hovered: bool) {
    let c = ty.color();
    let r = if hovered { 6.5 } else { 5.0 };
    if ty == PinType::Exec {
        painter.add(exec_triangle(p, connected || hovered, c));
    } else if connected || hovered {
        painter.circle_filled(p, r, c);
    } else {
        painter.circle(p, r - 0.5, theme::NODE_BG, Stroke::new(1.6, c));
    }
}

/// Can `a` and `b` be connected? Returns (output, input) in the right order.
fn link_pair(
    ws: &Workspace,
    a: &LinkDrag,
    b: &(NodeId, String, bool, PinType),
) -> Option<(PinRef, PinRef)> {
    if a.is_output == b.2 || a.node == b.0 {
        return None;
    }
    let (out_ty, in_ty) = if a.is_output {
        (a.ty, b.3)
    } else {
        (b.3, a.ty)
    };
    let ok = if out_ty == PinType::Exec || in_ty == PinType::Exec {
        out_ty == in_ty
    } else {
        blocks::compatible(out_ty, in_ty)
    };
    if !ok {
        return None;
    }
    let (o, i) = if a.is_output {
        (
            PinRef {
                node: a.node,
                pin: a.pin.clone(),
            },
            PinRef {
                node: b.0,
                pin: b.1.clone(),
            },
        )
    } else {
        (
            PinRef {
                node: b.0,
                pin: b.1.clone(),
            },
            PinRef {
                node: a.node,
                pin: a.pin.clone(),
            },
        )
    };
    let _ = ws;
    Some((o, i))
}

/// Adds a link, replacing links that would conflict (one wire per data input / exec output).
pub fn connect(ws: &mut Workspace, from: PinRef, to: PinRef, exec: bool) {
    if exec {
        ws.links.retain(|l| l.from != from);
    } else {
        ws.links.retain(|l| l.to != to);
    }
    ws.links.push(Link { from, to });
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let rect = ui.available_rect_before_wrap();
    app.canvas.rect = rect;
    let ctx = ui.ctx().clone();
    let bg_resp = ui.interact(
        rect,
        ui.scope_id().with("canvas_bg"),
        Sense::click_and_drag(),
    );

    // ------------------------------------------------------------ view input
    let ws_idx = app.active_ws.min(app.project.workspaces.len() - 1);
    if app.canvas.fit_request {
        app.canvas.fit_request = false;
        fit_view(app, rect);
    }
    if let Some((w, n)) = app.focus_node.take() {
        app.switch_ws(w);
        let ws = app.ws();
        if let Some(node) = ws.node(n) {
            let size = app
                .canvas
                .sizes
                .get(&n)
                .copied()
                .unwrap_or(vec2(240.0, 120.0));
            let c = pos2(node.pos[0], node.pos[1]) + size / 2.0;
            let view = app.view();
            view.pan = rect.size() / 2.0 - c.to_vec2() * view.zoom;
            app.selection.clear();
            app.selection.insert(n);
        }
    }

    let pointer = ctx.pointer_hover_pos();
    let pointer_in = pointer.map(|p| rect.contains(p)).unwrap_or(false);
    let over_popup = pointer
        .and_then(|p| ctx.layer_id_at(p))
        .map(|l| l.order != egui::Order::Background && l.order != egui::Order::Middle)
        .unwrap_or(false);
    let scene_layer = LayerId::new(ui.layer_id().order, ui.scope_id().with("scene"));

    let left_selects = app.settings.left_drag_selects;
    {
        let view = app.views.entry(ws_idx).or_default();
        if pointer_in && !over_popup && app.palette.is_none() {
            let (scroll, zoom_delta) = ctx.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
            let mut factor = zoom_delta;
            if scroll.y != 0.0 {
                factor *= (scroll.y * 0.0018).exp();
            }
            if factor != 1.0 {
                let p = pointer.unwrap();
                let origin = rect.min.to_vec2() + view.pan;
                let scene = (p.to_vec2() - origin) / view.zoom;
                let nz = (view.zoom * factor).clamp(0.25, 1.6);
                view.pan = p.to_vec2() - rect.min.to_vec2() - scene * nz;
                view.zoom = nz;
            }
            // Middle mouse pans everywhere.
            if ctx.input(|i| i.pointer.middle_down()) {
                view.pan += ctx.input(|i| i.pointer.delta());
            }
        }
        let shift = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
        if bg_resp.drag_started_by(egui::PointerButton::Primary) && (shift != left_selects) {
            let origin = rect.min.to_vec2() + view.pan;
            if let Some(p) = bg_resp.interact_pointer_pos() {
                app.canvas.box_start = Some(((p.to_vec2() - origin) / view.zoom).to_pos2());
            }
        } else if (bg_resp.dragged_by(egui::PointerButton::Primary)
            && app.canvas.box_start.is_none())
            || bg_resp.dragged_by(egui::PointerButton::Secondary)
        {
            view.pan += bg_resp.drag_delta();
        }
    }
    let view = *app.views.entry(ws_idx).or_default();
    let to_screen = TSTransform::new(rect.min.to_vec2() + view.pan, view.zoom);
    let to_scene = to_screen.inverse();
    let pointer_scene = pointer.map(|p| to_scene * p);

    // ------------------------------------------------------------ background
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::BG_CANVAS);
    let step = 24.0 * view.zoom;
    if step > 7.0 {
        let origin = rect.min + view.pan;
        let start_x = rect.left() + (origin.x - rect.left()).rem_euclid(step);
        let start_y = rect.top() + (origin.y - rect.top()).rem_euclid(step);
        let r = (1.1 * view.zoom).clamp(0.6, 1.4);
        let mut y = start_y;
        while y < rect.bottom() {
            let mut x = start_x;
            while x < rect.right() {
                painter.circle_filled(pos2(x, y), r, theme::GRID_DOT);
                x += step;
            }
            y += step;
        }
    }

    // --------------------------------------------------------------- scene
    ctx.set_sublayer(ui.layer_id(), scene_layer);
    ctx.set_transform_layer(scene_layer, to_screen);
    let mut sui = ui.new_child(
        UiBuilder::new()
            .layer_id(scene_layer)
            .max_rect(Rect::from_min_size(Pos2::ZERO, vec2(1.0, 1.0)))
            .id_salt("scene_ui"),
    );
    sui.set_clip_rect(to_scene * rect);
    let wires_slot = sui.painter().add(Shape::Noop);

    // Problems per node.
    let mut node_problems: HashMap<NodeId, (bool, Vec<String>)> = HashMap::new();
    for p in app.problems.iter().filter(|p| p.ws == ws_idx) {
        if let Some(n) = p.node {
            let e = node_problems.entry(n).or_insert((false, vec![]));
            e.0 |= p.error;
            e.1.push(p.msg.clone());
        }
    }

    // ---------------------------------------------------------------- notes
    let mut note_events: Vec<(NodeId, Vec2, bool)> = vec![]; // (note, delta, resize)
    let mut note_select: Option<NodeId> = None;
    let mut note_drag_start: Option<NodeId> = None;
    let mut note_drag_end = false;
    {
        let ws = &app.project.workspaces[ws_idx];
        for note in &ws.notes {
            let r = Rect::from_min_size(
                pos2(note.pos[0], note.pos[1]),
                vec2(note.size[0], note.size[1]),
            );
            let col = theme::NOTE_COLORS[note.color % theme::NOTE_COLORS.len()];
            let selected = app.selected_note == Some(note.id);
            let p = sui.painter();
            p.rect(
                r,
                CornerRadius::same(8),
                col.gamma_multiply(0.07),
                Stroke::new(
                    if selected { 2.0 } else { 1.2 },
                    col.gamma_multiply(if selected { 0.9 } else { 0.45 }),
                ),
                StrokeKind::Inside,
            );
            let header = Rect::from_min_size(r.min, vec2(r.width(), 44.0));
            p.rect_filled(
                header,
                CornerRadius {
                    nw: 8,
                    ne: 8,
                    sw: 0,
                    se: 0,
                },
                col.gamma_multiply(0.10),
            );
            p.text(
                header.min + vec2(12.0, 8.0),
                egui::Align2::LEFT_TOP,
                if note.title.is_empty() {
                    t("Notiz", "Note")
                } else {
                    &note.title
                },
                FontId::proportional(14.0),
                theme::TEXT,
            );
            if !note.text.is_empty() {
                let g = p.layout(
                    note.text.clone(),
                    FontId::proportional(11.5),
                    theme::TEXT_WEAK,
                    r.width() - 24.0,
                );
                p.galley(header.min + vec2(12.0, 26.0), g, theme::TEXT_WEAK);
            }
            let hr = sui.interact(
                header,
                Id::unique(("note", note.id)),
                Sense::click_and_drag(),
            );
            if hr.clicked() || hr.drag_started() {
                note_select = Some(note.id);
            }
            if hr.drag_started() {
                note_drag_start = Some(note.id);
            }
            if hr.dragged() {
                note_events.push((note.id, hr.drag_delta(), false));
            }
            if hr.drag_stopped() {
                note_drag_end = true;
            }
            let handle = Rect::from_min_size(r.max - vec2(16.0, 16.0), vec2(16.0, 16.0));
            let rr = sui.interact(handle, Id::unique(("note_rs", note.id)), Sense::drag());
            let hc = if rr.hovered() || rr.dragged() {
                col
            } else {
                col.gamma_multiply(0.5)
            };
            sui.painter().line_segment(
                [
                    handle.left_bottom() + vec2(4.0, -3.0),
                    handle.right_top() + vec2(-3.0, 4.0),
                ],
                Stroke::new(1.5, hc),
            );
            sui.painter().line_segment(
                [
                    handle.center_bottom() + vec2(2.0, -3.0),
                    handle.right_center() + vec2(-3.0, 2.0),
                ],
                Stroke::new(1.5, hc),
            );
            if rr.dragged() {
                note_events.push((note.id, rr.drag_delta(), true));
            }
            if rr.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            }
        }
    }
    if let Some(id) = note_drag_start {
        // Capture nodes inside the note.
        let ws = &app.project.workspaces[ws_idx];
        if let Some(note) = ws.notes.iter().find(|n| n.id == id) {
            let r = Rect::from_min_size(
                pos2(note.pos[0], note.pos[1]),
                vec2(note.size[0], note.size[1]),
            );
            let inside: Vec<NodeId> = ws
                .nodes
                .iter()
                .filter(|n| {
                    let s = app
                        .canvas
                        .sizes
                        .get(&n.id)
                        .copied()
                        .unwrap_or(vec2(200.0, 80.0));
                    r.contains_rect(Rect::from_min_size(pos2(n.pos[0], n.pos[1]), s))
                })
                .map(|n| n.id)
                .collect();
            app.canvas.note_drag = Some((id, inside));
        }
    }
    if let Some(id) = note_select {
        app.selected_note = Some(id);
        app.selection.clear();
    }
    for (id, d, resize) in note_events {
        let inside = app
            .canvas
            .note_drag
            .as_ref()
            .filter(|(n, _)| *n == id)
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let ws = &mut app.project.workspaces[ws_idx];
        if let Some(n) = ws.notes.iter_mut().find(|n| n.id == id) {
            if resize {
                n.size[0] = (n.size[0] + d.x).max(200.0);
                n.size[1] = (n.size[1] + d.y).max(120.0);
            } else {
                n.pos[0] += d.x;
                n.pos[1] += d.y;
                for nid in inside {
                    if let Some(node) = ws.node_mut(nid) {
                        node.pos[0] += d.x;
                        node.pos[1] += d.y;
                    }
                }
            }
        }
    }
    if note_drag_end {
        app.canvas.note_drag = None;
    }

    // ---------------------------------------------------------------- nodes
    app.canvas.pins.clear();
    let platform = app.project.settings.platform;
    let mut events: Vec<NodeEvent> = vec![];
    let connected: HashSet<(NodeId, String, bool)> = {
        let ws = &app.project.workspaces[ws_idx];
        ws.links
            .iter()
            .flat_map(|l| {
                [
                    (l.from.node, l.from.pin.clone(), true),
                    (l.to.node, l.to.pin.clone(), false),
                ]
            })
            .collect()
    };
    let order: Vec<usize> = {
        let ws = &app.project.workspaces[ws_idx];
        let mut v: Vec<usize> = (0..ws.nodes.len())
            .filter(|i| !app.selection.contains(&ws.nodes[*i].id))
            .collect();
        v.extend((0..ws.nodes.len()).filter(|i| app.selection.contains(&ws.nodes[*i].id)));
        v
    };
    let hovered_pin_candidate = app.canvas.link_drag.clone();
    // Command blocks: title and option outputs come from the command definition.
    let cmd_info: HashMap<NodeId, (String, Vec<blocks::PinDef>)> = app.project.workspaces[ws_idx]
        .nodes
        .iter()
        .filter(|n| n.kind == "ev_command")
        .map(|n| {
            let title = match app.project.node_command(n) {
                Some(c) => format!("{} · /{}", t("Befehl", "Command"), c.name),
                None => format!("{} · ?", t("Befehl", "Command")),
            };
            let outs = blocks::shape(n, &app.project)
                .map(|s| s.outputs.into_owned())
                .unwrap_or_default();
            (n.id, (title, outs))
        })
        .collect();
    let mut new_sizes = HashMap::new();
    for idx in order {
        let node = &mut app.project.workspaces[ws_idx].nodes[idx];
        let Some(def) = blocks::get(&node.kind) else {
            // Unknown block: draw a red box so it can be deleted.
            let r = Rect::from_min_size(pos2(node.pos[0], node.pos[1]), vec2(200.0, 50.0));
            sui.painter()
                .rect_filled(r, 6.0, Color32::from_rgb(0x55, 0x22, 0x22));
            sui.painter().text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                format!("? {}", node.kind),
                FontId::proportional(13.0),
                theme::TEXT,
            );
            let resp = sui.interact(r, Id::unique(("node", node.id)), Sense::click_and_drag());
            if resp.clicked() {
                events.push(NodeEvent::Select {
                    id: node.id,
                    add: false,
                });
            }
            if resp.dragged() {
                events.push(NodeEvent::Drag {
                    id: node.id,
                    delta: resp.drag_delta(),
                });
            }
            continue;
        };
        let selected = app.selection.contains(&node.id);
        let width = node_width(def);
        let pos = pos2(node.pos[0], node.pos[1]);
        let prev_size = app
            .canvas
            .sizes
            .get(&node.id)
            .copied()
            .unwrap_or(vec2(width, 80.0));
        let node_rect_prev = Rect::from_min_size(pos, prev_size);

        // Body interaction is registered first so widgets inside win.
        let body = sui.interact(
            node_rect_prev,
            Id::unique(("node", node.id)),
            Sense::click_and_drag(),
        );
        if body.clicked() || body.drag_started() {
            let add = ctx.input(|i| i.modifiers.command || i.modifiers.shift);
            if !(body.drag_started() && selected) {
                events.push(NodeEvent::Select { id: node.id, add });
            }
        }
        if body.dragged_by(egui::PointerButton::Primary) {
            events.push(NodeEvent::Drag {
                id: node.id,
                delta: body.drag_delta(),
            });
        }
        if body.drag_stopped() {
            events.push(NodeEvent::DragStop);
        }
        let nid = node.id;
        body.context_menu(|ui| {
            if ui.button(t("Duplizieren", "Duplicate")).clicked() {
                events.push(NodeEvent::Duplicate(nid));
                ui.close();
            }
            if ui
                .button(t("Alle Verbindungen trennen", "Disconnect all"))
                .clicked()
            {
                events.push(NodeEvent::Disconnect(nid));
                ui.close();
            }
            ui.separator();
            if ui
                .button(egui::RichText::new(t("Löschen", "Delete")).color(theme::RED))
                .clicked()
            {
                events.push(NodeEvent::Delete(nid));
                ui.close();
            }
        });

        let bg_slot = sui.painter().add(Shape::Noop);
        let mut nui = sui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(pos, vec2(width, 4000.0)))
                .layout(egui::Layout::top_down(egui::Align::Min))
                .id_salt(("node_ui", node.id)),
        );
        nui.spacing_mut().item_spacing = vec2(6.0, 4.0);
        nui.spacing_mut().interact_size.y = 20.0;
        nui.style_mut().spacing.button_padding = vec2(6.0, 2.0);
        {
            let v = &mut nui.style_mut().visuals;
            let field = Color32::from_rgb(0x1C, 0x1D, 0x21);
            v.extreme_bg_color = field;
            v.text_edit_bg_color = Some(field);
            for w in [&mut v.widgets.inactive, &mut v.widgets.hovered] {
                w.corner_radius = CornerRadius::same(4);
            }
            v.widgets.inactive.bg_fill = field;
            v.widgets.inactive.weak_bg_fill = field;
            v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x34, 0x37, 0x3D));
            v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x55, 0x59, 0x61));
        }

        // Header
        let (hrect, _) = nui.allocate_exact_size(vec2(width, HEADER_H), Sense::hover());
        let cat = def.cat.color();
        let mut pin_list: Vec<(String, bool, PinType, Pos2)> = vec![];
        {
            let p = nui.painter();
            let title_text = cmd_info
                .get(&nid)
                .map(|c| c.0.clone())
                .unwrap_or_else(|| tl(def.name).to_string());
            crate::ui::icons::paint(
                p,
                Rect::from_center_size(
                    hrect.left_center() + vec2(PAD_X + 7.0, 0.0),
                    vec2(15.0, 15.0),
                ),
                def.cat.icon_kind(),
                cat,
            );
            let title_galley =
                p.layout_no_wrap(title_text.clone(), FontId::proportional(13.5), theme::TEXT);
            let max_title = width - 2.0 * PAD_X - 60.0;
            let title_pos = hrect.left_center() + vec2(PAD_X + 22.0, -title_galley.size().y / 2.0);
            if title_galley.size().x > max_title {
                let g = p.layout(
                    title_text.clone(),
                    FontId::proportional(12.0),
                    theme::TEXT,
                    max_title,
                );
                p.galley(
                    hrect.left_center() + vec2(PAD_X + 22.0, -g.size().y / 2.0),
                    g,
                    theme::TEXT,
                );
            } else {
                p.galley(title_pos, title_galley, theme::TEXT);
            }
            p.text(
                hrect.right_center() - vec2(PAD_X + 24.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                format!("#{}", node.id),
                FontId::proportional(10.5),
                theme::TEXT_WEAK,
            );
        }
        // "⋯" menu button (left click).
        {
            let mr = Rect::from_center_size(
                hrect.right_center() - vec2(PAD_X + 6.0, 0.0),
                vec2(18.0, 18.0),
            );
            let mresp = nui.interact(mr, Id::unique(("node_menu", nid)), Sense::click());
            if mresp.hovered() {
                nui.painter()
                    .rect_filled(mr, 4.0, Color32::from_white_alpha(18));
            }
            crate::ui::icons::paint(
                nui.painter(),
                mr.shrink(2.0),
                crate::ui::icons::Icon::Dots,
                if mresp.hovered() {
                    theme::TEXT
                } else {
                    theme::TEXT_WEAK
                },
            );
            let mresp = mresp.on_hover_text(t("Block-Menü", "Block menu"));
            egui::Popup::menu(&mresp).show(|ui| {
                if ui.button(t("Duplizieren", "Duplicate")).clicked() {
                    events.push(NodeEvent::Duplicate(nid));
                    ui.close();
                }
                if ui
                    .button(t("Alle Verbindungen trennen", "Disconnect all"))
                    .clicked()
                {
                    events.push(NodeEvent::Disconnect(nid));
                    ui.close();
                }
                ui.separator();
                if ui
                    .button(egui::RichText::new(t("Löschen", "Delete")).color(theme::RED))
                    .clicked()
                {
                    events.push(NodeEvent::Delete(nid));
                    ui.close();
                }
            });
        }
        if def.has_exec_in() {
            pin_list.push((
                "in".into(),
                false,
                PinType::Exec,
                pos2(hrect.left(), hrect.center().y),
            ));
        }
        let header_next = matches!(def.kind, Kind::Event | Kind::Action);
        if header_next {
            pin_list.push((
                "next".into(),
                true,
                PinType::Exec,
                pos2(hrect.right(), hrect.center().y),
            ));
        }
        nui.add_space(6.0);

        let inner_w = width - 2.0 * PAD_X;
        let label_w = 86.0;
        let edit_w = inner_w - label_w - 8.0;
        let mut changed = false;

        // Fields
        for f in def.fields.iter().filter(|f| !f.advanced) {
            let multiline = matches!(f.kind, blocks::FieldKind::Multiline);
            if multiline {
                row(&mut nui, |ui| {
                    ui.label(
                        egui::RichText::new(tl(f.label))
                            .size(11.5)
                            .color(theme::TEXT_WEAK),
                    );
                });
                row(&mut nui, |ui| {
                    changed |=
                        widgets::field_editor(ui, f, &mut node.values, inner_w, ("f", nid, f.id));
                });
            } else {
                row(&mut nui, |ui| {
                    row_label(
                        ui,
                        label_w,
                        egui::RichText::new(tl(f.label))
                            .size(11.5)
                            .color(theme::TEXT_WEAK),
                    );
                    changed |=
                        widgets::field_editor(ui, f, &mut node.values, edit_w, ("f", nid, f.id));
                });
            }
        }

        // Data inputs (optional ones are folded away until opened or connected)
        let adv_id = Id::unique(("adv_open", nid));
        let adv_open = ctx.data(|d| d.get_temp::<bool>(adv_id)).unwrap_or(false);
        let hidden: Vec<&blocks::PinDef> = def
            .inputs
            .iter()
            .filter(|p| p.advanced && !connected.contains(&(nid, p.id.to_string(), false)))
            .collect();
        for pin in def.inputs.iter().filter(|p| p.ty != PinType::Exec) {
            if !adv_open && hidden.iter().any(|h| h.id == pin.id) {
                continue;
            }
            if node.kind.starts_with("data_")
                && pin.id == "player"
                && node.values.get("scope").map(|s| s.as_str()) == Some("global")
            {
                continue;
            }
            let is_conn = connected.contains(&(nid, pin.id.to_string(), false));
            let r = row(&mut nui, |ui| {
                row_label(
                    ui,
                    label_w,
                    egui::RichText::new(pin.label.text()).size(11.5),
                );
                if !is_conn && pin.ty.has_inline() {
                    changed |= widgets::pin_editor(
                        ui,
                        pin.ty,
                        &pin.id,
                        pin.default,
                        &mut node.values,
                        edit_w,
                        ("p", nid, pin.id.clone()),
                    );
                } else if !is_conn {
                    ui.label(
                        egui::RichText::new(t("automatisch", "automatic"))
                            .size(11.0)
                            .italics()
                            .color(theme::TEXT_WEAK),
                    )
                    .on_hover_text(t(
                        "Nicht verbunden: der Spieler aus dem Event wird benutzt.",
                        "Not connected: the player from the event is used.",
                    ));
                }
            });
            pin_list.push((
                pin.id.to_string(),
                false,
                pin.ty,
                pos2(hrect.left(), r.center().y),
            ));
        }

        if !hidden.is_empty() {
            let label = if adv_open {
                format!("▾ {}", t("weniger Optionen", "fewer options"))
            } else {
                if hidden.len() == 1 {
                    format!("▸ 1 {}", t("weitere Option", "more option"))
                } else {
                    format!(
                        "▸ {} {}",
                        hidden.len(),
                        t("weitere Optionen", "more options")
                    )
                }
            };
            let r = row(&mut nui, |ui| {
                ui.label(
                    egui::RichText::new(&label)
                        .size(11.0)
                        .color(theme::TEXT_WEAK),
                );
            });
            let resp = nui.interact(r, Id::unique(("adv_btn", nid)), Sense::click());
            if resp.hovered() {
                nui.painter().rect_filled(
                    r.shrink2(vec2(6.0, 0.0)),
                    4.0,
                    Color32::from_white_alpha(8),
                );
            }
            if resp.clicked() {
                ctx.data_mut(|d| d.insert_temp(adv_id, !adv_open));
            }
        }

        // Outputs (exec rows only for flow blocks)
        let dyn_outs = cmd_info.get(&nid).map(|c| c.1.clone());
        let outputs: &[blocks::PinDef] = dyn_outs.as_deref().unwrap_or(&def.outputs);
        for pin in outputs.iter() {
            if pin.ty == PinType::Exec && header_next {
                continue;
            }
            let r = row(&mut nui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut txt = egui::RichText::new(pin.label.text()).size(11.5);
                    if pin.ty == PinType::Exec {
                        txt = txt.color(theme::GREEN);
                    }
                    ui.label(txt);
                });
            });
            pin_list.push((
                pin.id.to_string(),
                true,
                pin.ty,
                pos2(hrect.right(), r.center().y),
            ));
        }
        nui.add_space(8.0);
        let node_rect = Rect::from_min_size(pos, vec2(width, nui.min_rect().height()));
        new_sizes.insert(nid, node_rect.size());

        // Background + border
        let problem = node_problems.get(&nid);
        let border = if selected {
            Stroke::new(2.0, theme::ACCENT)
        } else if let Some((true, _)) = problem {
            Stroke::new(1.5, theme::RED)
        } else if let Some((false, _)) = problem {
            Stroke::new(1.2, theme::YELLOW.gamma_multiply(0.7))
        } else {
            Stroke::new(1.0, theme::NODE_BORDER)
        };
        let unsupported = !def.supports(platform);
        let shadow = egui::epaint::Shadow {
            offset: [0, 4],
            blur: 14,
            spread: 0,
            color: Color32::from_black_alpha(90),
        };
        let mut shapes = vec![shadow.as_shape(node_rect, CornerRadius::same(8)).into()];
        shapes.push(Shape::rect_filled(
            node_rect,
            CornerRadius::same(8),
            theme::NODE_BG,
        ));
        shapes.push(Shape::rect_filled(
            hrect,
            CornerRadius {
                nw: 8,
                ne: 8,
                sw: 0,
                se: 0,
            },
            blend(theme::NODE_HEADER, cat, 0.10),
        ));
        shapes.push(Shape::line_segment(
            [hrect.left_bottom(), hrect.right_bottom()],
            Stroke::new(1.0, theme::NODE_BORDER),
        ));
        if unsupported {
            shapes.push(Shape::rect_filled(
                node_rect,
                CornerRadius::same(8),
                Color32::from_black_alpha(110),
            ));
        }
        shapes.push(Shape::rect_stroke(
            node_rect,
            CornerRadius::same(8),
            border,
            StrokeKind::Inside,
        ));
        sui.painter().set(bg_slot, Shape::Vec(shapes));

        // Problem badge
        if let Some((err, msgs)) = problem {
            let c = pos2(node_rect.right() - 2.0, node_rect.top() + 2.0);
            let col = if *err { theme::RED } else { theme::YELLOW };
            sui.painter().circle_filled(c, 8.0, col);
            sui.painter().text(
                c,
                egui::Align2::CENTER_CENTER,
                "!",
                FontId::proportional(12.0),
                Color32::BLACK,
            );
            let br = sui.interact(
                Rect::from_center_size(c, vec2(16.0, 16.0)),
                Id::unique(("badge", nid)),
                Sense::hover(),
            );
            br.on_hover_text(msgs.join("\n"));
        }
        if unsupported {
            sui.painter().text(
                node_rect.center_bottom() + vec2(0.0, 12.0),
                egui::Align2::CENTER_CENTER,
                match platform {
                    crate::model::Platform::Paper => t("Gibt es nur für Fabric", "Fabric only"),
                    crate::model::Platform::Fabric => t("Gibt es nur für Paper", "Paper only"),
                },
                FontId::proportional(11.0),
                theme::YELLOW,
            );
        }

        // Pins
        for (pin, is_out, ty, p) in pin_list {
            let pr = sui.interact(
                Rect::from_center_size(p, vec2(18.0, 18.0)),
                Id::unique(("pin", nid, &pin, is_out)),
                Sense::click_and_drag(),
            );
            let is_conn = connected.contains(&(nid, pin.clone(), is_out));
            let mut hov = pr.hovered() || pr.dragged();
            if let Some(d) = &hovered_pin_candidate {
                if let Some(ps) = pointer_scene {
                    if ps.distance(p) < 14.0 {
                        let cand = (nid, pin.clone(), is_out, ty);
                        hov = link_pair(&app.project.workspaces[ws_idx], d, &cand).is_some();
                    }
                }
            }
            pin_shape(sui.painter(), p, ty, is_conn, hov);
            if pr.drag_started() {
                events.push(NodeEvent::PinDrag(LinkDrag {
                    node: nid,
                    pin: pin.clone(),
                    is_output: is_out,
                    ty,
                }));
            }
            if pr.clicked() && !ctx.input(|i| i.modifiers.alt) {
                events.push(NodeEvent::PinClick(LinkDrag {
                    node: nid,
                    pin: pin.clone(),
                    is_output: is_out,
                    ty,
                }));
            }
            if pr.secondary_clicked() || (pr.clicked() && ctx.input(|i| i.modifiers.alt)) {
                events.push(NodeEvent::PinClear {
                    node: nid,
                    pin: pin.clone(),
                    is_output: is_out,
                });
            }
            if pr.hovered() && app.canvas.link_drag.is_none() {
                pr.on_hover_text(format!(
                    "{} · {}{}",
                    tl(ty.label()),
                    if is_out {
                        t("Ausgang", "Output")
                    } else {
                        t("Eingang", "Input")
                    },
                    if is_conn {
                        t(
                            " · Klick: verbinden · Rechtsklick: trennen",
                            " · click: connect · right-click: disconnect",
                        )
                    } else {
                        t(
                            " · Klick oder Ziehen: verbinden",
                            " · click or drag: connect",
                        )
                    }
                ));
            }
            app.canvas.pins.insert((nid, pin, is_out), (p, ty));
        }
        let _ = changed;
    }
    app.canvas.sizes = new_sizes;
    app.ui_rects.insert("canvas", rect);
    {
        let ws = &app.project.workspaces[ws_idx];
        let first = ws
            .nodes
            .iter()
            .filter(|n| {
                blocks::get(&n.kind)
                    .map(|d| d.kind == Kind::Event)
                    .unwrap_or(false)
            })
            .min_by(|a, b| (a.pos[1] + a.pos[0] * 0.3).total_cmp(&(b.pos[1] + b.pos[0] * 0.3)));
        if let Some(n) = first {
            let size = app
                .canvas
                .sizes
                .get(&n.id)
                .copied()
                .unwrap_or(vec2(260.0, 80.0));
            let r = Rect::from_min_size(pos2(n.pos[0], n.pos[1]), size);
            app.ui_rects.insert("first_event", to_screen * r);
        } else {
            app.ui_rects.remove("first_event");
        }
    }

    // ---------------------------------------------------------------- wires
    let mut hovered_link: Option<Link> = None;
    {
        let ws = &app.project.workspaces[ws_idx];
        let mut shapes = vec![];
        let mut best = f32::MAX;
        let mut mid_of_selected = None;
        for l in &ws.links {
            let a = app
                .canvas
                .pins
                .get(&(l.from.node, l.from.pin.clone(), true));
            let b = app.canvas.pins.get(&(l.to.node, l.to.pin.clone(), false));
            if let (Some((pa, _)), Some(_)) = (a, b) {
                let pb = app.canvas.pins[&(l.to.node, l.to.pin.clone(), false)].0;
                if let Some(ps) = pointer_scene {
                    if pointer_in && !over_popup && app.canvas.link_drag.is_none() {
                        let d = bezier_distance(*pa + vec2(6.0, 0.0), pb - vec2(6.0, 0.0), ps);
                        if d < 7.0 && d < best {
                            best = d;
                            hovered_link = Some(l.clone());
                        }
                    }
                }
            }
        }
        for l in &ws.links {
            let a = app
                .canvas
                .pins
                .get(&(l.from.node, l.from.pin.clone(), true));
            let b = app.canvas.pins.get(&(l.to.node, l.to.pin.clone(), false));
            if let (Some((pa, ta)), Some((pb, _))) = (a, b) {
                let sel = app.canvas.selected_link.as_ref() == Some(l);
                let hov = hovered_link.as_ref() == Some(l);
                let hl = sel
                    || hov
                    || app.selection.contains(&l.from.node)
                    || app.selection.contains(&l.to.node);
                let (col, w) = if *ta == PinType::Exec {
                    (theme::GREEN, 2.6)
                } else {
                    (ta.color(), 2.0)
                };
                let col = if sel || hov {
                    Color32::WHITE.lerp_to_gamma(col, 0.45)
                } else if hl {
                    col
                } else {
                    col.gamma_multiply(0.8)
                };
                let a2 = *pa + vec2(6.0, 0.0);
                let b2 = *pb - vec2(6.0, 0.0);
                if sel {
                    shapes.push(bezier(a2, b2, theme::ACCENT.gamma_multiply(0.35), w + 6.0));
                    mid_of_selected = Some(bezier_point(a2, b2, 0.5));
                }
                shapes.push(bezier(a2, b2, col, if hl { w + 1.0 } else { w }));
            }
        }
        sui.painter().set(wires_slot, Shape::Vec(shapes));
        // Delete button on the selected wire.
        if let Some(m) = mid_of_selected {
            let r = Rect::from_center_size(m, vec2(20.0, 20.0));
            let resp = sui.interact(r, Id::unique("wire_delete"), Sense::click());
            let fill = if resp.hovered() {
                theme::RED
            } else {
                Color32::from_rgb(0x3A, 0x3D, 0x43)
            };
            sui.painter().circle(
                m,
                10.0,
                fill,
                Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.6)),
            );
            crate::ui::icons::paint(
                sui.painter(),
                Rect::from_center_size(m, vec2(9.0, 9.0)),
                crate::ui::icons::Icon::Close,
                Color32::WHITE,
            );
            if resp.clicked() {
                if let Some(l) = app.canvas.selected_link.take() {
                    app.project.workspaces[ws_idx].links.retain(|x| *x != l);
                }
            }
            resp.on_hover_text(t("Verbindung löschen (Entf)", "Delete connection (Del)"));
        }
    }
    if hovered_link.is_some() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    // --------------------------------------------------------- link drag
    if let Some(d) = &app.canvas.link_drag {
        if let (Some((p, _)), Some(ps)) = (
            app.canvas.pins.get(&(d.node, d.pin.clone(), d.is_output)),
            pointer_scene,
        ) {
            let col = if d.ty == PinType::Exec {
                theme::GREEN
            } else {
                d.ty.color()
            };
            let shape = if d.is_output {
                bezier(*p, ps, col, 2.2)
            } else {
                bezier(ps, *p, col, 2.2)
            };
            sui.painter().add(shape);
        }
    }

    // ------------------------------------------------------- box select
    if let (Some(start), Some(ps)) = (app.canvas.box_start, pointer_scene) {
        let r = Rect::from_two_pos(start, ps);
        sui.painter().rect(
            r,
            2.0,
            theme::ACCENT.gamma_multiply(0.08),
            Stroke::new(1.0, theme::ACCENT),
            StrokeKind::Inside,
        );
        if ctx.input(|i| i.pointer.primary_released()) || !ctx.input(|i| i.pointer.primary_down()) {
            let add = ctx.input(|i| i.modifiers.command);
            if !add {
                app.selection.clear();
            }
            let ws = &app.project.workspaces[ws_idx];
            for n in &ws.nodes {
                let s = app
                    .canvas
                    .sizes
                    .get(&n.id)
                    .copied()
                    .unwrap_or(vec2(200.0, 80.0));
                if r.intersects(Rect::from_min_size(pos2(n.pos[0], n.pos[1]), s)) {
                    app.selection.insert(n.id);
                }
            }
            app.canvas.box_start = None;
        }
    }

    // ------------------------------------------------------ apply events
    let snap = app.settings.snap;
    for ev in events {
        match ev {
            NodeEvent::Select { id, add } => {
                app.selected_note = None;
                if add {
                    if !app.selection.remove(&id) {
                        app.selection.insert(id);
                    }
                } else {
                    app.selection.clear();
                    app.selection.insert(id);
                }
                app.right_tab = crate::app::RightTab::Inspector;
            }
            NodeEvent::Drag { id, delta } => {
                let ids: Vec<NodeId> = if app.selection.contains(&id) {
                    app.selection.iter().copied().collect()
                } else {
                    vec![id]
                };
                let ws = &mut app.project.workspaces[ws_idx];
                for i in ids {
                    if let Some(n) = ws.node_mut(i) {
                        n.pos[0] += delta.x;
                        n.pos[1] += delta.y;
                    }
                }
            }
            NodeEvent::DragStop => {
                if snap {
                    let sel = app.selection.clone();
                    let ws = &mut app.project.workspaces[ws_idx];
                    for n in ws.nodes.iter_mut().filter(|n| sel.contains(&n.id)) {
                        let p = snap_pos(pos2(n.pos[0], n.pos[1]));
                        n.pos = [p.x, p.y];
                    }
                }
            }
            NodeEvent::PinClick(d) => {
                let ws = &mut app.project.workspaces[ws_idx];
                match (&app.canvas.link_drag, app.canvas.click_link) {
                    (Some(start), true) => {
                        let cand = (d.node, d.pin.clone(), d.is_output, d.ty);
                        if let Some((from, to)) = link_pair(ws, start, &cand) {
                            connect(ws, from, to, start.ty == PinType::Exec);
                        }
                        app.canvas.link_drag = None;
                        app.canvas.click_link = false;
                    }
                    _ => {
                        app.canvas.link_drag = Some(d);
                        app.canvas.click_link = true;
                    }
                }
            }
            NodeEvent::PinDrag(d) => {
                app.canvas.click_link = false;
                let ws = &mut app.project.workspaces[ws_idx];
                // Dragging from a connected data input picks up the existing wire.
                if !d.is_output && d.ty != PinType::Exec {
                    if let Some(pos) = ws
                        .links
                        .iter()
                        .position(|l| l.to.node == d.node && l.to.pin == d.pin)
                    {
                        let l = ws.links.remove(pos);
                        let ty = blocks::get(
                            &ws.node(l.from.node)
                                .map(|n| n.kind.clone())
                                .unwrap_or_default(),
                        )
                        .and_then(|def| def.output(&l.from.pin).map(|p| p.ty))
                        .unwrap_or(d.ty);
                        app.canvas.link_drag = Some(LinkDrag {
                            node: l.from.node,
                            pin: l.from.pin,
                            is_output: true,
                            ty,
                        });
                        continue;
                    }
                }
                app.canvas.link_drag = Some(d);
            }
            NodeEvent::PinClear {
                node,
                pin,
                is_output,
            } => {
                let ws = &mut app.project.workspaces[ws_idx];
                ws.links.retain(|l| {
                    if is_output {
                        !(l.from.node == node && l.from.pin == pin)
                    } else {
                        !(l.to.node == node && l.to.pin == pin)
                    }
                });
            }
            NodeEvent::Delete(id) => {
                if app.selection.contains(&id) {
                    app.delete_selection();
                } else {
                    app.ws_mut().remove_node(id);
                }
            }
            NodeEvent::Duplicate(id) => {
                if !app.selection.contains(&id) {
                    app.selection.clear();
                    app.selection.insert(id);
                }
                app.duplicate(&ctx);
            }
            NodeEvent::Disconnect(id) => {
                app.ws_mut()
                    .links
                    .retain(|l| l.from.node != id && l.to.node != id);
            }
        }
    }

    // Finish a click-connection on empty space: offer matching blocks.
    if app.canvas.click_link && app.canvas.link_drag.is_some() && bg_resp.clicked() {
        let d = app.canvas.link_drag.take().unwrap();
        app.canvas.click_link = false;
        if let (Some(ps), Some(p)) = (pointer_scene, pointer) {
            app.palette = Some(Palette {
                scene_pos: ps,
                screen_pos: p,
                query: String::new(),
                selected: 0,
                connect: Some((d.node, d.pin.clone(), d.is_output, d.ty)),
                just_opened: true,
            });
        }
    }

    // Finish a link drag.
    if app.canvas.link_drag.is_some()
        && !app.canvas.click_link
        && ctx.input(|i| i.pointer.primary_released() || !i.pointer.primary_down())
    {
        let d = app.canvas.link_drag.take().unwrap();
        let mut done = false;
        if let Some(ps) = pointer_scene {
            let mut best: Option<((NodeId, String, bool, PinType), f32)> = None;
            for ((n, p, o), (pos, ty)) in &app.canvas.pins {
                let dist = pos.distance(ps);
                if dist < 16.0 && best.as_ref().map(|b| dist < b.1).unwrap_or(true) {
                    best = Some(((*n, p.clone(), *o, *ty), dist));
                }
            }
            if let Some((cand, _)) = best {
                let ws = &mut app.project.workspaces[ws_idx];
                if let Some((from, to)) = link_pair(ws, &d, &cand) {
                    connect(ws, from, to, d.ty == PinType::Exec);
                }
                done = true;
            }
            if !done && pointer_in {
                // Dropped on empty space: offer matching blocks.
                app.palette = Some(Palette {
                    scene_pos: ps,
                    screen_pos: pointer.unwrap(),
                    query: String::new(),
                    selected: 0,
                    connect: Some((d.node, d.pin.clone(), d.is_output, d.ty)),
                    just_opened: true,
                });
            }
        }
    }

    // ------------------------------------------------- background clicks
    if bg_resp.clicked() {
        app.selection.clear();
        app.selected_note = None;
        app.palette = None;
        app.canvas.selected_link = hovered_link.clone();
    }
    if bg_resp.double_clicked() && hovered_link.is_none() {
        if let Some(p) = bg_resp.interact_pointer_pos() {
            app.palette = Some(Palette {
                scene_pos: to_scene * p,
                screen_pos: p,
                query: String::new(),
                selected: 0,
                connect: None,
                just_opened: true,
            });
        }
    }
    if bg_resp.secondary_clicked()
        && bg_resp
            .total_drag_delta()
            .map(|d| d.length() < 4.0)
            .unwrap_or(true)
    {
        if let Some(p) = bg_resp.interact_pointer_pos() {
            app.palette = Some(Palette {
                scene_pos: to_scene * p,
                screen_pos: p,
                query: String::new(),
                selected: 0,
                connect: None,
                just_opened: true,
            });
        }
    }
    // Drop from the block library.
    if let Some(kind) = bg_resp.dnd_release_payload::<String>() {
        if let Some(p) = pointer {
            app.add_node(&kind, to_scene * p);
        }
    }

    // Empty hint
    if app.project.workspaces[ws_idx].nodes.is_empty()
        && app.project.workspaces[ws_idx].notes.is_empty()
    {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            t(
                "Doppelklick, Rechtsklick, Leertaste oder „+ Block“ zum Hinzufügen von Blöcken",
                "Double-click, right-click, space or “+ Block” to add blocks",
            ),
            FontId::proportional(15.0),
            theme::TEXT_WEAK,
        );
    }

    keyboard(app, &ctx, pointer_in, pointer_scene, rect, to_scene);
    toolbar(app, ui, rect, to_scene);
    crate::ui::palette::show(app, &ctx);
}

fn row(ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> Rect {
    let inner_w = ui.max_rect().width() - 2.0 * PAD_X;
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(PAD_X as i8, 0))
        .show(ui, |ui| {
            ui.set_width(inner_w);
            ui.allocate_ui_with_layout(
                vec2(inner_w, ROW_H),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_size(vec2(inner_w, ROW_H));
                    add(ui);
                },
            );
        })
        .response
        .rect
}

/// Left-aligned, truncated label of fixed width.
fn row_label(ui: &mut Ui, w: f32, text: egui::RichText) {
    ui.allocate_ui_with_layout(
        vec2(w, ROW_H),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(w);
            ui.add(egui::Label::new(text).truncate());
        },
    );
}

fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn keyboard(
    app: &mut App,
    ctx: &egui::Context,
    pointer_in: bool,
    pointer_scene: Option<Pos2>,
    rect: Rect,
    to_scene: TSTransform,
) {
    let typing = ctx.memory(|m| m.focused().is_some());
    if typing || app.dialog != crate::app::Dialog::None {
        return;
    }
    // Clipboard events come as events, not key presses.
    let evs = ctx.input(|i| i.events.clone());
    for e in evs {
        match e {
            egui::Event::Copy => app.copy_selection(ctx),
            egui::Event::Cut => {
                app.copy_selection(ctx);
                app.delete_selection();
            }
            egui::Event::Paste(s) => {
                let clip = s
                    .strip_prefix("MCB-CLIP:")
                    .and_then(|j| serde_json::from_str::<Clip>(j).ok())
                    .or_else(|| app.clipboard.clone());
                if let Some(c) = clip {
                    let at = if pointer_in { pointer_scene } else { None };
                    app.paste(c, at);
                }
            }
            _ => {}
        }
    }
    use egui::{Key, Modifiers};
    let (del, esc, space, dup, all, fit) = ctx.input_mut(|i| {
        (
            i.consume_key(Modifiers::NONE, Key::Delete)
                || i.consume_key(Modifiers::NONE, Key::Backspace),
            i.consume_key(Modifiers::NONE, Key::Escape),
            i.consume_key(Modifiers::NONE, Key::Space),
            i.consume_key(Modifiers::COMMAND, Key::D),
            i.consume_key(Modifiers::COMMAND, Key::A),
            i.consume_key(Modifiers::NONE, Key::F),
        )
    });
    if del {
        if let Some(l) = app.canvas.selected_link.take() {
            app.ws_mut().links.retain(|x| *x != l);
        } else {
            app.delete_selection();
        }
    }
    if esc {
        app.selection.clear();
        app.selected_note = None;
        app.palette = None;
        app.canvas.link_drag = None;
        app.canvas.click_link = false;
        app.canvas.selected_link = None;
    }
    if dup {
        app.duplicate(ctx);
    }
    if all {
        app.selection = app.ws().nodes.iter().map(|n| n.id).collect();
    }
    if fit {
        app.canvas.fit_request = true;
    }
    if space && app.palette.is_none() {
        let screen = ctx
            .pointer_hover_pos()
            .filter(|p| rect.contains(*p))
            .unwrap_or(rect.center());
        app.palette = Some(Palette {
            scene_pos: to_scene * screen,
            screen_pos: screen,
            query: String::new(),
            selected: 0,
            connect: None,
            just_opened: true,
        });
    }
}

pub fn fit_view(app: &mut App, rect: Rect) {
    let ws = app.ws();
    let mut bounds = Rect::NOTHING;
    for n in &ws.nodes {
        let s = app
            .canvas
            .sizes
            .get(&n.id)
            .copied()
            .unwrap_or(vec2(240.0, 120.0));
        bounds = bounds.union(Rect::from_min_size(pos2(n.pos[0], n.pos[1]), s));
    }
    for n in &ws.notes {
        bounds = bounds.union(Rect::from_min_size(
            pos2(n.pos[0], n.pos[1]),
            vec2(n.size[0], n.size[1]),
        ));
    }
    let view = app.view();
    if !bounds.is_positive() {
        *view = Default::default();
        return;
    }
    let bounds = bounds.expand(40.0);
    let fit = (rect.width() / bounds.width()).min(rect.height() / bounds.height());
    let zoom = fit.clamp(0.75, 1.0);
    view.zoom = zoom;
    if fit >= 0.75 {
        view.pan = rect.size() / 2.0 - bounds.center().to_vec2() * zoom;
    } else {
        // Too big to fit readably: show the top-left corner.
        view.pan = -bounds.min.to_vec2() * zoom;
    }
}

/// Square toolbar button with an icon; `active` highlights it.
fn tool_icon(ui: &mut Ui, icon: crate::ui::icons::Icon, active: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(32.0, 30.0), Sense::click());
    let fill = if resp.hovered() {
        Color32::from_rgb(0x30, 0x33, 0x39)
    } else {
        Color32::from_rgb(0x26, 0x28, 0x2D)
    };
    let stroke = if active {
        theme::ACCENT.gamma_multiply(0.7)
    } else {
        Color32::from_rgb(0x44, 0x47, 0x4E)
    };
    ui.painter()
        .rect(r, 6.0, fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    crate::ui::icons::paint(
        ui.painter(),
        r.shrink2(vec2(8.5, 7.5)),
        icon,
        if active { theme::ACCENT } else { theme::TEXT },
    );
    resp
}

fn toolbar(app: &mut App, ui: &mut Ui, rect: Rect, to_scene: TSTransform) {
    let pos = rect.left_bottom() + vec2(14.0, -48.0);
    egui::Area::new(Id::unique("canvas_toolbar"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if crate::ui::icons::button(
                    ui,
                    Some(crate::ui::icons::Icon::Plus),
                    t("Block", "Block"),
                )
                .on_hover_text(t("Block hinzufügen (Leertaste)", "Add block (space)"))
                .clicked()
                {
                    let c = rect.center();
                    app.palette = Some(Palette {
                        scene_pos: to_scene * c,
                        screen_pos: pos + vec2(0.0, -380.0),
                        query: String::new(),
                        selected: 0,
                        connect: None,
                        just_opened: true,
                    });
                }
                if crate::ui::icons::button(
                    ui,
                    Some(crate::ui::icons::Icon::Note),
                    t("Notiz", "Note"),
                )
                .on_hover_text(t("Notiz-Rahmen hinzufügen", "Add a note frame"))
                .clicked()
                {
                    let c = to_scene * rect.center();
                    let id = app.project.alloc_id();
                    app.ws_mut().notes.push(Note {
                        id,
                        pos: [c.x - 200.0, c.y - 120.0],
                        size: [480.0, 300.0],
                        title: t("Neue Notiz", "New note").into(),
                        text: String::new(),
                        color: 0,
                    });
                    app.selected_note = Some(id);
                    app.selection.clear();
                }
                let snap = app.settings.snap;
                let snap_btn = tool_icon(ui, crate::ui::icons::Icon::Grid, snap)
                    .on_hover_text(t("Am Raster ausrichten", "Snap to grid"));
                if snap_btn.clicked() {
                    app.settings.snap = !snap;
                }
                ui.add_space(6.0);
                let zoom = app.view().zoom;
                let zoom_to = |app: &mut App, nz: f32| {
                    let view = app.view();
                    let c = rect.size() / 2.0;
                    let scene = (c - view.pan) / view.zoom;
                    view.zoom = nz.clamp(0.25, 1.6);
                    view.pan = c - scene * view.zoom;
                };
                if tool_icon(ui, crate::ui::icons::Icon::Minus, false)
                    .on_hover_text(t("Verkleinern", "Zoom out"))
                    .clicked()
                {
                    zoom_to(app, zoom / 1.2);
                }
                if crate::ui::icons::button(ui, None, &format!("{:.0}%", zoom * 100.0))
                    .on_hover_text(t("Auf 100 % setzen", "Reset to 100 %"))
                    .clicked()
                {
                    zoom_to(app, 1.0);
                }
                if tool_icon(ui, crate::ui::icons::Icon::Plus, false)
                    .on_hover_text(t("Vergrößern", "Zoom in"))
                    .clicked()
                {
                    zoom_to(app, zoom * 1.2);
                }
                if tool_icon(ui, crate::ui::icons::Icon::Expand, false)
                    .on_hover_text(t("Alles anzeigen (F)", "Fit all (F)"))
                    .clicked()
                {
                    app.canvas.fit_request = true;
                }
            });
        });
}
