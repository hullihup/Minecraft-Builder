//! Start screen: all projects as cards, quick actions and templates.

use crate::app::{App, Dialog};
use crate::examples::Starter;
use crate::i18n::{t, tl};
use crate::model::Platform;
use crate::theme;
use crate::ui::icons::{self, Icon};
use egui::{
    Color32, CornerRadius, FontId, Id, Margin, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2,
    vec2,
};
use std::path::PathBuf;

fn ago(t_: std::time::SystemTime) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(t_)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let de = crate::i18n::is_german();
    match secs {
        0..=59 => (if de { "gerade eben" } else { "just now" }).into(),
        60..=3599 => {
            let m = secs / 60;
            if de {
                format!("vor {m} Min.")
            } else {
                format!("{m} min ago")
            }
        }
        3600..=86399 => {
            let h = secs / 3600;
            if de {
                format!("vor {h} Std.")
            } else {
                format!("{h} h ago")
            }
        }
        _ => {
            let d = secs / 86400;
            if de {
                format!("vor {d} Tag{}", if d == 1 { "" } else { "en" })
            } else {
                format!("{d} day{} ago", if d == 1 { "" } else { "s" })
            }
        }
    }
}

fn side_button(ui: &mut Ui, icon: Icon, text: &str, primary: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    let fill = if primary {
        if resp.hovered() {
            Color32::from_rgb(0x33, 0xF0, 0xFF)
        } else {
            theme::ACCENT
        }
    } else if resp.hovered() {
        Color32::from_rgb(0x2E, 0x31, 0x37)
    } else {
        Color32::from_rgb(0x26, 0x28, 0x2D)
    };
    ui.painter().rect(
        r,
        8.0,
        fill,
        Stroke::new(
            1.0,
            if primary {
                theme::ACCENT
            } else {
                theme::BORDER
            },
        ),
        StrokeKind::Inside,
    );
    let fg = if primary { Color32::BLACK } else { theme::TEXT };
    icons::paint(
        ui.painter(),
        Rect::from_center_size(r.left_center() + vec2(24.0, 0.0), vec2(16.0, 16.0)),
        icon,
        fg,
    );
    ui.painter().text(
        r.left_center() + vec2(44.0, 0.0),
        egui::Align2::LEFT_CENTER,
        text,
        FontId::proportional(14.5),
        fg,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::Panel::left("home_left")
        .exact_size(300.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(theme::BG_SIDEBAR)
                .inner_margin(Margin::same(24))
                .stroke(Stroke::new(1.0, theme::BORDER)),
        )
        .show(ui, |ui| left_column(app, ui));
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let r = ui.max_rect();
        ui.painter().rect_filled(r, 0.0, theme::BG_CANVAS);
        // Subtle dot grid like the editor.
        let step = 24.0;
        let mut y = r.top() + 12.0;
        while y < r.bottom() {
            let mut x = r.left() + 12.0;
            while x < r.right() {
                ui.painter()
                    .circle_filled(pos2(x, y), 1.0, theme::GRID_DOT.gamma_multiply(0.6));
                x += step;
            }
            y += step;
        }
        egui::Frame::new()
            .inner_margin(Margin::symmetric(40, 32))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| projects(app, ui));
            });
    });
}

fn left_column(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
        let tex = theme::logo(ui.ctx());
        ui.painter().image(
            tex.id(),
            r,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Minecraft Builder")
                    .size(19.0)
                    .strong()
                    .color(theme::TEXT),
            );
            ui.label(
                RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                    .size(11.5)
                    .color(theme::TEXT_WEAK),
            );
        });
    });
    ui.add_space(26.0);
    if side_button(ui, Icon::Plus, t("Neues Projekt", "New project"), true).clicked() {
        app.new_project_dialog();
    }
    ui.add_space(8.0);
    if side_button(
        ui,
        Icon::Folder,
        t("Projekt öffnen…", "Open project…"),
        false,
    )
    .clicked()
    {
        app.open_dialog();
    }
    ui.add_space(8.0);
    if side_button(
        ui,
        Icon::Book,
        t("Tutorial starten", "Start tutorial"),
        false,
    )
    .clicked()
    {
        crate::ui::onboarding::start_tutorial(app);
    }
    ui.add_space(8.0);
    if side_button(
        ui,
        Icon::Gear,
        t("Einrichtung & Sprache", "Setup & language"),
        false,
    )
    .clicked()
    {
        app.onboarding = Some(0);
    }

    ui.add_space(26.0);
    crate::ui::widgets::section_header(ui, t("Werkzeuge", "Tools"));
    ui.add_space(6.0);
    tool_status(app, ui, &ctx);

    if app.update.release.is_some() {
        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| crate::ui::panels::update_badge(app, ui));
    }
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.label(
            RichText::new(t(
                "Nicht offiziell von Mojang oder Microsoft.",
                "Not affiliated with Mojang or Microsoft.",
            ))
            .size(10.5)
            .color(theme::TEXT_WEAK),
        );
        ui.horizontal(|ui| {
            if ui
                .selectable_label(crate::i18n::is_german(), "Deutsch")
                .clicked()
            {
                crate::i18n::set_german(true);
                app.settings.german = true;
                app.settings.save();
            }
            if ui
                .selectable_label(!crate::i18n::is_german(), "English")
                .clicked()
            {
                crate::i18n::set_german(false);
                app.settings.german = false;
                app.settings.save();
            }
        });
    });
}

