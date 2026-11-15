//! Application state, file handling, undo and the top-level layout.

use crate::build::{self, AppSettings, Job, Level, Msg};
use crate::codegen::{self, Problem};
use crate::examples::{self, Starter};
use crate::i18n::t;
use crate::model::{Link, Node, NodeId, Platform, Project, Versions, Workspace};
use crate::theme;
use crate::ui;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

/// A panel of the editor. Every view can be docked left, right or at the bottom.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum View {
    Workspaces,
    Blocks,
    Resources,
    Project,
    Build,
    Plugins,
    Inspector,
    Code,
    Console,
    Problems,
    Server,
}

/// Where a view is docked.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum Dock {
    Left,
    Right,
    Bottom,
}

pub type Activity = View;
pub type RightTab = View;
pub type BottomTab = View;

impl View {
    pub const ALL: [View; 11] = [
        View::Workspaces,
        View::Blocks,
        View::Resources,
        View::Project,
        View::Build,
        View::Plugins,
        View::Inspector,
        View::Code,
        View::Console,
        View::Problems,
        View::Server,
    ];
    pub fn default_dock(self) -> Dock {
        match self {
            View::Inspector | View::Code => Dock::Right,
            View::Console | View::Problems | View::Server => Dock::Bottom,
            _ => Dock::Left,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            View::Workspaces => t("Arbeitsbereiche & Befehle", "Workspaces & commands"),
            View::Blocks => t("Block-Bibliothek", "Block library"),
            View::Resources => t("Texturen & Modelle", "Textures & models"),
            View::Project => t("Projekt & Versionen", "Project & versions"),
            View::Build => t("Bauen & Export", "Build & export"),
            View::Plugins => t("Plugins (MCP)", "Plugins (MCP)"),
            View::Inspector => t("Inspector", "Inspector"),
            View::Code => t("Java-Code", "Java code"),
            View::Console => t("Konsole", "Console"),
            View::Problems => t("Probleme", "Problems"),
            View::Server => t("Server-Konsole", "Server console"),
        }
    }
    pub fn icon(self) -> crate::ui::icons::Icon {
        use crate::ui::icons::Icon as I;
        match self {
            View::Workspaces => I::Layers,
            View::Blocks => I::Puzzle,
            View::Resources => I::Image,
            View::Project => I::Sliders,
            View::Build => I::Package,
            View::Plugins => I::Sparkle,
            View::Inspector => I::Gear,
            View::Code => I::Code,
            View::Console => I::Keyboard,
            View::Problems => I::Warning,
            View::Server => I::Server,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Editor,
}

/// Running Java/Gradle setup.
pub struct SetupState {
    pub rx: Receiver<crate::tools::SetupMsg>,
    pub status: String,
    pub progress: f32,
    pub lines: Vec<String>,
    pub result: Option<Result<(), String>>,
}

/// Cached information about the project's resource pack.
#[derive(Default)]
pub struct PackCache {
    /// What the info was computed for: pack paths, item rules, namespace.
    pub key: Option<(
        Vec<PathBuf>,
        std::collections::BTreeMap<String, crate::model::ItemRule>,
        String,
    )>,
    pub info: Option<Result<crate::resourcepack::PackInfo, String>>,
    pub thumbs: HashMap<String, Option<egui::TextureHandle>>,
}

/// Item pictures (menu editor, item search, block previews), drawn by `item_render`.
#[derive(Default)]
pub struct VanillaCache {
    /// Files the renderer was made for: project packs, then the vanilla source.
    pub key: Option<Vec<PathBuf>>,
    pub renderer: Option<crate::item_render::Renderer>,
    /// The vanilla source in use (template pack or Minecraft .jar), if any.
    pub vanilla: Option<PathBuf>,
    pub thumbs: HashMap<String, Option<egui::TextureHandle>>,
    /// Items of the texture source that the built-in item list does not know: (id, is block).
    pub extra_items: Vec<(String, bool)>,
    /// Pictures made in the current frame (kept small so the UI stays smooth).
    pub loaded_this_frame: usize,
    /// The packs were already checked in this frame.
    pub checked_this_frame: bool,
}

#[derive(Clone)]
pub struct ProjectSummary {
    pub name: String,
    pub platform: Platform,
    pub minecraft: String,
    pub blocks: usize,
    pub commands: usize,
    pub modified: std::time::SystemTime,
}

/// Console of the project's game server, shown in the bottom panel.
#[derive(Default)]
pub struct ServerConsole {
    pub text: String,
    pub error: Option<String>,
    pub rx: Option<Receiver<Result<String, String>>>,
    pub last_poll: f64,
    pub input: String,
    /// Result of the last command / power action.
    pub action_rx: Option<Receiver<Result<String, String>>>,
    pub status: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConsoleFilter {
    All,
    Info,
    Done,
    Warn,
    Error,
}

pub struct LogLine {
    pub time: String,
    pub level: Level,
    pub text: String,
}

/// Pan/zoom of one workspace.
#[derive(Clone, Copy)]
pub struct Viewport {
    pub pan: egui::Vec2,
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            pan: egui::vec2(40.0, 40.0),
            zoom: 1.0,
        }
    }
}

/// Nodes + links copied with Ctrl+C.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Clip {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
}

/// Pending modal dialogs.
#[derive(Clone, PartialEq)]
pub enum Dialog {
    None,
    NewProject {
        name: String,
        platform: Platform,
        starter: Starter,
        folder: String,
    },
    UnsavedChanges {
        then: AfterSave,
    },
    About,
    RenameWorkspace {
        index: usize,
        name: String,
    },
    Message {
        title: String,
        text: String,
    },
    Updates,
    Settings,
    /// Search & replace in all texts of the project.
    Replace {
        find: String,
        with: String,
    },
}

