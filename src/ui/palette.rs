//! "Add block" popup with search, keyboard navigation and auto-connect.

use crate::app::App;
use crate::blocks::{self, BlockDef, CATEGORIES, PinType};
use crate::i18n::{t, tl};
use crate::model::PinRef;
use crate::theme;
use crate::ui::canvas::connect as canvas_connect;
use egui::{Color32, Id, RichText, vec2};

fn matches_query(d: &BlockDef, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let q = q.to_lowercase();
    q.split_whitespace().all(|w| {
        d.name.0.to_lowercase().contains(w)
            || d.name.1.to_lowercase().contains(w)
            || d.id.contains(w)
            || d.desc.0.to_lowercase().contains(w)
            || d.desc.1.to_lowercase().contains(w)
            || tl(d.cat.label()).to_lowercase().contains(w)
    })
}

/// Pin of `d` that fits the dragged pin, if any.
pub fn fitting_pin(d: &BlockDef, ty: PinType, dragged_is_output: bool) -> Option<&'static str> {
    if dragged_is_output {
        d.inputs
            .iter()
            .find(|p| {
                if ty == PinType::Exec {
                    p.ty == PinType::Exec
                } else {
                    p.ty != PinType::Exec && blocks::compatible(ty, p.ty)
                }
            })
            .map(|p| p.id)
    } else {
        d.outputs
            .iter()
            .find(|p| {
                if ty == PinType::Exec {
                    p.ty == PinType::Exec
                } else {
                    p.ty != PinType::Exec && blocks::compatible(p.ty, ty)
                }
            })
            .map(|p| p.id)
    }
}

/// 0 = name starts with query, 1 = name contains, 2 = other match.
fn score(d: &BlockDef, q: &str) -> u8 {
    let q = q.trim().to_lowercase();
    if q.is_empty() {
        return 0;
    }
    let names = [d.name.0.to_lowercase(), d.name.1.to_lowercase()];
    if names
        .iter()
        .any(|n| n.starts_with(&q) || n.split_whitespace().any(|w| w.starts_with(&q)))
    {
        0
    } else if names.iter().any(|n| n.contains(&q)) {
        1
    } else {
        2
    }
}

