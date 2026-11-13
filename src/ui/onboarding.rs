//! First-start wizard (language, Java/Gradle setup, tutorial offer) and the
//! guided tutorial that highlights parts of the editor.

use crate::app::{App, Screen};
use crate::examples::{self, Starter};
use crate::i18n::{is_german, t};
use crate::model::Platform;
use crate::theme;
use egui::{Color32, FontId, Id, LayerId, Order, Rect, RichText, Sense, Stroke, StrokeKind, vec2};

const STEPS: usize = 3;

fn big_choice(ui: &mut egui::Ui, title: &str, sub: &str, on: bool, w: f32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(w, 84.0), Sense::click());
    let hov = resp.hovered();
    ui.painter().rect(
        r,
        10.0,
        if on {
            theme::ACCENT_BG
        } else if hov {
            Color32::from_rgb(0x2E, 0x31, 0x37)
        } else {
            Color32::from_rgb(0x28, 0x2A, 0x2F)
        },
        Stroke::new(
            if on { 2.0 } else { 1.0 },
            if on { theme::ACCENT } else { theme::BORDER },
        ),
        StrokeKind::Inside,
    );
    ui.painter().text(
        r.min + vec2(18.0, 18.0),
        egui::Align2::LEFT_TOP,
        title,
        FontId::proportional(19.0),
        theme::TEXT,
    );
    ui.painter().text(
        r.min + vec2(18.0, 50.0),
        egui::Align2::LEFT_TOP,
        sub,
        FontId::proportional(12.0),
        theme::TEXT_WEAK,
    );
    if hov {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(
        RichText::new(text)
            .color(Color32::BLACK)
            .strong()
            .size(14.0),
    )
    .fill(theme::ACCENT)
    .corner_radius(8)
    .min_size(vec2(150.0, 36.0))
}

fn secondary(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).size(14.0))
        .corner_radius(8)
        .min_size(vec2(120.0, 36.0))
}

fn finish(app: &mut App) {
    app.onboarding = None;
    app.settings.onboarded = true;
    app.settings.save();
}