/// State of the updater.
#[derive(Default)]
pub struct UpdateState {
    pub rx: Option<Receiver<crate::update::UpdateMsg>>,
    pub checking: bool,
    pub checked: bool,
    pub release: Option<crate::update::Release>,
    pub progress: f32,
    pub downloading: bool,
    pub installer: Option<PathBuf>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AfterSave {
    Quit,
    NewProject,
    Open,
    Home,
    Update,
}

/// "Add block" popup.
pub struct Palette {
    /// Where the new block goes (scene coordinates).
    pub scene_pos: egui::Pos2,
    /// Where the popup is shown (screen coordinates).
    pub screen_pos: egui::Pos2,
    pub query: String,
    pub selected: usize,
    /// Connect the new block to this pin: (node, pin, is_output, type).
    pub connect: Option<(NodeId, String, bool, crate::blocks::PinType)>,
    pub just_opened: bool,
}

pub struct App {
    pub project: Project,
    pub path: Option<PathBuf>,
    saved: Project,
    undo: Vec<Project>,
    redo: Vec<Project>,
    committed: Project,
    pub active_ws: usize,
    pub views: HashMap<usize, Viewport>,
    pub selection: BTreeSet<NodeId>,
    pub selected_note: Option<NodeId>,
    pub activity: Activity,
    pub show_sidebar: bool,
    pub show_right: bool,
    pub show_bottom: bool,
    pub right_tab: RightTab,
    pub bottom_tab: BottomTab,
    pub console: Vec<LogLine>,
    pub console_filter: ConsoleFilter,
    pub console_search: String,
    pub problems: Vec<Problem>,
    pub code_preview: String,
    /// Highlighted code preview, laid out once: (code, pixels per point, text).
    pub code_galley: Option<(String, f32, std::sync::Arc<egui::Galley>)>,
    gen_for: Option<Project>,
    /// Code generation running in the background: (project it is for, result).
    gen_job: Option<std::sync::mpsc::Receiver<(Project, codegen::Output)>>,
    pub palette: Option<Palette>,
    pub dialog: Dialog,
    pub job: Option<Job>,
    pub job_export_only: bool,
    pub last_jar: Option<PathBuf>,
    pub clipboard: Option<Clip>,
    pub settings: AppSettings,
    pub canvas: ui::canvas::CanvasState,
    pub block_search: String,
    pub version_job: Option<Receiver<Result<Versions, String>>>,
    pub mc_versions: Vec<String>,
    pub mc_job: Option<Receiver<Result<Vec<String>, String>>>,
    /// Focus this node on the next frame (workspace, node).
    pub focus_node: Option<(usize, NodeId)>,
    pub screen: Screen,
    /// First-start wizard step, if shown.
    pub onboarding: Option<usize>,
    /// Tutorial step, if running.
    pub tutorial: Option<usize>,
    pub setup: Option<SetupState>,
    pub pack: PackCache,
    pub show_commands: bool,
    pub update: UpdateState,
    /// Collapsed categories in the block search.
    pub palette_collapsed: std::collections::HashSet<String>,
    pub palette_filter: u8,
    pub cmd_draft: Option<crate::ui::commands::Draft>,
    pub home_search: String,
    pub summaries: HashMap<PathBuf, (std::time::SystemTime, Option<ProjectSummary>)>,
    /// Screen rects of UI parts, for the tutorial highlights.
    pub ui_rects: HashMap<&'static str, egui::Rect>,
    pub vanilla: VanillaCache,
    pub settings_tab: crate::ui::settings::Tab,
    pub server_ui: crate::ui::settings::ServerUi,
    pub menu_edit: Option<crate::ui::menu_editor::MenuEdit>,
    /// Recipe block being edited: (workspace, node).
    pub recipe_edit: Option<(usize, NodeId)>,
    /// "Reason (screen)" block being edited: (workspace, node).
    pub screen_edit: Option<(usize, NodeId)>,
    pub server_console: ServerConsole,
    /// Running MCP server (Plugins panel).
    pub mcp: Option<crate::mcp::Server>,
    pub mcp_error: Option<String>,
    pub mcp_log: Vec<String>,
    /// Result of the last "Add" / "Remove" in the Plugins panel: (ok, text).
    pub mcp_note: Option<(bool, String)>,
    /// State of the AI programs, checked every few seconds: (time, states).
    pub mcp_clients: Option<(f64, Vec<(crate::mcp::Client, crate::mcp::ClientState)>)>,
    /// Test mode run, if shown.
    pub trace: Option<crate::ui::trace::Trace>,
    pub item_picker: Option<crate::ui::menu_editor::ItemPicker>,
    allow_close: bool,
    title: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>) -> Self {
        theme::apply(&cc.egui_ctx);
        let settings = AppSettings::load();
        crate::i18n::set_german(settings.german);
        if settings.ui_scale > 0.5 && settings.ui_scale < 3.0 {
            cc.egui_ctx.set_zoom_factor(settings.ui_scale);
        }
        let project = examples::new_project("MeinPlugin", Platform::Paper, Starter::Welcome);
        let mut app = Self {
            saved: project.clone(),
            committed: project.clone(),
            project,
            path: None,
            undo: vec![],
            redo: vec![],
            active_ws: 0,
            views: HashMap::new(),
            selection: BTreeSet::new(),
            selected_note: None,
            activity: Activity::Workspaces,
            show_sidebar: true,
            show_right: true,
            show_bottom: true,
            right_tab: RightTab::Inspector,
            bottom_tab: BottomTab::Console,
            console: vec![],
            console_filter: ConsoleFilter::All,
            console_search: String::new(),
            problems: vec![],
            code_preview: String::new(),
            code_galley: None,
            gen_for: None,
            gen_job: None,
            palette: None,
            dialog: Dialog::None,
            job: None,
            job_export_only: false,
            last_jar: None,
            clipboard: None,
            settings,
            canvas: Default::default(),
            block_search: String::new(),
            version_job: None,
            mc_versions: vec![],
            mc_job: None,
            focus_node: None,
            screen: Screen::Home,
            onboarding: None,
            tutorial: None,
            setup: None,
            pack: PackCache::default(),
            show_commands: false,
            update: UpdateState::default(),
            palette_collapsed: Default::default(),
            palette_filter: 0,
            cmd_draft: None,
            home_search: String::new(),
            summaries: HashMap::new(),
            ui_rects: HashMap::new(),
            vanilla: VanillaCache::default(),
            settings_tab: crate::ui::settings::Tab::General,
            server_ui: Default::default(),
            menu_edit: None,
            recipe_edit: None,
            trace: None,
            server_console: ServerConsole::default(),
            mcp: None,
            mcp_error: None,
            mcp_log: vec![],
            mcp_note: None,
            mcp_clients: None,
            screen_edit: None,
            item_picker: None,
            allow_close: false,
            title: String::new(),
        };
        app.log(
            Level::Info,
            format!(
                "Minecraft Builder {} – {} Blöcke geladen",
                env!("CARGO_PKG_VERSION"),
                crate::blocks::all().len()
            ),
        );
        app.canvas.fit_request = true;
        // The last update could not be installed: say so instead of silently staying old.
        if let Ok(why) = std::fs::read_to_string(crate::update::failure_file()) {
            let _ = std::fs::remove_file(crate::update::failure_file());
            app.log(Level::Error, format!("Update fehlgeschlagen: {}", why.trim()));
            app.dialog = Dialog::Message {
                title: t("Update nicht installiert", "Update not installed").into(),
                text: format!(
                    "{}\n\n{}",
                    t(
                        "Das letzte Update konnte nicht installiert werden. Du kannst es unter Hilfe → Nach Updates suchen erneut versuchen oder den Installer von GitHub herunterladen und starten.",
                        "The last update could not be installed. Try again under Help → Check for updates, or download and run the installer from GitHub.",
                    ),
                    why.trim()
                ),
            };
        }
        if !app.settings.onboarded {
            app.onboarding = Some(0);
        }
        if app.settings.auto_update && crate::update::is_installed() {
            app.check_updates(&cc.egui_ctx);
        }
        if let Some(p) = open {
            app.open_path(p);
        }
        app
    }

    // ------------------------------------------------------------------ log

    pub fn log(&mut self, level: Level, text: impl Into<String>) {
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        self.console.push(LogLine {
            time,
            level,
            text: text.into(),
        });
        if self.console.len() > 5000 {
            self.console.drain(0..1000);
        }
    }

    // ------------------------------------------------------------ project

