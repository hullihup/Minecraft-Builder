//! The settings dialog (gear at the bottom left): general, editor, appearance and
//! the game servers the built jar is uploaded to.

use crate::app::{App, Dialog};
use crate::i18n::t;
use crate::server::{self, AfterUpload, RemoteServer, ServerConfig};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets::{section_header, toggle};
use egui::{Color32, RichText, Sense, Stroke, StrokeKind, Ui, vec2};
use std::sync::mpsc::Receiver;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    General,
    Editor,
    Appearance,
    Servers,
    About,
}

pub struct Connected {
    pub user: String,
    pub servers: Vec<RemoteServer>,
}

/// State of the "add server" steps.
#[derive(Default)]
pub struct ServerUi {
    /// Server being added or edited.
    pub draft: Option<ServerConfig>,
    pub address: String,
    /// The panel page was opened: watch the clipboard for the new key.
    pub waiting_for_key: bool,
    pub last_clip_check: f64,
    pub rx: Option<Receiver<Result<Connected, String>>>,
    pub connected: Option<Connected>,
    pub error: Option<String>,
    pub key_from_clipboard: bool,
}

impl ServerUi {
    fn reset(&mut self) {
        *self = ServerUi::default();
    }
    fn busy(&self) -> bool {
        self.rx.is_some()
    }
}

/// Opens the settings on a tab.
pub fn open(app: &mut App, tab: Tab) {
    app.settings_tab = tab;
    app.dialog = Dialog::Settings;
}

fn connect(app: &mut App, ctx: &egui::Context) {
    let Some(d) = app.server_ui.draft.as_ref() else {
        return;
    };
    let (url, key) = (d.url.clone(), d.api_key.trim().to_string());
    let (tx, rx) = std::sync::mpsc::channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let r = server::check_account(&url, &key).and_then(|user| {
            server::list_servers(&url, &key).map(|servers| Connected { user, servers })
        });
        let _ = tx.send(r);
        ctx.request_repaint();
    });
    app.server_ui.rx = Some(rx);
    app.server_ui.error = None;
    app.server_ui.connected = None;
}

/// Background work of the settings: connection results and the clipboard watcher.
pub fn poll(app: &mut App, ctx: &egui::Context) {
    if let Some(rx) = &app.server_ui.rx {
        if let Ok(r) = rx.try_recv() {
            app.server_ui.rx = None;
            match r {
                Ok(c) => {
                    if let Some(d) = app.server_ui.draft.as_mut() {
                        if c.servers.len() == 1 && d.server_uuid.is_empty() {
                            d.server_uuid = c.servers[0].uuid.clone();
                            d.server_name = c.servers[0].name.clone();
                        }
                    }
                    app.server_ui.connected = Some(c);
                    app.server_ui.waiting_for_key = false;
                }
                Err(e) => app.server_ui.error = Some(e),
            }
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }
    }
    if app.server_ui.waiting_for_key && app.dialog == Dialog::Settings {
        let now = ctx.input(|i| i.time);
        if now - app.server_ui.last_clip_check > 0.6 {
            app.server_ui.last_clip_check = now;
            let found = arboard::Clipboard::new()
                .ok()
                .and_then(|mut c| c.get_text().ok())
                .and_then(|text| server::find_key(&text));
            if let Some(key) = found {
                let changed = app
                    .server_ui
                    .draft
                    .as_mut()
                    .map(|d| {
                        let changed = d.api_key != key;
                        d.api_key = key;
                        changed
                    })
                    .unwrap_or(false);
                if changed {
                    app.server_ui.key_from_clipboard = true;
                    connect(app, ctx);
                }
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(300));
    }
}

fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).color(Color32::BLACK).strong())
        .fill(theme::ACCENT)
        .corner_radius(6)
        .min_size(vec2(110.0, 30.0))
}

fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.5).color(theme::TEXT_WEAK));
}