/// The first-start wizard.
pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(step) = app.onboarding else { return };
    let screen = ctx.content_rect();
    ctx.layer_painter(LayerId::new(Order::Foreground, Id::unique("onb_dim")))
        .rect_filled(screen, 0.0, Color32::from_black_alpha(200));

    let size = vec2(660.0, 470.0);
    let mut next = step;
    let mut done = false;
    egui::Area::new(Id::unique("onboarding"))
        .order(Order::Tooltip)
        .fixed_pos(screen.center() - size / 2.0)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgb(0x20, 0x22, 0x26))
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(14)
                .inner_margin(32)
                .shadow(egui::epaint::Shadow { offset: [0, 10], blur: 40, spread: 0, color: Color32::from_black_alpha(160) })
                .show(ui, |ui| {
                    ui.set_width(size.x - 64.0);
                    let top = ui.cursor().top();
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
                        let tex = theme::logo(ui.ctx());
                        ui.painter().image(tex.id(), r, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                        ui.label(RichText::new("Minecraft Builder").size(16.0).color(theme::TEXT_WEAK));
                    });
                    ui.add_space(14.0);
                    match step {
                        0 => {
                            ui.label(RichText::new("Willkommen! · Welcome!").size(26.0).strong());
                            ui.add_space(4.0);
                            ui.label(RichText::new("Wähle deine Sprache · Choose your language").color(theme::TEXT_WEAK));
                            ui.add_space(22.0);
                            let w = (ui.available_width() - 16.0) / 2.0;
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 16.0;
                                if big_choice(ui, "Deutsch", "Die Oberfläche auf Deutsch", is_german(), w).clicked() {
                                    crate::i18n::set_german(true);
                                    app.settings.german = true;
                                }
                                if big_choice(ui, "English", "Use the app in English", !is_german(), w).clicked() {
                                    crate::i18n::set_german(false);
                                    app.settings.german = false;
                                }
                            });
                        }
                        1 => {
                            ui.label(RichText::new(t("Java & Gradle einrichten", "Set up Java & Gradle")).size(26.0).strong());
                            ui.add_space(4.0);
                            ui.label(RichText::new(t(
                                "Um aus deinen Blöcken eine fertige .jar zu bauen, braucht der Builder Java und Gradle. \
                                 Das kann automatisch heruntergeladen werden (einmalig, ca. 300 MB).",
                                "To turn your blocks into a finished .jar, the builder needs Java and Gradle. \
                                 They can be downloaded automatically (one time, about 300 MB).",
                            )).color(theme::TEXT_WEAK));
                            ui.add_space(16.0);
                            crate::ui::home::tool_status(app, ui, ctx);
                            if let Some(s) = &app.setup {
                                ui.add_space(6.0);
                                egui::Frame::new().fill(theme::INPUT_BG).corner_radius(6).inner_margin(8).show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    egui::ScrollArea::vertical().max_height(110.0).stick_to_bottom(true).show(ui, |ui| {
                                        for l in s.lines.iter().rev().take(40).rev() {
                                            ui.label(RichText::new(l).monospace().size(11.0).color(theme::TEXT_WEAK));
                                        }
                                    });
                                });
                            }
                        }
                        _ => {
                            ui.label(RichText::new(t("Kurze Tour gefällig?", "Want a quick tour?")).size(26.0).strong());
                            ui.add_space(4.0);
                            ui.label(RichText::new(t(
                                "In etwa einer Minute zeigen wir dir die wichtigsten Stellen: Blöcke, Verbindungen, Inspector und den Bauen-Knopf. \
                                 Du kannst das Tutorial jederzeit über die Startseite oder „Hilfe“ wieder öffnen.",
                                "In about a minute we show you the important parts: blocks, wires, the inspector and the build button. \
                                 You can open the tutorial again any time from the start screen or “Help”.",
                            )).color(theme::TEXT_WEAK));
                            ui.add_space(22.0);
                            let w = (ui.available_width() - 16.0) / 2.0;
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 16.0;
                                if big_choice(ui, t("Tutorial starten", "Start tutorial"), t("Empfohlen für den Anfang", "Recommended for beginners"), false, w).clicked() {
                                    finish(app);
                                    start_tutorial(app);
                                    done = true;
                                }
                                if big_choice(ui, t("Überspringen", "Skip"), t("Direkt loslegen", "Jump right in"), false, w).clicked() {
                                    finish(app);
                                    done = true;
                                }
                            });
                        }
                    }

                    // Footer
                    let footer_y = top + size.y - 64.0 - 36.0;
                    let used = ui.cursor().top();
                    if footer_y > used {
                        ui.add_space(footer_y - used);
                    }
                    ui.horizontal(|ui| {
                        for i in 0..STEPS {
                            let (r, _) = ui.allocate_exact_size(vec2(if i == step { 22.0 } else { 8.0 }, 8.0), Sense::hover());
                            ui.painter().rect_filled(r, 4.0, if i == step { theme::ACCENT } else { theme::BORDER });
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if step + 1 < STEPS {
                                let label = if step == 1 && app.setup.as_ref().map(|s| s.result.is_none()).unwrap_or(false) {
                                    t("Im Hintergrund weiter", "Continue in background")
                                } else {
                                    t("Weiter", "Next")
                                };
                                if ui.add(primary(label)).clicked() {
                                    next = step + 1;
                                }
                            }
                            if step > 0 && ui.add(secondary(t("Zurück", "Back"))).clicked() {
                                next = step - 1;
                            }
                            if step == 0 && app.settings.onboarded && ui.add(secondary(t("Schließen", "Close"))).clicked() {
                                done = true;
                            }
                        });
                    });
                });
        });
    if done {
        app.onboarding = None;
        app.settings.save();
    } else if next != step {
        app.onboarding = Some(next);
        app.settings.save();
    }
}

// ---------------------------------------------------------------- tutorial

struct Step {
    target: &'static str,
    title: (&'static str, &'static str),
    text: (&'static str, &'static str),
}