    pub fn ws(&self) -> &Workspace {
        &self.project.workspaces[self.active_ws.min(self.project.workspaces.len() - 1)]
    }
    pub fn ws_mut(&mut self) -> &mut Workspace {
        let i = self.active_ws.min(self.project.workspaces.len() - 1);
        &mut self.project.workspaces[i]
    }
    pub fn view(&mut self) -> &mut Viewport {
        self.views.entry(self.active_ws).or_default()
    }

    pub fn dirty(&self) -> bool {
        self.project != self.saved
    }

    pub fn set_project(&mut self, p: Project, path: Option<PathBuf>) {
        self.project = p;
        self.saved = self.project.clone();
        self.committed = self.project.clone();
        self.undo.clear();
        self.redo.clear();
        self.path = path;
        self.active_ws = 0;
        self.views.clear();
        self.selection.clear();
        self.selected_note = None;
        self.gen_for = None;
        self.gen_job = None;
        self.palette = None;
        self.canvas.fit_request = true;
        self.screen = Screen::Editor;
        self.cmd_draft = None;
        self.show_commands = false;
        self.menu_edit = None;
        self.recipe_edit = None;
        self.item_picker = None;
        self.refresh_pack(true);
    }

    /// Goes back to the start screen (asks to save first).
    pub fn go_home(&mut self) {
        if self.dirty() && self.screen == Screen::Editor {
            self.dialog = Dialog::UnsavedChanges {
                then: AfterSave::Home,
            };
            return;
        }
        self.screen = Screen::Home;
        self.summaries.clear();
    }

    /// Reloads the resource pack info if the packs or item rules changed (or `force`).
    pub fn refresh_pack(&mut self, force: bool) {
        let s = &self.project.settings.pack;
        if !s.enabled() {
            if self.pack.info.is_some() || force {
                self.pack = PackCache::default();
                crate::resourcepack::set_known_models(vec![]);
                crate::resourcepack::set_known_sounds(vec![]);
            }
            return;
        }
        let ns = crate::resourcepack::namespace_for(&self.project.settings.id);
        let key = (s.resolve(self.path.as_deref()), s.items.clone(), ns);
        if !force && self.pack.key.as_ref() == Some(&key) && self.pack.info.is_some() {
            return;
        }
        let info = crate::resourcepack::inspect(&key.0, &key.1, &key.2);
        crate::resourcepack::set_known_models(
            info.as_ref()
                .map(|i| i.item_models.clone())
                .unwrap_or_default(),
        );
        crate::resourcepack::set_known_sounds(
            info.as_ref().map(|i| i.sounds.clone()).unwrap_or_default(),
        );
        // Thumbnails stay valid as long as the pack files are the same.
        let same_files = self.pack.key.as_ref().map(|k| &k.0) == Some(&key.0) && !force;
        let thumbs = if same_files {
            std::mem::take(&mut self.pack.thumbs)
        } else {
            HashMap::new()
        };
        self.pack = PackCache {
            key: Some(key),
            info: Some(info),
            thumbs,
        };
    }