/// A switch with title and explanation.
fn option(ui: &mut Ui, on: &mut bool, title: &str, sub: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed = toggle(ui, on).changed();
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(RichText::new(title).size(13.5));
            if !sub.is_empty() {
                ui.label(RichText::new(sub).size(11.0).color(theme::TEXT_WEAK));
            }
        });
    });
    ui.add_space(8.0);
    changed
}

fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(0x26, 0x28, 0x2D))
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(8)
        .inner_margin(12)
}

pub fn show(app: &mut App, ctx: &egui::Context, close: &mut bool) {
    let modal = egui::Modal::new(egui::Id::unique("dlg_settings"))
        .frame(
            egui::Frame::new()
                .fill(Color32::from_rgb(0x20, 0x22, 0x26))
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(12)
                .inner_margin(0),
        )
        .show(ctx, |ui| {
            ui.set_width(860.0);
            ui.set_height(560.0);
            ui.horizontal_top(|ui| {
                // Tabs on the left.
                egui::Frame::new()
                    .fill(theme::BG_PANEL)
                    .corner_radius(egui::CornerRadius {
                        nw: 12,
                        sw: 12,
                        ne: 0,
                        se: 0,
                    })
                    .inner_margin(egui::Margin::symmetric(8, 16))
                    .show(ui, |ui| {
                      ui.vertical(|ui| {
                        ui.set_width(196.0);
                        ui.set_height(528.0);
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(t("Einstellungen", "Settings"))
                                    .size(17.0)
                                    .strong(),
                            );
                        });
                        ui.add_space(10.0);
                        let tabs = [
                            (Tab::General, Icon::Sliders, t("Allgemein", "General")),
                            (Tab::Editor, Icon::Puzzle, t("Editor & Blöcke", "Editor & blocks")),
                            (Tab::Appearance, Icon::Image, t("Darstellung", "Appearance")),
                            (Tab::Servers, Icon::Cloud, t("Server", "Servers")),
                            (Tab::About, Icon::Info, t("Über & Updates", "About & updates")),
                        ];
                        for (tab, icon, label) in tabs {
                            let on = app.settings_tab == tab;
                            let (r, resp) =
                                ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
                            if on {
                                ui.painter().rect_filled(r, 6.0, theme::ACCENT_BG);
                            } else if resp.hovered() {
                                ui.painter()
                                    .rect_filled(r, 6.0, Color32::from_rgb(0x2A, 0x2C, 0x31));
                            }
                            icons::paint(
                                ui.painter(),
                                egui::Rect::from_center_size(
                                    r.left_center() + vec2(18.0, 0.0),
                                    vec2(16.0, 16.0),
                                ),
                                icon,
                                if on { theme::ACCENT } else { theme::TEXT_WEAK },
                            );
                            ui.painter().text(
                                r.left_center() + vec2(36.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                label,
                                egui::FontId::proportional(13.5),
                                theme::TEXT,
                            );
                            if resp.clicked() {
                                app.settings_tab = tab;
                            }
                        }
                      });
                    });
                egui::Frame::new().inner_margin(20).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(470.0)
                            .show(ui, |ui| match app.settings_tab {
                                Tab::General => general(app, ui),
                                Tab::Editor => editor(app, ui),
                                Tab::Appearance => appearance(app, ui),
                                Tab::Servers => servers(app, ui),
                                Tab::About => about(app, ui),
                            });
                        ui.add_space(8.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(primary(t("Fertig", "Done"))).clicked() {
                                *close = true;
                            }
                        });
                    });
                });
            });
        });
    if modal.should_close() {
        *close = true;
    }
    if *close {
        app.settings.save();
        app.server_ui.waiting_for_key = false;
    }
}

