//! Sidebar panel "Settings": values that end up in the plugin's config.yml,
//! so server owners can change texts and numbers without rebuilding.

use crate::app::App;
use crate::i18n::t;
use crate::model::{ConfigEntry, ConfigType, Platform};
use crate::theme;
use crate::ui::icons::{self, Icon};
use crate::ui::widgets::{self, section_header};
use egui::{Color32, RichText, Stroke, Ui};

fn clean_key(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c == ' ' { '-' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

pub fn sidebar(app: &mut App, ui: &mut Ui) {
    section_header(ui, t("Einstellungen", "Settings"));
    ui.add_space(6.0);
    let file = match app.project.settings.platform {
        Platform::Paper => format!(
            "plugins/{}/config.yml",
            app.project.settings.name.replace(' ', "_")
        ),
        Platform::Fabric => format!("config/{}.properties", app.project.settings.id),
    };
    ui.label(
        RichText::new(t(
            "Lege hier Werte an, die später ohne neuen Build geändert werden können – z. B. Texte, Preise oder Wartezeiten. Mit dem Block „Einstellung lesen“ benutzt du sie.",
            "Create values here that can be changed later without a new build – e.g. texts, prices or cooldowns. Use them with the block “Read setting”.",
        ))
        .size(11.5)
        .color(theme::TEXT_WEAK),
    );
    ui.label(
        RichText::new(format!(
            "{} {file}",
            t("Datei auf dem Server:", "File on the server:")
        ))
        .size(11.0)
        .color(theme::TEXT_WEAK),
    );
    ui.add_space(10.0);

    let mut remove = None;
    let mut renamed: Option<(String, String)> = None;
    let n = app.project.config.len();
    for i in 0..n {
        let c = &mut app.project.config[i];
        egui::Frame::new()
            .fill(Color32::from_rgb(0x28, 0x2A, 0x2F))
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(8)
            .inner_margin(10)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let mut key = c.key.clone();
                    let w = ui.available_width() - 34.0;
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut key)
                                .desired_width(w)
                                .font(egui::FontId::monospace(13.0))
                                .hint_text("name"),
                        )
                        .changed()
                    {
                        let key = clean_key(&key);
                        if key != c.key {
                            renamed = Some((c.key.clone(), key.clone()));
                            c.key = key;
                        }
                    }
                    if icons::icon_button(ui, Icon::Trash, 24.0, t("Löschen", "Delete")).clicked()
                    {
                        remove = Some(i);
                    }
                });
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt(("cfg_ty", i))
                        .width(92.0)
                        .selected_text(match c.ty {
                            ConfigType::Text => t("Text", "Text"),
                            ConfigType::Number => t("Zahl", "Number"),
                            ConfigType::Bool => t("Ja/Nein", "Yes/No"),
                        })
                        .show_ui(ui, |ui| {
                            for (ty, l) in [
                                (ConfigType::Text, t("Text", "Text")),
                                (ConfigType::Number, t("Zahl", "Number")),
                                (ConfigType::Bool, t("Ja/Nein", "Yes/No")),
                            ] {
                                if ui.selectable_label(c.ty == ty, l).clicked() && c.ty != ty {
                                    c.ty = ty;
                                    c.value = match ty {
                                        ConfigType::Text => c.value.clone(),
                                        ConfigType::Number => "0".into(),
                                        ConfigType::Bool => "true".into(),
                                    };
                                }
                            }
                        });
                    match c.ty {
                        ConfigType::Text => {
                            ui.add(
                                egui::TextEdit::singleline(&mut c.value)
                                    .desired_width(f32::INFINITY)
                                    .hint_text(t("Standardwert", "Default value")),
                            );
                        }
                        ConfigType::Number => {
                            widgets::number_edit(
                                ui,
                                &mut c.value,
                                ui.available_width(),
                                ("cfg_num", i),
                            );
                        }
                        ConfigType::Bool => {
                            let mut b = c.value == "true";
                            if widgets::toggle(ui, &mut b).changed() {
                                c.value = if b { "true".into() } else { "false".into() };
                            }
                        }
                    }
                });
                ui.add(
                    egui::TextEdit::singleline(&mut c.description)
                        .desired_width(f32::INFINITY)
                        .hint_text(t(
                            "Erklärung (steht als Kommentar in der Datei)",
                            "Explanation (written as a comment into the file)",
                        )),
                );
            });
        ui.add_space(6.0);
    }
    if let Some(i) = remove {
        app.project.config.remove(i);
    }
    // Keep "Read setting" blocks pointing at a renamed setting.
    if let Some((old, new)) = renamed {
        if !old.is_empty() {
            for ws in &mut app.project.workspaces {
                for node in ws.nodes.iter_mut().filter(|n| n.kind == "config_get") {
                    if node.values.get("key") == Some(&old) {
                        node.values.insert("key".into(), new.clone());
                    }
                }
            }
        }
    }
    if icons::button(
        ui,
        Some(Icon::Plus),
        t("Einstellung hinzufügen", "Add setting"),
    )
    .clicked()
    {
        let mut k = 1;
        while app
            .project
            .config
            .iter()
            .any(|c| c.key == format!("{}{k}", t("einstellung", "setting")))
        {
            k += 1;
        }
        app.project.config.push(ConfigEntry {
            key: format!("{}{k}", t("einstellung", "setting")),
            ty: ConfigType::Text,
            value: String::new(),
            description: String::new(),
        });
    }
    let dup =
        app.project.config.iter().enumerate().any(|(i, c)| {
            c.key.is_empty() || app.project.config[..i].iter().any(|o| o.key == c.key)
        });
    if dup {
        ui.add_space(6.0);
        ui.label(
            RichText::new(t(
                "Jede Einstellung braucht einen eigenen Namen.",
                "Every setting needs its own name.",
            ))
            .size(11.5)
            .color(theme::YELLOW),
        );
    }
    ui.add_space(14.0);
    ui.label(
        RichText::new(t(
            "Tipp: Mit „Einstellungen neu laden“ (z. B. in einem /reload-Befehl) werden Änderungen an der Datei ohne Neustart übernommen.",
            "Tip: with “Reload settings” (e.g. in a /reload command) changes to the file apply without a restart.",
        ))
        .size(11.0)
        .color(theme::TEXT_WEAK),
    );
}