/// Java / Gradle status with a setup button. Shared by the start screen and the build panel.
pub fn tool_status(app: &mut App, ui: &mut Ui, ctx: &egui::Context) {
    let st = crate::tools::status();
    let java = st.java.clone();
    let gradle = st.gradle;
    let line = |ui: &mut Ui, ok: bool, text: String| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(if ok { "✔" } else { "⚠" }).color(if ok {
                theme::GREEN
            } else {
                theme::YELLOW
            }));
            ui.label(RichText::new(text).size(12.5));
        });
    };
    match &java {
        Some(_) => line(
            ui,
            true,
            format!(
                "Java {}",
                st.java_version
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".into())
            ),
        ),
        None => line(ui, false, t("Java fehlt", "Java missing").into()),
    }
    line(
        ui,
        gradle,
        if gradle {
            t("Gradle bereit", "Gradle ready").into()
        } else {
            t("Gradle noch nicht geladen", "Gradle not downloaded yet").into()
        },
    );

    let running = app
        .setup
        .as_ref()
        .map(|s| s.result.is_none())
        .unwrap_or(false);
    if let Some(s) = &app.setup {
        ui.add_space(4.0);
        ui.label(RichText::new(&s.status).size(11.5).color(theme::TEXT_WEAK));
        if running {
            if s.progress >= 0.0 {
                ui.add(egui::ProgressBar::new(s.progress).desired_height(6.0));
            } else {
                ui.add(
                    egui::ProgressBar::new(0.0)
                        .animate(true)
                        .desired_height(6.0),
                );
            }
        }
        if let Some(Err(e)) = &s.result {
            ui.label(RichText::new(e).size(11.5).color(theme::RED));
        }
    }
    if !running && (java.is_none() || !gradle) {
        ui.add_space(4.0);
        if ui
            .button(t("Automatisch einrichten", "Set up automatically"))
            .clicked()
        {
            app.start_tools_setup(ctx, false);
        }
    }
}