fn general(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    section_header(ui, t("Sprache", "Language"));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for (de, label) in [(true, "Deutsch"), (false, "English")] {
            if ui
                .selectable_label(crate::i18n::is_german() == de, label)
                .clicked()
            {
                crate::i18n::set_german(de);
                app.settings.german = de;
            }
        }
    });
    ui.add_space(14.0);
    section_header(ui, t("Größe der Oberfläche", "Interface size"));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for (scale, label) in [(0.85, "85 %"), (1.0, "100 %"), (1.15, "115 %"), (1.3, "130 %"), (1.5, "150 %")] {
            let on = (app.settings.ui_scale - scale).abs() < 0.01
                || (app.settings.ui_scale <= 0.5 && scale == 1.0);
            if ui.selectable_label(on, label).clicked() {
                app.settings.ui_scale = scale;
                ctx.set_zoom_factor(scale);
            }
        }
    });
    ui.add_space(14.0);
    section_header(ui, t("Bauen", "Building"));
    ui.add_space(6.0);
    option(
        ui,
        &mut app.settings.autosave_on_build,
        t("Vor dem Bauen automatisch speichern", "Save automatically before building"),
        t(
            "Aus: Bei ungespeicherten Änderungen wirst du vor dem Build gefragt, wohin gespeichert wird.",
            "Off: with unsaved changes you are asked where to save before the build.",
        ),
    );
    section_header(ui, t("Java & Gradle", "Java & Gradle"));
    ui.add_space(4.0);
    crate::ui::home::tool_status(app, ui, &ctx);
    ui.add_space(14.0);
    section_header(ui, t("Hilfe", "Help"));
    ui.add_space(4.0);
    if ui
        .add_enabled(
            app.screen == crate::app::Screen::Editor,
            egui::Button::new(t("Tutorial noch einmal zeigen", "Show the tutorial again")),
        )
        .on_disabled_hover_text(t("Öffne zuerst ein Projekt.", "Open a project first."))
        .clicked()
    {
        app.dialog = Dialog::None;
        crate::ui::onboarding::start_tutorial(app);
    }
}

fn editor(app: &mut App, ui: &mut Ui) {
    let s = &mut app.settings;
    section_header(ui, t("Blöcke", "Blocks"));
    ui.add_space(6.0);
    option(
        ui,
        &mut s.block_descriptions,
        t("Beschreibung auf jedem Block", "Description on every block"),
        t(
            "Zeigt unter dem Titel in einem Satz, was der Block macht.",
            "Shows in one sentence below the title what the block does.",
        ),
    );
    option(
        ui,
        &mut s.pin_types,
        t("Typ an den Ausgängen anzeigen", "Show the type at outputs"),
        t(
            "Kleine farbige Marke wie „Spieler“ oder „Zahl“ neben jedem Ausgang.",
            "Small colored tag like “Player” or “Number” next to every output.",
        ),
    );
    option(
        ui,
        &mut s.block_ids,
        t("Block-Nummern anzeigen (#12)", "Show block numbers (#12)"),
        t(
            "Hilft, wenn eine Fehlermeldung eine Nummer nennt.",
            "Helps when an error message mentions a number.",
        ),
    );
    section_header(ui, t("Fläche", "Canvas"));
    ui.add_space(6.0);
    option(
        ui,
        &mut s.snap,
        t("Am Raster ausrichten", "Snap to grid"),
        t("Blöcke rasten beim Verschieben ein.", "Blocks snap into place when moved."),
    );
    option(
        ui,
        &mut s.wire_flow,
        t("Laufende Punkte auf Ablauf-Kabeln", "Moving dots on flow wires"),
        t(
            "Zeigt die Richtung des Ablaufs, wenn du mit der Maus über einen Block gehst.",
            "Shows the direction of the flow when you hover a block.",
        ),
    );
    option(
        ui,
        &mut s.left_drag_selects,
        t("Linksziehen wählt aus", "Left drag selects"),
        t(
            "Ziehen auf leerer Fläche zieht einen Auswahlrahmen (verschieben dann mit der mittleren Maustaste).",
            "Dragging on empty space draws a selection box (pan with the middle mouse button then).",
        ),
    );
}