pub fn candidates(app: &App, query: &str) -> Vec<&'static BlockDef> {
    let plat = app.project.settings.platform;
    let connect = app.palette.as_ref().and_then(|p| p.connect.clone());
    let mut out = vec![];
    for cat in CATEGORIES {
        for d in blocks::all().iter().filter(|d| d.cat == cat) {
            if !d.supports(plat) || !matches_query(d, query) {
                continue;
            }
            if let Some((_, _, is_out, ty)) = &connect {
                if fitting_pin(d, *ty, *is_out).is_none() {
                    continue;
                }
            }
            out.push(d);
        }
    }
    // Best name matches first; category order is kept within each group.
    out.sort_by_key(|d| score(d, query));
    out
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(pal) = app.palette.as_ref() else {
        return;
    };
    let query = pal.query.clone();
    let list = candidates(app, &query);
    let screen = ctx.content_rect();
    let mut pos = pal.screen_pos;
    pos.x = pos.x.min(screen.right() - 330.0).max(screen.left() + 4.0);
    pos.y = pos.y.min(screen.bottom() - 440.0).max(screen.top() + 40.0);

    let mut chosen: Option<&'static BlockDef> = None;
    let mut close = false;
    let area = egui::Area::new(Id::unique("block_palette"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.set_width(310.0);
                    let pal = app.palette.as_mut().unwrap();
                    let te = ui.add(
                        egui::TextEdit::singleline(&mut pal.query)
                            .hint_text(t("Block suchen…", "Search blocks…"))
                            .desired_width(f32::INFINITY),
                    );
                    if pal.just_opened {
                        te.request_focus();
                        pal.just_opened = false;
                    }
                    if te.changed() {
                        pal.selected = 0;
                    }
                    let (up, down, enter, esc) = ui.input_mut(|i| {
                        (
                            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                            i.key_pressed(egui::Key::Enter),
                            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                        )
                    });
                    if down && !list.is_empty() {
                        pal.selected = (pal.selected + 1).min(list.len() - 1);
                    }
                    if up {
                        pal.selected = pal.selected.saturating_sub(1);
                    }
                    if enter && !list.is_empty() {
                        chosen = Some(list[pal.selected.min(list.len() - 1)]);
                    }
                    if esc {
                        close = true;
                    }
                    if let Some((_, _, _, ty)) = &pal.connect {
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                t("Passend zu:", "Matching:"),
                                tl(ty.label())
                            ))
                            .size(11.0)
                            .color(ty.color()),
                        );
                    }
                    ui.add_space(4.0);
                    let selected = pal.selected;
                    egui::ScrollArea::vertical()
                        .max_height(360.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            let mut last_cat = None;
                            if list.is_empty() {
                                ui.label(
                                    RichText::new(t("Nichts gefunden.", "Nothing found."))
                                        .color(theme::TEXT_WEAK),
                                );
                            }
                            for (i, d) in list.iter().enumerate() {
                                if last_cat != Some(d.cat) {
                                    last_cat = Some(d.cat);
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(tl(d.cat.label()).to_uppercase())
                                            .size(10.5)
                                            .color(theme::TEXT_WEAK)
                                            .strong(),
                                    );
                                }
                                let is_sel = i == selected;
                                let r = ui
                                    .horizontal(|ui| {
                                        ui.set_min_width(290.0);
                                        ui.label(RichText::new(d.cat.icon()).color(d.cat.color()));
                                        ui.label(RichText::new(tl(d.name)).color(if is_sel {
                                            Color32::WHITE
                                        } else {
                                            theme::TEXT
                                        }));
                                    })
                                    .response;
                                let r = ui.interact(
                                    r.rect.expand2(vec2(4.0, 1.0)),
                                    Id::unique(("pal", d.id)),
                                    egui::Sense::click(),
                                );
                                if is_sel || r.hovered() {
                                    ui.painter().rect_filled(
                                        r.rect,
                                        4.0,
                                        theme::ACCENT_BG.gamma_multiply(if is_sel {
                                            1.0
                                        } else {
                                            0.6
                                        }),
                                    );
                                    // Repaint the row content on top of the highlight.
                                    ui.painter().text(
                                        r.rect.left_center() + vec2(4.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        d.cat.icon(),
                                        egui::FontId::proportional(14.0),
                                        d.cat.color(),
                                    );
                                    ui.painter().text(
                                        r.rect.left_center() + vec2(26.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        tl(d.name),
                                        egui::FontId::proportional(14.0),
                                        Color32::WHITE,
                                    );
                                }
                                if is_sel && (up || down) {
                                    r.scroll_to_me(None);
                                }
                                let r = r.on_hover_text(tl(d.desc));
                                if r.clicked() {
                                    chosen = Some(d);
                                }
                            }
                        });
                });
        });

    // Click outside closes.
    if ctx.input(|i| i.pointer.any_pressed()) {
        if let Some(p) = ctx.pointer_interact_pos() {
            if !area.response.rect.contains(p)
                && !app.palette.as_ref().map(|p| p.just_opened).unwrap_or(false)
            {
                close = true;
            }
        }
    }

    if let Some(d) = chosen {
        let pal = app.palette.take().unwrap();
        let id = app.add_node(d.id, pal.scene_pos);
        if let Some((node, pin, is_out, ty)) = pal.connect {
            if let Some(p) = fitting_pin(d, ty, is_out) {
                let ws = app.ws_mut();
                let (from, to) = if is_out {
                    (
                        PinRef { node, pin },
                        PinRef {
                            node: id,
                            pin: p.to_string(),
                        },
                    )
                } else {
                    (
                        PinRef {
                            node: id,
                            pin: p.to_string(),
                        },
                        PinRef { node, pin },
                    )
                };
                canvas_connect(ws, from, to, ty == PinType::Exec);
            }
        }
    } else if close {
        app.palette = None;
    }
}