const TUTORIAL: &[Step] = &[
    Step {
        target: "canvas",
        title: ("Die Arbeitsfläche", "The canvas"),
        text: (
            "Hier baust du dein Plugin aus Blöcken. Mit gedrückter Maustaste auf freier Fläche verschiebst du die Ansicht, mit dem Mausrad zoomst du.",
            "This is where you build your plugin from blocks. Drag empty space to move the view, use the mouse wheel to zoom.",
        ),
    },
    Step {
        target: "first_event",
        title: ("Events starten alles", "Events start everything"),
        text: (
            "Grüne Event-Blöcke wie „Spieler betritt Server“ oder „Befehl“ starten einen Ablauf. Die grünen Pfeile zeigen, was danach der Reihe nach passiert.",
            "Green event blocks like “Player joins” or “Command” start a flow. The green arrows show what happens next, step by step.",
        ),
    },
    Step {
        target: "canvas",
        title: ("Blöcke verbinden", "Connecting blocks"),
        text: (
            "Ziehe von einem Punkt zum anderen – oder klicke erst den einen und dann den anderen Punkt an. Farbige Punkte sind Werte: Spieler, Text, Zahl … \
             Ziehst du ein Kabel ins Leere, bekommst du direkt passende Blöcke. Ein Klick auf ein Kabel wählt es aus, Entf löscht es.",
            "Drag from one dot to another – or click one dot and then the other. Colored dots are values: player, text, number … \
             Drop a wire on empty space to get matching blocks. Click a wire to select it, Del removes it.",
        ),
    },
    Step {
        target: "search",
        title: ("Blöcke hinzufügen", "Adding blocks"),
        text: (
            "Klicke hier, mach einen Doppelklick oder Rechtsklick auf die Fläche oder drücke die Leertaste – dann kannst du nach Blöcken suchen.",
            "Click here, double-click or right-click the canvas, or press space – then search for blocks.",
        ),
    },
    Step {
        target: "right",
        title: ("Inspector & Java-Code", "Inspector & Java code"),
        text: (
            "Klicke einen Block an: Hier siehst du alle Einstellungen und eine Erklärung. Im Tab „Java-Code“ siehst du live, was der Builder erzeugt.",
            "Click a block: all its settings and an explanation show up here. The “Java code” tab shows the generated code live.",
        ),
    },
    Step {
        target: "activity",
        title: ("Seitenleiste", "Sidebar"),
        text: (
            "Arbeitsbereiche & Befehle, Block-Bibliothek, Texturen & Modelle (Resourcepack), Projekt-Einstellungen und Bauen & Export.",
            "Workspaces & commands, block library, textures & models (resource pack), project settings and build & export.",
        ),
    },
    Step {
        target: "bottom",
        title: ("Konsole & Probleme", "Console & problems"),
        text: (
            "Fehler stehen hier und direkt am Block (rotes Ausrufezeichen). Ein Klick auf ein Problem springt zum Block.",
            "Errors show up here and right at the block (red exclamation mark). Click a problem to jump to the block.",
        ),
    },
    Step {
        target: "build",
        title: ("Bauen!", "Build!"),
        text: (
            "Ein Klick (oder F5) erzeugt die fertige .jar-Datei. Bei Paper kommt sie in den Ordner plugins/ deines Servers, bei Fabric in mods/.",
            "One click (or F5) creates the finished .jar. For Paper put it into your server's plugins/ folder, for Fabric into mods/.",
        ),
    },
    Step {
        target: "",
        title: ("Fertig – viel Spaß!", "Done – have fun!"),
        text: (
            "Das Beispielprojekt begrüßt Spieler beim Betreten. Probiere es aus: ändere den Text, füge Blöcke hinzu und drück auf „Bauen“.",
            "The example project greets players when they join. Try it: change the text, add blocks and press “Build”.",
        ),
    },
];

pub fn start_tutorial(app: &mut App) {
    if app.screen == Screen::Home {
        let p = examples::new_project(t("Tutorial", "Tutorial"), Platform::Paper, Starter::Welcome);
        app.set_project(p, None);
    }
    app.show_sidebar = true;
    app.show_right = true;
    app.show_bottom = true;
    app.tutorial = Some(0);
}