fn appearance(app: &mut App, ui: &mut Ui) {
    section_header(ui, t("Item-Bilder", "Item pictures"));
    ui.add_space(6.0);
    hint(
        ui,
        t(
            "Item-Suche, Menü-Editor und Block-Vorschau zeigen Items wie im Inventar: Blöcke als 3D-Würfel, Items flach. Die Texturen und Modelle kommen aus deinem Resourcepack und aus einem Minecraft-Texturen-Pack (Vanilla-Template .zip) oder der Minecraft-.jar auf diesem Computer. Der Builder selbst liefert keine Minecraft-Dateien mit.",
            "The item search, menu editor and block previews show items like the inventory: blocks as 3D cubes, items flat. Textures and models come from your resource pack and from a Minecraft texture pack (vanilla template .zip) or the Minecraft .jar on this computer. The builder itself ships no Minecraft files.",
        ),
    );
    ui.add_space(8.0);
    if option(
        ui,
        &mut app.settings.vanilla_textures,
        t("Minecraft-Texturen benutzen", "Use Minecraft textures"),
        t("Aus: Platzhalter-Bilder statt Texturen.", "Off: stand-in pictures instead of textures."),
    ) {
        app.reset_item_pictures();
    }
    let custom = !app.settings.texture_source.trim().is_empty();
    let found = app.settings.vanilla_source();
    egui::Frame::new()
        .fill(Color32::from_rgb(0x26, 0x28, 0x2D))
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(8)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                match &found {
                    Some(p) => {
                        ui.label(RichText::new("✔").color(theme::GREEN));
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(
                                RichText::new(
                                    p.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                )
                                .strong(),
                            );
                            ui.label(
                                RichText::new(if custom {
                                    t("Ausgewähltes Texturen-Pack", "Selected texture pack")
                                } else {
                                    t("Installiertes Minecraft (automatisch gefunden)", "Installed Minecraft (found automatically)")
                                })
                                .size(11.0)
                                .color(theme::TEXT_WEAK),
                            );
                        });
                    }
                    None => {
                        ui.label(RichText::new("⚠").color(theme::YELLOW));
                        ui.label(
                            RichText::new(if custom {
                                t("Das ausgewählte Texturen-Pack gibt es nicht mehr.", "The selected texture pack no longer exists.")
                            } else {
                                t("Keine Minecraft-Texturen – es werden Platzhalter gezeigt.", "No Minecraft textures – stand-ins are shown.")
                            })
                            .size(12.0),
                        );
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if icons::styled_button(
                    ui,
                    Some(Icon::Folder),
                    t("Texturen-Pack wählen…", "Pick texture pack…"),
                    Some((theme::ACCENT, Color32::BLACK)),
                )
                .clicked()
                {
                    app.pick_texture_source();
                }
                if custom
                    && ui
                        .button(t("Automatisch suchen", "Find automatically"))
                        .on_hover_text(t(
                            "Statt des Packs die Minecraft-.jar aus .minecraft/versions benutzen",
                            "Use the Minecraft .jar from .minecraft/versions instead of the pack",
                        ))
                        .clicked()
                {
                    app.settings.texture_source.clear();
                    app.reset_item_pictures();
                }
            });
        });
    ui.add_space(6.0);
    hint(
        ui,
        t(
            "Geht: ein Vanilla-Template-Resourcepack (.zip, z. B. „26.3.x Template“) oder die Datei .minecraft/versions/<Version>/<Version>.jar. Eine .zip wird in den Einstellungsordner des Builders kopiert.",
            "Works: a vanilla template resource pack (.zip, e.g. “26.3.x Template”) or the file .minecraft/versions/<version>/<version>.jar. A .zip is copied into the builder's settings folder.",
        ),
    );
}