    /// Adds a resource pack .zip to the project (copied next to the project file).
    pub fn add_pack(&mut self, src: PathBuf) {
        match crate::resourcepack::inspect(
            std::slice::from_ref(&src),
            &Default::default(),
            "custom",
        ) {
            Err(e) => {
                self.dialog = Dialog::Message {
                    title: t("Resourcepack", "Resource pack").into(),
                    text: e,
                };
            }
            Ok(info) => {
                let mut stored = src.display().to_string();
                if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
                    let file = src
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "pack.zip".into());
                    let rel = format!("resourcepacks/{file}");
                    let target = dir.join(&rel);
                    if target != src {
                        match std::fs::create_dir_all(dir.join("resourcepacks"))
                            .and_then(|_| std::fs::copy(&src, &target))
                        {
                            Ok(_) => stored = rel,
                            Err(e) => self.log(
                                Level::Warn,
                                format!("Konnte das Pack nicht in den Projektordner kopieren: {e}"),
                            ),
                        }
                    } else {
                        stored = rel;
                    }
                }
                let packs = &mut self.project.settings.pack.packs;
                if let Some(p) = packs.iter_mut().find(|p| p.path == stored) {
                    p.enabled = true;
                } else {
                    packs.push(crate::model::PackEntry {
                        path: stored,
                        enabled: true,
                    });
                }
                let count = packs.len();
                self.log(
                    Level::Done,
                    format!(
                        "Resourcepack hinzugefügt: {} Modelle, {} Texturen{}",
                        info.item_models.len(),
                        info.textures.len(),
                        if count > 1 {
                            format!(" – {count} Packs werden zu einem zusammengeführt")
                        } else {
                            String::new()
                        }
                    ),
                );
                self.refresh_pack(true);
            }
        }
    }

    /// Thumbnail of a texture inside the project's packs.
    pub fn pack_thumb(&mut self, ctx: &egui::Context, entry: &str) -> Option<egui::TextureId> {
        if let Some(h) = self.pack.thumbs.get(entry) {
            return h.as_ref().map(|h| h.id());
        }
        if self.vanilla.loaded_this_frame >= 6 {
            ctx.request_repaint();
            return None;
        }
        self.vanilla.loaded_this_frame += 1;
        let handle = match &self.pack.info {
            Some(Ok(info)) => info
                .source
                .get(entry)
                .and_then(|i| info.packs.get(*i))
                .and_then(|p| crate::resourcepack::texture_preview(&p.path, entry))
                .map(|img| {
                    ctx.load_texture(format!("pack:{entry}"), img, egui::TextureOptions::NEAREST)
                }),
            _ => None,
        };
        let id = handle.as_ref().map(|h| h.id());
        self.pack.thumbs.insert(entry.to_string(), handle);
        id
    }

    /// Makes sure the item renderer reads the current packs and vanilla source.
    pub fn ensure_renderer(&mut self) {
        if self.vanilla.checked_this_frame && self.vanilla.key.is_some() {
            return;
        }
        self.vanilla.checked_this_frame = true;
        let vanilla = if self.settings.vanilla_textures {
            self.settings.vanilla_source()
        } else {
            None
        };
        let mut key = self.project.settings.pack.resolve(self.path.as_deref());
        key.extend(vanilla.clone());
        if self.vanilla.key.as_ref() != Some(&key) {
            let r = crate::item_render::Renderer::new(&key);
            self.vanilla.extra_items = r
                .vanilla_items()
                .into_iter()
                .filter(|(id, _)| crate::items::get(id).is_none())
                .collect();
            self.vanilla.renderer = Some(r);
            self.vanilla.key = Some(key);
            self.vanilla.vanilla = vanilla;
            self.vanilla.thumbs.clear();
        }
    }

    /// Forgets all item pictures (after the texture settings changed).
    pub fn reset_item_pictures(&mut self) {
        self.vanilla.key = None;
        self.vanilla.renderer = None;
        self.vanilla.thumbs.clear();
    }

    /// Picture of an item like in the inventory: blocks as 3D cubes, items flat.
    /// A custom model (resource pack) is used if given.
    pub fn item_texture(
        &mut self,
        ctx: &egui::Context,
        item: &str,
        model: &str,
    ) -> Option<egui::TextureId> {
        let item = item.trim();
        let model = model.trim();
        let key = if model.is_empty() {
            item.to_string()
        } else {
            format!("{item}|{model}")
        };
        if key.is_empty() {
            return None;
        }
        self.ensure_renderer();
        if let Some(h) = self.vanilla.thumbs.get(&key) {
            return h.as_ref().map(|h| h.id());
        }
        if self.vanilla.loaded_this_frame >= 16 {
            ctx.request_repaint();
            return None;
        }
        self.vanilla.loaded_this_frame += 1;
        let r = self.vanilla.renderer.as_mut()?;
        let img = (!model.is_empty())
            .then(|| r.render(model))
            .flatten()
            .or_else(|| r.render(item));
        let handle = img.map(|img| {
            ctx.load_texture(format!("item:{key}"), img, egui::TextureOptions::NEAREST)
        });
        let id = handle.as_ref().map(|h| h.id());
        self.vanilla.thumbs.insert(key, handle);
        id
    }

    /// Lets the user pick the vanilla textures (template pack .zip or Minecraft .jar).
    /// A .zip is copied into the app's settings folder so it stays available.
    pub fn pick_texture_source(&mut self) {
        let Some(src) = rfd::FileDialog::new()
            .add_filter(t("Texturen-Pack oder Minecraft", "Texture pack or Minecraft"), &["zip", "jar"])
            .set_title(t("Minecraft-Texturen wählen", "Pick Minecraft textures"))
            .pick_file()
        else {
            return;
        };
        self.set_texture_source(src);
    }

    pub fn set_texture_source(&mut self, src: PathBuf) {
        if !crate::item_render::is_vanilla_source(&src) {
            self.dialog = Dialog::Message {
                title: t("Keine Minecraft-Texturen", "No Minecraft textures").into(),
                text: t(
                    "In dieser Datei fehlen die Block-Modelle und Texturen von Minecraft (assets/minecraft/models/block, textures/block). Nimm ein vollständiges Vanilla-Template-Pack für deine Minecraft-Version oder die Minecraft-.jar aus .minecraft/versions.",
                    "This file lacks Minecraft's block models and textures (assets/minecraft/models/block, textures/block). Use a complete vanilla template pack for your Minecraft version or the Minecraft .jar from .minecraft/versions.",
                )
                .into(),
            };
            return;
        }
        let mut stored = src.clone();
        if src
            .extension()
            .map(|e| e.eq_ignore_ascii_case("zip"))
            .unwrap_or(false)
        {
            let dir = build::config_dir().join("textures");
            let target = dir.join(src.file_name().unwrap_or_default());
            if target != src {
                match std::fs::create_dir_all(&dir).and_then(|_| std::fs::copy(&src, &target)) {
                    Ok(_) => stored = target,
                    Err(e) => self.log(
                        Level::Warn,
                        format!("Konnte das Texturen-Pack nicht kopieren, es wird vom alten Ort gelesen: {e}"),
                    ),
                }
            }
        }
        self.settings.texture_source = stored.display().to_string();
        self.settings.vanilla_textures = true;
        self.settings.save();
        self.reset_item_pictures();
        self.log(
            Level::Done,
            format!("Minecraft-Texturen: {}", stored.display()),
        );
    }

    /// Loads the project summaries for the start screen.
    pub fn project_summary(&mut self, p: &PathBuf) -> Option<ProjectSummary> {
        let modified = std::fs::metadata(p).and_then(|m| m.modified()).ok()?;
        if let Some((t, s)) = self.summaries.get(p) {
            if *t == modified {
                return s.clone();
            }
        }
        let s = Project::load(p).ok().map(|proj| ProjectSummary {
            name: proj.settings.name.clone(),
            platform: proj.settings.platform,
            minecraft: proj.settings.versions.minecraft.clone(),
            blocks: proj.workspaces.iter().map(|w| w.nodes.len()).sum(),
            commands: proj
                .workspaces
                .iter()
                .flat_map(|w| &w.nodes)
                .filter(|n| n.kind == "ev_command")
                .count(),
            modified,
        });
        self.summaries.insert(p.clone(), (modified, s.clone()));
        s
    }

    pub fn start_tools_setup(&mut self, ctx: &egui::Context, force_jdk: bool) {
        if self
            .setup
            .as_ref()
            .map(|s| s.result.is_none())
            .unwrap_or(false)
        {
            return;
        }
        self.setup = Some(SetupState {
            rx: crate::tools::start_setup(force_jdk, Some(ctx.clone())),
            status: t("Starte…", "Starting…").into(),
            progress: -1.0,
            lines: vec![],
            result: None,
        });
    }

    fn poll_setup(&mut self) {
        let Some(s) = self.setup.as_mut() else { return };
        let mut logs = vec![];
        while let Ok(m) = s.rx.try_recv() {
            match m {
                crate::tools::SetupMsg::Status(x) => {
                    s.status = x.clone();
                    logs.push((Level::Info, x));
                }
                crate::tools::SetupMsg::Progress(p) => s.progress = p,
                crate::tools::SetupMsg::Line(l) => {
                    s.lines.push(l);
                    if s.lines.len() > 200 {
                        s.lines.remove(0);
                    }
                }
                crate::tools::SetupMsg::Done(r) => {
                    match &r {
                        Ok(()) => logs.push((
                            Level::Done,
                            t("Java & Gradle sind bereit.", "Java & Gradle are ready.").to_string(),
                        )),
                        Err(e) => logs.push((Level::Error, e.clone())),
                    }
                    s.result = Some(r);
                    crate::tools::invalidate_status();
                }
            }
        }
        for (l, x) in logs {
            self.log(l, x);
        }
    }

    pub fn open_path(&mut self, p: PathBuf) {
        match Project::load(&p) {
            Ok(proj) => {
                self.log(Level::Done, format!("Geöffnet: {}", p.display()));
                self.settings.add_recent(&p);
                self.settings.save();
                self.set_project(proj, Some(p));
            }
            Err(e) => {
                self.log(
                    Level::Error,
                    format!("Konnte {} nicht öffnen: {e}", p.display()),
                );
                self.dialog = Dialog::Message {
                    title: t("Fehler", "Error").into(),
                    text: e,
                };
            }
        }
    }

    pub fn open_dialog(&mut self) {
        if self.dirty() {
            self.dialog = Dialog::UnsavedChanges {
                then: AfterSave::Open,
            };
            return;
        }
        self.open_dialog_now();
    }

    fn open_dialog_now(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("Minecraft Builder", &["mcb"])
            .set_title(t("Projekt öffnen", "Open project"))
            .pick_file()
        {
            self.open_path(p);
        }
    }

    pub fn new_project_dialog(&mut self) {
        if self.dirty() {
            self.dialog = Dialog::UnsavedChanges {
                then: AfterSave::NewProject,
            };
            return;
        }
        self.new_project_with(Starter::Welcome);
    }

    pub fn new_project_with(&mut self, starter: Starter) {
        self.dialog = Dialog::NewProject {
            name: t("MeinPlugin", "MyPlugin").into(),
            platform: Platform::Paper,
            starter,
            folder: build::default_projects_dir().display().to_string(),
        };
    }

    /// Creates the project and saves it right away into `<folder>/<name>/<id>.mcb`.
    pub fn create_project(
        &mut self,
        name: &str,
        platform: Platform,
        starter: Starter,
        folder: &str,
    ) {
        let p = examples::new_project(name, platform, starter);
        let mut path = None;
        if !folder.trim().is_empty() {
            let dir = PathBuf::from(folder.trim()).join(crate::model::slug(name));
            let file = dir.join(format!("{}.mcb", p.settings.id));
            match std::fs::create_dir_all(&dir)
                .map_err(|e| e.to_string())
                .and_then(|_| {
                    if file.exists() {
                        Err(t(
                            "Dort gibt es schon ein Projekt mit diesem Namen.",
                            "A project with this name already exists there.",
                        )
                        .into())
                    } else {
                        p.save(&file)
                    }
                }) {
                Ok(()) => {
                    self.settings.add_recent(&file);
                    self.settings.save();
                    path = Some(file);
                }
                Err(e) => {
                    self.dialog = Dialog::Message {
                        title: t("Fehler", "Error").into(),
                        text: e,
                    };
                    return;
                }
            }
        }
        self.set_project(p, path);
        self.log(
            Level::Info,
            format!("Neues Projekt „{name}“ ({})", platform.label()),
        );
    }

    /// Saves; asks for a path if needed. Returns false if cancelled or failed.
    pub fn save(&mut self, save_as: bool) -> bool {
        let path = if save_as || self.path.is_none() {
            let name = format!("{}.mcb", self.project.settings.id);
            match rfd::FileDialog::new()
                .add_filter("Minecraft Builder", &["mcb"])
                .set_file_name(&name)
                .set_title(t("Projekt speichern", "Save project"))
                .save_file()
            {
                Some(mut p) => {
                    if p.extension().is_none() {
                        p.set_extension("mcb");
                    }
                    p
                }
                None => return false,
            }
        } else {
            self.path.clone().unwrap()
        };
        match self.project.save(&path) {
            Ok(()) => {
                self.saved = self.project.clone();
                self.settings.add_recent(&path);
                self.settings.save();
                self.log(Level::Done, format!("Gespeichert: {}", path.display()));
                self.path = Some(path);
                true
            }
            Err(e) => {
                self.log(Level::Error, format!("Speichern fehlgeschlagen: {e}"));
                false
            }
        }
    }

    fn after_save(&mut self, then: AfterSave, ctx: &egui::Context) {
        match then {
            AfterSave::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            AfterSave::NewProject => {
                self.saved = self.project.clone();
                self.new_project_dialog();
            }
            AfterSave::Open => {
                self.saved = self.project.clone();
                self.open_dialog_now();
            }
            AfterSave::Home => {
                self.saved = self.project.clone();
                self.go_home();
            }
            AfterSave::Update => {
                self.saved = self.project.clone();
                self.install_update_now(ctx);
            }
        }
    }

    // ------------------------------------------------------------ updates

    pub fn check_updates(&mut self, ctx: &egui::Context) {
        if self.update.checking || self.update.downloading {
            return;
        }
        self.update.checking = true;
        self.update.error = None;
        self.update.rx = Some(crate::update::start_check(ctx.clone()));
    }

    pub fn download_update(&mut self, ctx: &egui::Context) {
        let Some(rel) = self.update.release.clone() else {
            return;
        };
        if self.update.downloading || self.update.installer.is_some() {
            return;
        }
        self.update.downloading = true;
        self.update.progress = 0.0;
        self.update.rx = Some(crate::update::start_download(rel, ctx.clone()));
    }

    /// Installs the downloaded update now and restarts the app.
    pub fn install_update_now(&mut self, ctx: &egui::Context) {
        if self.dirty() && self.screen == Screen::Editor {
            self.dialog = Dialog::UnsavedChanges {
                then: AfterSave::Update,
            };
            return;
        }
        let Some(path) = self.update.installer.take() else {
            return;
        };
        match crate::update::run_installer(&path, true) {
            Ok(()) => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(e) => {
                self.update.error = Some(e);
                self.update.installer = Some(path);
            }
        }
    }

    fn poll_update(&mut self, ctx: &egui::Context) {
        let mut msgs = vec![];
        if let Some(rx) = &self.update.rx {
            while let Ok(m) = rx.try_recv() {
                msgs.push(m);
            }
        }
        for m in msgs {
            match m {
                crate::update::UpdateMsg::Checked(r) => {
                    self.update.checking = false;
                    self.update.checked = true;
                    self.update.rx = None;
                    match r {
                        Ok(Some(rel)) => {
                            self.log(
                                Level::Info,
                                format!("Update verfügbar: Version {}", rel.version),
                            );
                            self.update.release = Some(rel);
                            if self.settings.auto_update && crate::update::is_installed() {
                                self.download_update(ctx);
                            }
                        }
                        Ok(None) => self.update.release = None,
                        Err(e) => self.update.error = Some(e),
                    }
                }
                crate::update::UpdateMsg::Progress(p) => self.update.progress = p,
                crate::update::UpdateMsg::Downloaded(r) => {
                    self.update.downloading = false;
                    self.update.rx = None;
                    match r {
                        Ok(p) => {
                            self.log(
                                Level::Done,
                                t(
                                    "Update heruntergeladen – wird beim Beenden installiert.",
                                    "Update downloaded – it is installed when you close the app.",
                                ),
                            );
                            self.update.installer = Some(p);
                        }
                        Err(e) => {
                            self.log(Level::Warn, format!("Update-Download fehlgeschlagen: {e}"));
                            self.update.error = Some(e);
                        }
                    }
                }
            }
        }
    }

    // --------------------------------------------------------------- undo

    /// Records an undo step when the project changed and no gesture is in progress.
    fn commit_undo(&mut self, ctx: &egui::Context) {
        let busy = ctx.input(|i| i.pointer.any_down()) || ctx.memory(|m| m.focused().is_some());
        if busy || self.project == self.committed {
            return;
        }
        let prev = std::mem::replace(&mut self.committed, self.project.clone());
        self.undo.push(prev);
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self) {
        if self.project != self.committed {
            // Uncommitted edit: just revert to the last committed state.
            let cur = std::mem::replace(&mut self.project, self.committed.clone());
            self.redo.push(cur);
        } else if let Some(prev) = self.undo.pop() {
            let cur = std::mem::replace(&mut self.project, prev);
            self.redo.push(cur);
            self.committed = self.project.clone();
        }
        self.fix_after_history();
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let cur = std::mem::replace(&mut self.project, next);
            self.undo.push(cur);
            self.committed = self.project.clone();
        }
        self.fix_after_history();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty() || self.project != self.committed
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn fix_after_history(&mut self) {
        if self.active_ws >= self.project.workspaces.len() {
            self.active_ws = 0;
        }
        let ids: BTreeSet<NodeId> = self.ws().nodes.iter().map(|n| n.id).collect();
        self.selection.retain(|i| ids.contains(i));
    }

    // ------------------------------------------------------------- editing

    pub fn add_node(&mut self, kind: &str, pos: egui::Pos2) -> NodeId {
        let id = self.project.alloc_id();
        let snap = self.settings.snap;
        let p = if snap { ui::canvas::snap_pos(pos) } else { pos };
        self.ws_mut().nodes.push(Node {
            id,
            kind: kind.into(),
            pos: [p.x, p.y],
            values: Default::default(),
        });
        self.selection.clear();
        self.selection.insert(id);
        self.selected_note = None;
        id
    }

    /// Exec outputs in the current workspace that nothing is connected to yet:
    /// (node, pin, label). Events and blocks with "next" first.
    pub fn free_flow_outputs(&self) -> Vec<(NodeId, String, String)> {
        let ws = self.ws();
        let mut out = vec![];
        for n in &ws.nodes {
            let Some(shape) = crate::blocks::shape(n, &self.project) else { continue };
            for o in shape.outputs.iter().filter(|o| o.ty == crate::blocks::PinType::Exec) {
                if ws.links_from(n.id, &o.id).next().is_some() {
                    continue;
                }
                let mut label = match self.project.node_command(n) {
                    Some(c) => format!("{} /{}", t("Befehl", "Command"), c.name),
                    None => crate::i18n::tl(shape.def.name).to_string(),
                };
                if o.id != "next" {
                    label = format!("{label} → {}", o.label.text());
                }
                out.push((n.id, o.id.to_string(), format!("{label}  #{}", n.id)));
            }
        }
        out
    }

    /// Adds a block and, if it has a flow input, connects it to the block that was
    /// selected before (or the nearest free flow output on its left).
    pub fn add_node_auto(&mut self, kind: &str, pos: egui::Pos2) -> NodeId {
        let before: Option<NodeId> = if self.selection.len() == 1 {
            self.selection.iter().next().copied()
        } else {
            None
        };
        let id = self.add_node(kind, pos);
        let Some(def) = crate::blocks::get(kind) else {
            return id;
        };
        if !def.has_exec_in() {
            return id;
        }
        let free = self.free_flow_outputs();
        let new_pos = self.ws().node(id).map(|n| n.pos).unwrap_or([pos.x, pos.y]);
        let pick = before
            .and_then(|b| free.iter().find(|(n, _, _)| *n == b && *n != id))
            .or_else(|| {
                // Nearest free output to the left, roughly on the same height.
                free.iter()
                    .filter(|(n, _, _)| *n != id)
                    .filter_map(|f| {
                        let p = self.ws().node(f.0)?.pos;
                        let (dx, dy) = (new_pos[0] - p[0], (new_pos[1] - p[1]).abs());
                        (dx > 40.0 && dx < 900.0 && dy < 260.0).then_some((dx + dy * 2.0, f))
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                    .map(|x| x.1)
            })
            .cloned();
        if let Some((from, pin, label)) = pick {
            ui::canvas::connect(
                self.ws_mut(),
                crate::model::PinRef { node: from, pin },
                crate::model::PinRef { node: id, pin: "in".into() },
                true,
            );
            self.log(
                Level::Info,
                format!("Automatisch verbunden: {label} → {}", crate::i18n::tl(def.name)),
            );
        }
        id
    }

    /// Texts of all blocks that contain `find`: count of occurrences.
    pub fn count_text(&self, find: &str) -> usize {
        if find.is_empty() {
            return 0;
        }
        self.project
            .workspaces
            .iter()
            .flat_map(|w| w.nodes.iter())
            .flat_map(|n| n.values.iter())
            .filter(|(k, _)| *k != "cmd")
            .map(|(_, v)| v.matches(find).count())
            .sum::<usize>()
            + self
                .project
                .workspaces
                .iter()
                .flat_map(|w| w.notes.iter())
                .map(|n| n.title.matches(find).count() + n.text.matches(find).count())
                .sum::<usize>()
    }

    /// Replaces `find` in all block values and notes. Returns how many were replaced.
    pub fn replace_text(&mut self, find: &str, with: &str) -> usize {
        let n = self.count_text(find);
        if n == 0 {
            return 0;
        }
        for w in &mut self.project.workspaces {
            for node in &mut w.nodes {
                for (k, v) in node.values.iter_mut() {
                    if k != "cmd" {
                        *v = v.replace(find, with);
                    }
                }
            }
            for note in &mut w.notes {
                note.title = note.title.replace(find, with);
                note.text = note.text.replace(find, with);
            }
        }
        self.log(Level::Done, format!("{n}× „{find}“ durch „{with}“ ersetzt"));
        n
    }

    // ---------------------------------------------------------------- docks

    pub fn dock_of(&self, v: View) -> Dock {
        self.settings
            .docks
            .iter()
            .find(|(x, _)| *x == v)
            .map(|(_, d)| *d)
            .unwrap_or(v.default_dock())
    }

    /// Views docked at one side, in their fixed order.
    pub fn views_in(&self, d: Dock) -> Vec<View> {
        View::ALL.into_iter().filter(|v| self.dock_of(*v) == d).collect()
    }

    /// The view shown in a dock (the remembered one if it is still docked there).
    pub fn active_in(&self, d: Dock) -> Option<View> {
        let cur = match d {
            Dock::Left => self.activity,
            Dock::Right => self.right_tab,
            Dock::Bottom => self.bottom_tab,
        };
        let views = self.views_in(d);
        if views.contains(&cur) {
            Some(cur)
        } else {
            views.first().copied()
        }
    }

    /// Brings a view to the front, wherever it is docked.
    pub fn show_view(&mut self, v: View) {
        match self.dock_of(v) {
            Dock::Left => {
                self.activity = v;
                self.show_sidebar = true;
            }
            Dock::Right => {
                self.right_tab = v;
                self.show_right = true;
            }
            Dock::Bottom => {
                self.bottom_tab = v;
                self.show_bottom = true;
            }
        }
    }

    /// Moves a view to another side and shows it there.
    pub fn move_view(&mut self, v: View, d: Dock) {
        self.settings.docks.retain(|(x, _)| *x != v);
        if d != v.default_dock() {
            self.settings.docks.push((v, d));
        }
        self.settings.save();
        self.show_view(v);
    }

    pub fn delete_selection(&mut self) {
        let sel: Vec<NodeId> = self.selection.iter().copied().collect();
        for id in sel {
            self.ws_mut().remove_node(id);
        }
        self.selection.clear();
        if let Some(n) = self.selected_note.take() {
            self.ws_mut().notes.retain(|x| x.id != n);
        }
    }

    pub fn copy_selection(&mut self, ctx: &egui::Context) {
        let ws = self.ws();
        let nodes: Vec<Node> = ws
            .nodes
            .iter()
            .filter(|n| self.selection.contains(&n.id))
            .cloned()
            .collect();
        if nodes.is_empty() {
            return;
        }
        let links: Vec<Link> = ws
            .links
            .iter()
            .filter(|l| {
                self.selection.contains(&l.from.node) && self.selection.contains(&l.to.node)
            })
            .cloned()
            .collect();
        let clip = Clip { nodes, links };
        if let Ok(s) = serde_json::to_string(&clip) {
            ctx.copy_text(format!("MCB-CLIP:{s}"));
        }
        self.clipboard = Some(clip);
    }

    pub fn paste(&mut self, clip: Clip, at: Option<egui::Pos2>) {
        if clip.nodes.is_empty() {
            return;
        }
        let min_x = clip.nodes.iter().map(|n| n.pos[0]).fold(f32::MAX, f32::min);
        let min_y = clip.nodes.iter().map(|n| n.pos[1]).fold(f32::MAX, f32::min);
        let offset = match at {
            Some(p) => egui::vec2(p.x - min_x, p.y - min_y),
            None => egui::vec2(40.0, 40.0),
        };
        let mut map = HashMap::new();
        self.selection.clear();
        for n in &clip.nodes {
            let id = self.project.alloc_id();
            map.insert(n.id, id);
            let mut nn = n.clone();
            nn.id = id;
            nn.pos = [n.pos[0] + offset.x, n.pos[1] + offset.y];
            self.ws_mut().nodes.push(nn);
            self.selection.insert(id);
        }
        for l in &clip.links {
            let mut nl = l.clone();
            nl.from.node = map[&l.from.node];
            nl.to.node = map[&l.to.node];
            self.ws_mut().links.push(nl);
        }
    }

    pub fn duplicate(&mut self, ctx: &egui::Context) {
        let old = self.clipboard.clone();
        self.copy_selection(ctx);
        if let Some(c) = self.clipboard.clone() {
            self.paste(c, None);
        }
        self.clipboard = old.or(self.clipboard.take());
    }

    pub fn add_workspace(&mut self) {
        let n = self.project.workspaces.len() + 1;
        self.project.workspaces.push(Workspace::new(&format!(
            "{} {n}",
            t("Bereich", "Workspace")
        )));
        self.active_ws = self.project.workspaces.len() - 1;
        self.selection.clear();
    }

    pub fn switch_ws(&mut self, i: usize) {
        if i < self.project.workspaces.len() && i != self.active_ws {
            self.active_ws = i;
            self.selection.clear();
            self.selected_note = None;
            self.palette = None;
        }
    }

    pub fn set_platform(&mut self, p: Platform) {
        if self.project.settings.platform == p {
            return;
        }
        self.project.settings.platform = p;
        let mc = self.project.settings.versions.minecraft.clone();
        let mut v = Versions::default_for(p);
        if p == Platform::Paper && mc.starts_with("1.") {
            v.minecraft = mc;
            v.api = format!("{}-R0.1-SNAPSHOT", v.minecraft);
            v.java = 21;
        }
        self.project.settings.versions = v;
        self.log(Level::Info, format!("Plattform: {}", p.label()));
    }

    pub fn fetch_versions(&mut self, ctx: &egui::Context) {
        let (tx, rx) = std::sync::mpsc::channel();
        let plat = self.project.settings.platform;
        let mc = self.project.settings.versions.minecraft.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(build::fetch_versions(plat, &mc));
            ctx.request_repaint();
        });
        self.version_job = Some(rx);
        self.log(
            Level::Info,
            t("Suche aktuelle Versionen…", "Looking up current versions…"),
        );
    }

    pub fn fetch_mc_versions(&mut self, ctx: &egui::Context) {
        if self.mc_job.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(build::fetch_minecraft_versions());
            ctx.request_repaint();
        });
        self.mc_job = Some(rx);
    }

    // ---------------------------------------------------------------- build

    pub fn start_build(&mut self, ctx: &egui::Context, export_only: bool) {
        if self.job.is_some() {
            return;
        }
        if self.path.is_none() || self.dirty() {
            if self.path.is_some() && !self.settings.autosave_on_build {
                // Ask where to save instead of overwriting silently.
                if !self.save(true) {
                    return;
                }
            } else if !self.save(false) {
                return;
            }
        }
        let path = self.path.clone().unwrap();
        let out = codegen::generate(&self.project);
        if out.has_errors() {
            self.problems = out.problems;
            self.show_view(View::Problems);
            self.log(
                Level::Error,
                t(
                    "Build gestoppt: Das Projekt hat Fehler.",
                    "Build stopped: the project has errors.",
                ),
            );
            return;
        }
        let dir = build::export_dir(&path, &self.project);
        self.show_view(View::Console);
        self.log(
            Level::Info,
            if export_only {
                t("Exportiere Gradle-Projekt…", "Exporting Gradle project…")
            } else {
                t("Build gestartet…", "Build started…")
            },
        );
        self.job_export_only = export_only;
        let server = if export_only {
            None
        } else {
            self.settings
                .server(self.project.settings.deploy_server)
                .cloned()
        };
        self.job = Some(build::start(
            self.project.clone(),
            self.path.clone(),
            dir,
            !export_only,
            server,
            ctx.clone(),
        ));
    }

    fn poll_jobs(&mut self) {
        let mut finished = None;
        let mut lines = vec![];
        if let Some(job) = &self.job {
            while let Ok(m) = job.rx.try_recv() {
                match m {
                    Msg::Line(l, s) => lines.push((l, s)),
                    Msg::Finished(r) => finished = Some(r),
                }
            }
        }
        for (l, s) in lines {
            self.log(l, s);
        }
        if let Some(r) = finished {
            self.job = None;
            match r {
                Ok(p) => {
                    if self.job_export_only {
                        self.log(Level::Done, format!("Exportiert: {}", p.display()));
                        let _ = open::that_detached(&p);
                    } else {
                        self.log(Level::Done, format!("Fertig! Deine Datei: {}", p.display()));
                        self.last_jar = Some(p);
                    }
                }
                Err(e) => self.log(Level::Error, e),
            }
        }
        if let Some(rx) = &self.version_job {
            if let Ok(r) = rx.try_recv() {
                self.version_job = None;
                match r {
                    Ok(v) => {
                        self.log(
                            Level::Done,
                            format!(
                                "Versionen aktualisiert: Minecraft {} · API {} · Java {}",
                                v.minecraft, v.api, v.java
                            ),
                        );
                        self.project.settings.versions = v;
                    }
                    Err(e) => self.log(
                        Level::Error,
                        format!("Versionen konnten nicht geladen werden: {e}"),
                    ),
                }
            }
        }
        if let Some(rx) = &self.mc_job {
            if let Ok(r) = rx.try_recv() {
                self.mc_job = None;
                match r {
                    Ok(v) => self.mc_versions = v,
                    Err(e) => self.log(
                        Level::Warn,
                        format!("Minecraft-Versionen nicht abrufbar: {e}"),
                    ),
                }
            }
        }
    }

    pub fn cancel_build(&mut self) {
        if let Some(j) = &self.job {
            j.cancel();
        }
    }

    /// Re-runs the code generator when the project changed (for problems + preview).
    /// Problems and the code preview are made in the background, so typing and
    /// dragging never wait for the code generator.
    fn refresh_generated(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.gen_job {
            match rx.try_recv() {
                Ok((p, out)) => {
                    self.problems = out.problems;
                    self.code_preview = out.main_java;
                    self.gen_for = Some(p);
                    self.gen_job = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(_) => self.gen_job = None,
            }
        }
        if self.gen_for.as_ref() == Some(&self.project) {
            return;
        }
        // While a block is dragged its position changes every frame: wait for the drop.
        if self.gen_for.is_some() && ctx.input(|i| i.pointer.any_down()) {
            return;
        }
        let p = self.project.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let out = codegen::generate(&p);
            let _ = tx.send((p, out));
            ctx.request_repaint();
        });
        self.gen_job = Some(rx);
    }

    // ------------------------------------------------------------ shortcuts

    fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let cmd = Modifiers::COMMAND;
        let shift_cmd = Modifiers::COMMAND | Modifiers::SHIFT;
        let sc = |m, k| KeyboardShortcut::new(m, k);
        if ctx.input_mut(|i| i.consume_shortcut(&sc(shift_cmd, Key::S))) {
            self.save(true);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&sc(cmd, Key::S))) {
            self.save(false);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&sc(cmd, Key::O))) {
            self.open_dialog();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&sc(cmd, Key::N))) {
            self.new_project_dialog();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&sc(Modifiers::NONE, Key::F5))) {
            self.start_build(ctx, false);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&sc(cmd, Key::P))) {
            self.canvas.focus_top_search = true;
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        if !typing {
            if ctx.input_mut(|i| {
                i.consume_shortcut(&sc(shift_cmd, Key::Z)) || i.consume_shortcut(&sc(cmd, Key::Y))
            }) {
                self.redo();
            }
            if ctx.input_mut(|i| i.consume_shortcut(&sc(cmd, Key::Z))) {
                self.undo();
            }
        }
    }

    fn home_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::O)))
        {
            self.open_dialog();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::N)))
        {
            self.new_project_dialog();
        }
    }

    // --------------------------------------------------------------- close

    fn handle_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.dirty() && self.screen == Screen::Editor {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.dialog = Dialog::UnsavedChanges {
                    then: AfterSave::Quit,
                };
            } else {
                self.settings.save();
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        if self.screen == Screen::Home {
            let title = "Minecraft Builder".to_string();
            if title != self.title {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
                self.title = title;
            }
            return;
        }
        let name = match &self.path {
            Some(p) => p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            None => t("Unbenannt", "Untitled").to_string(),
        };
        let title = format!(
            "{}{} – Minecraft Builder",
            name,
            if self.dirty() { " •" } else { "" }
        );
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    pub fn resolve_dialog_after_save(
        &mut self,
        then: AfterSave,
        save_first: bool,
        ctx: &egui::Context,
    ) {
        self.dialog = Dialog::None;
        if save_first && !self.save(false) {
            return;
        }
        self.after_save(then, ctx);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.vanilla.loaded_this_frame = 0;
        self.vanilla.checked_this_frame = false;
        self.poll_jobs();
        ui::settings::poll(self, &ctx);
        ui::plugins::poll(self, &ctx);
        self.poll_setup();
        self.poll_update(&ctx);
        if self.update.checking || self.update.downloading {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        self.handle_close(&ctx);

        if self.screen == Screen::Home {
            self.home_shortcuts(&ctx);
            ui::home::show(self, ui);
            ui::dialogs::show(self, &ctx);
            ui::onboarding::show(self, &ctx);
            self.update_title(&ctx);
            if self
                .setup
                .as_ref()
                .map(|s| s.result.is_none())
                .unwrap_or(false)
            {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            return;
        }

        let prof = std::env::var_os("MCB_PROFILE").is_some();
        let mut tk = std::time::Instant::now();
        let mut lap = |name: &'static str| {
            if prof {
                let e = tk.elapsed().as_secs_f64() * 1000.0;
                PROF.with(|p| p.borrow_mut().add(name, e));
                tk = std::time::Instant::now();
            }
        };
        self.shortcuts(&ctx);
        ui::menu_editor::publish_ids(&self.project);
        lap("publish_ids");
        self.refresh_generated(&ctx);
        lap("codegen");
        self.refresh_pack(false);
        lap("pack");
        ui::panels::top_bar(self, ui);
        ui::panels::status_bar(self, ui);
        ui::panels::activity_bar(self, ui);
        lap("bars");
        if self.show_sidebar {
            ui::panels::sidebar(self, ui);
        }
        lap("sidebar");
        if self.show_right {
            ui::panels::right_panel(self, ui);
        }
        lap("right");
        if self.show_bottom {
            ui::panels::bottom_panel(self, ui);
        }
        lap("bottom");
        egui::CentralPanel::no_frame().show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui::panels::tabs(self, ui);
            ui::canvas::show(self, ui);
        });
        lap("canvas");
        ui::panels::drop_zones(self, &ctx);
        ui::dialogs::show(self, &ctx);
        if self.show_commands {
            ui::commands::show(self, &ctx);
        }
        ui::menu_editor::show(self, &ctx);
        ui::recipe_editor::show(self, &ctx);
        ui::screen_editor::show(self, &ctx);
        ui::trace::window(self, &ctx);
        ui::menu_editor::item_picker(self, &ctx);
        ui::onboarding::show(self, &ctx);
        ui::onboarding::tutorial(self, &ctx);
        lap("windows");
        self.commit_undo(&ctx);
        self.update_title(&ctx);
        lap("undo+title");
        if prof {
            if let Some(cpu) = _frame.info().cpu_usage {
                PROF.with(|p| p.borrow_mut().add("CPU incl. tessellation (prev)", cpu as f64 * 1000.0));
            }
            PROF.with(|p| p.borrow_mut().frame());
        }
        let setup_running = self
            .setup
            .as_ref()
            .map(|s| s.result.is_none())
            .unwrap_or(false);
        if self.job.is_some()
            || self.version_job.is_some()
            || self.mc_job.is_some()
            || setup_running
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.cancel_build();
        self.settings.save();
        // A downloaded update is installed silently after closing.
        if let Some(p) = self.update.installer.take() {
            if self.settings.auto_update {
                let _ = crate::update::run_installer(&p, false);
            }
        }
    }
}

thread_local! {
    static PROF: std::cell::RefCell<Prof> = std::cell::RefCell::new(Prof::default());
}

/// Frame timing per part of the UI (MCB_PROFILE=1), printed every 120 frames.
#[derive(Default)]
struct Prof {
    sums: Vec<(&'static str, f64, f64)>,
    frames: u32,
}

impl Prof {
    fn add(&mut self, name: &'static str, ms: f64) {
        match self.sums.iter_mut().find(|s| s.0 == name) {
            Some(s) => {
                s.1 += ms;
                s.2 = s.2.max(ms);
            }
            None => self.sums.push((name, ms, ms)),
        }
    }
    fn frame(&mut self) {
        self.frames += 1;
        if self.frames >= 120 {
            let total: f64 = self.sums.iter().filter(|s| !s.0.starts_with("CPU")).map(|s| s.1).sum::<f64>() / self.frames as f64;
            let parts: Vec<String> = self
                .sums
                .iter()
                .map(|(n, sum, max)| format!("{n} {:.2}/{:.1}", sum / self.frames as f64, max))
                .collect();
            eprintln!("frame {total:.2} ms avg | {}", parts.join(" | "));
            self.sums.clear();
            self.frames = 0;
        }
    }
}