pub fn tutorial(app: &mut App, ctx: &egui::Context) {
    let Some(step) = app.tutorial else { return };
    if step >= TUTORIAL.len() {
        app.tutorial = None;
        return;
    }
    let s = &TUTORIAL[step];
    let screen = ctx.content_rect();
    let target = app
        .ui_rects
        .get(s.target)
        .copied()
        .map(|r| r.intersect(screen).expand(4.0));

    // Dim everything except the target.
    let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::unique("tut_dim")));
    let dim = Color32::from_black_alpha(150);
    match target {
        Some(r) => {
            painter.rect_filled(
                Rect::from_min_max(screen.min, egui::pos2(screen.max.x, r.min.y)),
                0.0,
                dim,
            );
            painter.rect_filled(
                Rect::from_min_max(egui::pos2(screen.min.x, r.max.y), screen.max),
                0.0,
                dim,
            );
            painter.rect_filled(
                Rect::from_min_max(
                    egui::pos2(screen.min.x, r.min.y),
                    egui::pos2(r.min.x, r.max.y),
                ),
                0.0,
                dim,
            );
            painter.rect_filled(
                Rect::from_min_max(
                    egui::pos2(r.max.x, r.min.y),
                    egui::pos2(screen.max.x, r.max.y),
                ),
                0.0,
                dim,
            );
            let pulse = (ctx.input(|i| i.time) * 3.0).sin() as f32 * 0.5 + 0.5;
            painter.rect_stroke(
                r,
                8.0,
                Stroke::new(2.0 + pulse, theme::ACCENT),
                StrokeKind::Outside,
            );
            ctx.request_repaint();
        }
        None => {
            painter.rect_filled(screen, 0.0, dim);
        }
    }

    // Bubble next to the target.
    let bw = 360.0;
    let bh = 200.0;
    let pos = match target {
        None => screen.center() - vec2(bw, bh) / 2.0,
        Some(r) => {
            let candidates = [
                egui::pos2(r.max.x + 16.0, r.min.y),
                egui::pos2(r.min.x - bw - 16.0, r.min.y),
                egui::pos2(r.min.x, r.max.y + 16.0),
                egui::pos2(r.min.x, r.min.y - bh - 16.0),
                egui::pos2(r.center().x - bw / 2.0, r.center().y - bh / 2.0),
            ];
            let fits = |p: &egui::Pos2| screen.contains_rect(Rect::from_min_size(*p, vec2(bw, bh)));
            let p = candidates
                .iter()
                .copied()
                .find(fits)
                .unwrap_or(candidates[4]);
            egui::pos2(
                p.x.clamp(screen.min.x + 8.0, screen.max.x - bw - 8.0),
                p.y.clamp(screen.min.y + 8.0, screen.max.y - bh - 8.0),
            )
        }
    };
    let mut next = Some(step);
    egui::Area::new(Id::unique("tutorial_bubble"))
        .order(Order::Tooltip)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgb(0x20, 0x22, 0x26))
                .stroke(Stroke::new(1.5, theme::ACCENT))
                .corner_radius(12)
                .inner_margin(18)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 30,
                    spread: 0,
                    color: Color32::from_black_alpha(160),
                })
                .show(ui, |ui| {
                    ui.set_width(bw - 36.0);
                    ui.label(
                        RichText::new(format!(
                            "{} {}/{}",
                            t("Schritt", "Step"),
                            step + 1,
                            TUTORIAL.len()
                        ))
                        .size(11.0)
                        .color(theme::ACCENT),
                    );
                    ui.label(RichText::new(crate::i18n::tl(s.title)).size(18.0).strong());
                    ui.add_space(4.0);
                    ui.label(RichText::new(crate::i18n::tl(s.text)).color(theme::TEXT));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let last = step + 1 == TUTORIAL.len();
                        if ui
                            .add(primary(if last {
                                t("Los geht's!", "Let's go!")
                            } else {
                                t("Weiter", "Next")
                            }))
                            .clicked()
                        {
                            next = if last { None } else { Some(step + 1) };
                        }
                        if step > 0 && ui.add(secondary(t("Zurück", "Back"))).clicked() {
                            next = Some(step - 1);
                        }
                        if !last
                            && ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(t("Beenden", "End")).color(theme::TEXT_WEAK),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                        {
                            next = None;
                        }
                    });
                });
        });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        next = None;
    }
    app.tutorial = next;
}