fn about(app: &mut App, ui: &mut Ui) {
    ui.label(
        RichText::new("Minecraft Builder")
            .size(20.0)
            .strong()
            .color(theme::GREEN),
    );
    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
    ui.add_space(6.0);
    ui.label(
        RichText::new(format!(
            "{} {}",
            crate::blocks::all().len(),
            t("Blöcke", "blocks")
        ))
        .color(theme::TEXT_WEAK),
    );
    ui.add_space(12.0);
    section_header(ui, t("Updates", "Updates"));
    ui.add_space(6.0);
    option(
        ui,
        &mut app.settings.auto_update,
        t("Updates automatisch installieren", "Install updates automatically"),
        t(
            "Lädt neue Versionen im Hintergrund und installiert sie beim Beenden.",
            "Downloads new versions in the background and installs them on exit.",
        ),
    );
    if ui.button(t("Nach Updates suchen…", "Check for updates…")).clicked() {
        let ctx = ui.ctx().clone();
        app.settings.save();
        app.dialog = Dialog::Updates;
        app.check_updates(&ctx);
    }
    ui.add_space(14.0);
    hint(
        ui,
        t(
            "Nicht offiziell von Mojang oder Microsoft. Die Item-Liste stammt aus dem offenen Projekt minecraft-data.",
            "Not affiliated with Mojang or Microsoft. The item list comes from the open minecraft-data project.",
        ),
    );
}

// ------------------------------------------------------------------ servers

fn servers(app: &mut App, ui: &mut Ui) {
    if app.server_ui.draft.is_some() {
        server_form(app, ui);
        return;
    }
    section_header(ui, t("Server", "Servers"));
    ui.add_space(4.0);
    hint(
        ui,
        t(
            "Verbinde deinen Server, dann landet die .jar nach jedem Build automatisch im Ordner plugins/ (Paper) bzw. mods/ (Fabric). Unterstützt wird zurzeit das Calagopus-Panel.",
            "Connect your server and the jar ends up in plugins/ (Paper) or mods/ (Fabric) automatically after every build. Currently the Calagopus panel is supported.",
        ),
    );
    ui.add_space(10.0);
    let mut edit = None;
    let mut remove = None;
    for s in &app.settings.servers {
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                icons::icon(ui, Icon::Server, 22.0, if s.ready() { theme::GREEN } else { theme::YELLOW });
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.label(RichText::new(s.display_name()).strong());
                    ui.label(
                        RichText::new(format!(
                            "Calagopus · {}{}",
                            server::host_of(&s.url),
                            if s.server_name.is_empty() {
                                String::new()
                            } else {
                                format!(" · {}", s.server_name)
                            }
                        ))
                        .size(11.5)
                        .color(theme::TEXT_WEAK),
                    );
                    if !s.ready() {
                        ui.label(
                            RichText::new(t("Noch nicht fertig eingerichtet", "Not set up completely yet"))
                                .size(11.0)
                                .color(theme::YELLOW),
                        );
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::icon_button(ui, Icon::Trash, 26.0, t("Entfernen", "Remove")).clicked() {
                        remove = Some(s.id);
                    }
                    if icons::icon_button(ui, Icon::Edit, 26.0, t("Bearbeiten", "Edit")).clicked() {
                        edit = Some(s.clone());
                    }
                });
            });
        });
        ui.add_space(6.0);
    }
    if app.settings.servers.is_empty() {
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(t("Noch kein Server verbunden.", "No server connected yet."))
                    .color(theme::TEXT_WEAK),
            );
        });
        ui.add_space(6.0);
    }
    if icons::styled_button(
        ui,
        Some(Icon::Plus),
        t("Server hinzufügen", "Add server"),
        Some((theme::ACCENT, Color32::BLACK)),
    )
    .clicked()
    {
        app.server_ui.reset();
        app.server_ui.draft = Some(ServerConfig::default());
    }
    if let Some(s) = edit {
        app.server_ui.reset();
        app.server_ui.address = s.url.clone();
        app.server_ui.draft = Some(s);
        let ctx = ui.ctx().clone();
        connect(app, &ctx);
    }
    if let Some(id) = remove {
        app.settings.servers.retain(|s| s.id != id);
        if app.project.settings.deploy_server == id {
            app.project.settings.deploy_server = 0;
        }
        app.settings.save();
    }
    ui.add_space(12.0);
    hint(
        ui,
        t(
            "Welcher Server benutzt wird, stellst du pro Projekt unter „Bauen & Export“ ein. Der API-Key wird nur auf diesem Computer in den Einstellungen des Builders gespeichert.",
            "Which server is used is set per project under “Build & export”. The API key is only stored on this computer in the builder's settings.",
        ),
    );
}