fn projects(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(t("Deine Projekte", "Your projects"))
                .size(24.0)
                .strong()
                .color(theme::TEXT),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.home_search)
                    .hint_text(t("Projekte suchen…", "Search projects…"))
                    .desired_width(240.0),
            );
        });
    });
    ui.add_space(18.0);

    let mut paths: Vec<PathBuf> = app.settings.recent.clone();
    // Also list projects in the default projects folder.
    if let Ok(rd) = std::fs::read_dir(crate::build::default_projects_dir()) {
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            let candidates: Vec<PathBuf> = if p.is_dir() {
                std::fs::read_dir(&p)
                    .map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).collect())
                    .unwrap_or_default()
            } else {
                vec![p]
            };
            for c in candidates {
                if c.extension().map(|x| x == "mcb").unwrap_or(false) && !paths.contains(&c) {
                    paths.push(c);
                }
            }
        }
    }
    let q = app.home_search.to_lowercase();
    let mut cards = vec![];
    for p in paths {
        if !p.is_file() {
            continue;
        }
        if let Some(s) = app.project_summary(&p) {
            if q.is_empty() || s.name.to_lowercase().contains(&q) {
                cards.push((p, s));
            }
        }
    }
    cards.sort_by(|a, b| b.1.modified.cmp(&a.1.modified));

    let mut open = None;
    let mut remove = None;
    if cards.is_empty() {
        egui::Frame::new()
            .fill(Color32::from_rgb(0x26, 0x28, 0x2D))
            .corner_radius(10)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .inner_margin(24)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(t("Noch keine Projekte", "No projects yet"))
                        .size(17.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t(
                        "Starte mit einer Vorlage unten oder klicke links auf „Neues Projekt“.",
                        "Start with a template below or click “New project” on the left.",
                    ))
                    .color(theme::TEXT_WEAK),
                );
            });
    } else {
        let card_w = 300.0;
        let gap = 16.0;
        let cols = (((ui.available_width() + gap) / (card_w + gap)).floor() as usize).max(1);
        for row in cards.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (path, s) in row {
                    let (r, resp) = ui.allocate_exact_size(vec2(card_w, 128.0), Sense::click());
                    let hov = resp.hovered();
                    let col = match s.platform {
                        Platform::Paper => theme::GREEN,
                        Platform::Fabric => theme::BLUE,
                    };
                    let p = ui.painter();
                    p.rect(
                        r,
                        10.0,
                        if hov {
                            Color32::from_rgb(0x2E, 0x31, 0x37)
                        } else {
                            Color32::from_rgb(0x28, 0x2A, 0x2F)
                        },
                        Stroke::new(
                            1.0,
                            if hov {
                                theme::ACCENT.gamma_multiply(0.7)
                            } else {
                                theme::BORDER
                            },
                        ),
                        StrokeKind::Inside,
                    );
                    p.rect_filled(
                        Rect::from_min_size(r.min, vec2(4.0, r.height())),
                        CornerRadius {
                            nw: 10,
                            sw: 10,
                            ne: 0,
                            se: 0,
                        },
                        col,
                    );
                    p.text(
                        r.min + vec2(20.0, 18.0),
                        egui::Align2::LEFT_TOP,
                        &s.name,
                        FontId::proportional(17.0),
                        theme::TEXT,
                    );
                    let badge = format!("{} · Minecraft {}", s.platform.label(), s.minecraft);
                    p.text(
                        r.min + vec2(20.0, 46.0),
                        egui::Align2::LEFT_TOP,
                        badge,
                        FontId::proportional(12.0),
                        col,
                    );
                    p.text(
                        r.min + vec2(20.0, 72.0),
                        egui::Align2::LEFT_TOP,
                        format!(
                            "{} {} · {} {}",
                            s.blocks,
                            t("Blöcke", "blocks"),
                            s.commands,
                            t("Befehle", "commands")
                        ),
                        FontId::proportional(12.0),
                        theme::TEXT_WEAK,
                    );
                    p.text(
                        r.left_bottom() + vec2(20.0, -18.0),
                        egui::Align2::LEFT_CENTER,
                        ago(s.modified),
                        FontId::proportional(11.5),
                        theme::TEXT_WEAK,
                    );
                    let folder = path
                        .parent()
                        .map(|d| d.display().to_string())
                        .unwrap_or_default();
                    let short: String = if folder.chars().count() > 34 {
                        format!(
                            "…{}",
                            folder
                                .chars()
                                .rev()
                                .take(33)
                                .collect::<Vec<_>>()
                                .into_iter()
                                .rev()
                                .collect::<String>()
                        )
                    } else {
                        folder
                    };
                    p.text(
                        r.right_bottom() + vec2(-14.0, -18.0),
                        egui::Align2::RIGHT_CENTER,
                        short,
                        FontId::proportional(10.5),
                        theme::TEXT_WEAK.gamma_multiply(0.8),
                    );
                    if hov {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if resp.clicked() {
                        open = Some(path.clone());
                    }
                    resp.context_menu(|ui| {
                        if ui.button(t("Öffnen", "Open")).clicked() {
                            open = Some(path.clone());
                            ui.close();
                        }
                        if ui.button(t("Ordner öffnen", "Open folder")).clicked() {
                            if let Some(d) = path.parent() {
                                let _ = open::that_detached(d);
                            }
                            ui.close();
                        }
                        if ui
                            .button(t("Aus der Liste entfernen", "Remove from list"))
                            .clicked()
                        {
                            remove = Some(path.clone());
                            ui.close();
                        }
                    });
                }
            });
            ui.add_space(gap);
        }
    }

    ui.add_space(24.0);
    ui.label(
        RichText::new(t("Mit einer Vorlage starten", "Start from a template"))
            .size(18.0)
            .strong(),
    );
    ui.add_space(12.0);
    let mut start = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(16.0, 16.0);
        for s in Starter::ALL {
            let (r, resp) = ui.allocate_exact_size(vec2(230.0, 96.0), Sense::click());
            let hov = resp.hovered();
            ui.painter().rect(
                r,
                10.0,
                if hov {
                    Color32::from_rgb(0x2E, 0x31, 0x37)
                } else {
                    Color32::from_rgb(0x24, 0x26, 0x2B)
                },
                Stroke::new(
                    1.0,
                    if hov {
                        theme::GREEN.gamma_multiply(0.7)
                    } else {
                        theme::BORDER
                    },
                ),
                StrokeKind::Inside,
            );
            ui.painter().text(
                r.min + vec2(14.0, 14.0),
                egui::Align2::LEFT_TOP,
                tl(s.label()),
                FontId::proportional(14.0),
                theme::TEXT,
            );
            let g = ui.painter().layout(
                tl(s.desc()).to_string(),
                FontId::proportional(11.0),
                theme::TEXT_WEAK,
                r.width() - 28.0,
            );
            ui.painter()
                .galley(r.min + vec2(14.0, 38.0), g, theme::TEXT_WEAK);
            if hov {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                start = Some(s);
            }
        }
    });

    if let Some(p) = open {
        if p.is_file() {
            app.open_path(p);
        } else {
            app.dialog = Dialog::Message {
                title: t("Nicht gefunden", "Not found").into(),
                text: p.display().to_string(),
            };
        }
    }
    if let Some(p) = remove {
        app.settings.recent.retain(|r| *r != p);
        app.settings.save();
    }
    if let Some(s) = start {
        app.new_project_with(s);
    }
    let _ = Id::NULL;
}