fn step(ui: &mut Ui, n: usize, title: &str, done: bool, active: bool) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
        let col = if done {
            theme::GREEN
        } else if active {
            theme::ACCENT
        } else {
            Color32::from_rgb(0x4A, 0x4D, 0x55)
        };
        ui.painter().circle_filled(r.center(), 11.0, col);
        if done {
            icons::paint(ui.painter(), r.shrink(5.0), Icon::Check, Color32::BLACK);
        } else {
            ui.painter().text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                n.to_string(),
                egui::FontId::proportional(12.5),
                Color32::BLACK,
            );
        }
        ui.label(RichText::new(title).size(14.5).strong().color(if active || done {
            theme::TEXT
        } else {
            theme::TEXT_WEAK
        }));
    });
}

fn server_form(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let editing = app
        .server_ui
        .draft
        .as_ref()
        .map(|d| d.id != 0)
        .unwrap_or(false);
    ui.horizontal(|ui| {
        if ui.button(t("← Zurück", "← Back")).clicked() {
            app.server_ui.reset();
        }
        ui.label(
            RichText::new(if editing {
                t("Server bearbeiten", "Edit server")
            } else {
                t("Server hinzufügen · Calagopus-Panel", "Add server · Calagopus panel")
            })
            .size(16.0)
            .strong(),
        );
    });
    if app.server_ui.draft.is_none() {
        return;
    }
    ui.add_space(10.0);

    let has_url = app
        .server_ui
        .draft
        .as_ref()
        .map(|d| !d.url.is_empty())
        .unwrap_or(false);
    let has_key = app
        .server_ui
        .draft
        .as_ref()
        .map(|d| server::looks_like_key(&d.api_key))
        .unwrap_or(false);
    let connected = app.server_ui.connected.is_some();

    // ---- 1. address
    step(ui, 1, t("Adresse deines Panels", "Address of your panel"), has_url, !has_url);
    ui.add_space(4.0);
    let mut open_page = false;
    ui.horizontal(|ui| {
        ui.add_space(30.0);
        let te = ui.add(
            egui::TextEdit::singleline(&mut app.server_ui.address)
                .hint_text("panel.example.com")
                .desired_width(330.0),
        );
        let enter = te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let ok = !server::normalize_url(&app.server_ui.address).is_empty();
        if ui
            .add_enabled(ok, primary(t("API-Key-Seite öffnen", "Open API key page")))
            .clicked()
            || (enter && ok)
        {
            open_page = true;
        }
    });
    ui.horizontal(|ui| {
        ui.add_space(30.0);
        hint(
            ui,
            t(
                "Die Adresse, unter der du dein Panel im Browser öffnest.",
                "The address you open your panel at in the browser.",
            ),
        );
    });
    if open_page {
        let url = server::normalize_url(&app.server_ui.address);
        if let Some(d) = app.server_ui.draft.as_mut() {
            if d.url != url {
                d.url = url.clone();
                d.server_uuid.clear();
                d.server_name.clear();
            }
        }
        app.server_ui.address = url.clone();
        app.server_ui.waiting_for_key = true;
        app.server_ui.connected = None;
        app.server_ui.error = None;
        let _ = open::that_detached(server::api_key_page(&url));
    }
    ui.add_space(12.0);

    // ---- 2. key
    step(ui, 2, t("API-Key erstellen", "Create the API key"), has_key && connected, has_url && !connected);
    ui.add_space(4.0);
    if has_url {
        ui.horizontal(|ui| {
            ui.add_space(30.0);
            ui.vertical(|ui| {
                if app.server_ui.waiting_for_key && !has_key {
                    card().show(ui, |ui| {
                        ui.set_width(ui.available_width() - 8.0);
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new());
                            ui.label(RichText::new(t(
                                "Dein Browser zeigt jetzt die Seite „API-Key erstellen“.",
                                "Your browser now shows the “Create API key” page.",
                            )).strong());
                        });
                        ui.label(t(
                            "Gib dem Key einen Namen (z. B. „Minecraft Builder“), erlaube Dateien und Server-Steuerung, klicke auf Erstellen und kopiere den Key. Er wird hier automatisch übernommen.",
                            "Give the key a name (e.g. “Minecraft Builder”), allow files and server control, click Create and copy the key. It is picked up here automatically.",
                        ));
                    });
                    ui.add_space(4.0);
                }
                ui.horizontal(|ui| {
                    if let Some(d) = app.server_ui.draft.as_mut() {
                        let before = d.api_key.clone();
                        ui.add(
                            egui::TextEdit::singleline(&mut d.api_key)
                                .password(true)
                                .hint_text("c7sp_…")
                                .desired_width(330.0),
                        );
                        if before != d.api_key {
                            app.server_ui.connected = None;
                            app.server_ui.key_from_clipboard = false;
                        }
                    }
                    let busy = app.server_ui.busy();
                    if ui
                        .add_enabled(has_key && !busy, egui::Button::new(t("Verbinden", "Connect")))
                        .clicked()
                    {
                        connect(app, &ctx);
                    }
                    if busy {
                        ui.add(egui::Spinner::new());
                    }
                });
                if app.server_ui.key_from_clipboard && has_key {
                    ui.label(
                        RichText::new(t(
                            "✔ Key aus der Zwischenablage übernommen.",
                            "✔ Key picked up from the clipboard.",
                        ))
                        .size(11.5)
                        .color(theme::GREEN),
                    );
                } else if !has_key
                    && app
                        .server_ui
                        .draft
                        .as_ref()
                        .map(|d| !d.api_key.trim().is_empty())
                        .unwrap_or(false)
                {
                    ui.label(
                        RichText::new(t(
                            "Das sieht nicht nach einem Calagopus-Key aus (c7sp_ + 43 Zeichen).",
                            "This does not look like a Calagopus key (c7sp_ + 43 characters).",
                        ))
                        .size(11.5)
                        .color(theme::YELLOW),
                    );
                }
                if let Some(e) = &app.server_ui.error {
                    ui.label(RichText::new(e).size(12.0).color(theme::RED));
                }
                if let Some(c) = &app.server_ui.connected {
                    ui.label(
                        RichText::new(format!(
                            "✔ {} {}",
                            t("Verbunden als", "Connected as"),
                            c.user
                        ))
                        .color(theme::GREEN),
                    );
                }
            });
        });
    }
    ui.add_space(12.0);

    // ---- 3. server
    step(ui, 3, t("Server wählen", "Pick the server"), false, connected);
    ui.add_space(4.0);
    let mut save = false;
    if connected {
        let list: Vec<RemoteServer> = app
            .server_ui
            .connected
            .as_ref()
            .map(|c| c.servers.clone())
            .unwrap_or_default();
        ui.horizontal(|ui| {
            ui.add_space(30.0);
            ui.vertical(|ui| {
                if list.is_empty() {
                    ui.label(
                        RichText::new(t(
                            "Dieser Key sieht keine Server. Prüfe die Rechte des Keys.",
                            "This key sees no servers. Check the key's permissions.",
                        ))
                        .color(theme::YELLOW),
                    );
                }
                let Some(d) = app.server_ui.draft.as_mut() else {
                    return;
                };
                for s in &list {
                    let on = d.server_uuid == s.uuid;
                    let (r, resp) =
                        ui.allocate_exact_size(vec2(ui.available_width() - 8.0, 40.0), Sense::click());
                    ui.painter().rect(
                        r,
                        6.0,
                        if on {
                            theme::ACCENT_BG
                        } else if resp.hovered() {
                            Color32::from_rgb(0x2E, 0x30, 0x36)
                        } else {
                            Color32::from_rgb(0x26, 0x28, 0x2D)
                        },
                        Stroke::new(if on { 1.5 } else { 1.0 }, if on { theme::ACCENT } else { theme::BORDER }),
                        StrokeKind::Inside,
                    );
                    icons::paint(
                        ui.painter(),
                        egui::Rect::from_center_size(r.left_center() + vec2(20.0, 0.0), vec2(18.0, 18.0)),
                        Icon::Server,
                        if on { theme::ACCENT } else { theme::TEXT_WEAK },
                    );
                    ui.painter().text(
                        r.left_center() + vec2(40.0, -7.0),
                        egui::Align2::LEFT_CENTER,
                        &s.name,
                        egui::FontId::proportional(13.5),
                        theme::TEXT,
                    );
                    ui.painter().text(
                        r.left_center() + vec2(40.0, 9.0),
                        egui::Align2::LEFT_CENTER,
                        if s.node.is_empty() {
                            s.uuid.chars().take(8).collect::<String>()
                        } else {
                            format!("{} · {}", s.node, s.uuid.chars().take(8).collect::<String>())
                        },
                        egui::FontId::proportional(10.5),
                        theme::TEXT_WEAK,
                    );
                    if resp.clicked() {
                        d.server_uuid = s.uuid.clone();
                        d.server_name = s.name.clone();
                    }
                    ui.add_space(4.0);
                }
                ui.add_space(6.0);
                ui.label(RichText::new(t("Nach dem Hochladen", "After uploading")).size(12.0));
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut d.after, AfterUpload::Nothing, t("Nichts", "Nothing"));
                    ui.selectable_value(&mut d.after, AfterUpload::Restart, t("Server neu starten", "Restart the server"));
                    ui.selectable_value(&mut d.after, AfterUpload::Command, t("Befehl ausführen", "Run a command"));
                });
                if d.after == AfterUpload::Command {
                    ui.add(
                        egui::TextEdit::singleline(&mut d.command)
                            .hint_text("reload confirm")
                            .desired_width(330.0),
                    );
                }
                ui.add_space(6.0);
                ui.label(RichText::new(t("Name in der App (optional)", "Name in the app (optional)")).size(12.0));
                ui.add(
                    egui::TextEdit::singleline(&mut d.name)
                        .hint_text(d.server_name.as_str())
                        .desired_width(330.0),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(d.ready(), primary(t("Speichern", "Save")))
                        .clicked()
                    {
                        save = true;
                    }
                });
            });
        });
    }
    ui.add_space(8.0);
    if ui.button(t("Abbrechen", "Cancel")).clicked() {
        app.server_ui.reset();
        return;
    }
    if save {
        if let Some(mut d) = app.server_ui.draft.take() {
            if d.id == 0 {
                d.id = app.settings.next_server_id.max(1);
                app.settings.next_server_id = d.id + 1;
                // The first server becomes the upload target of the open project.
                if app.project.settings.deploy_server == 0 && app.screen == crate::app::Screen::Editor {
                    app.project.settings.deploy_server = d.id;
                }
                app.settings.servers.push(d);
            } else if let Some(s) = app.settings.servers.iter_mut().find(|s| s.id == d.id) {
                *s = d;
            }
            app.settings.save();
        }
        app.server_ui.reset();
    }
}
