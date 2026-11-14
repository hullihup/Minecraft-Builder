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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Workspaces,
    Blocks,
    Resources,
    Project,
    Build,
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
    pub path: PathBuf,
    pub info: Option<Result<crate::resourcepack::PackInfo, String>>,
    pub thumbs: HashMap<String, Option<egui::TextureHandle>>,
}

/// Short info about a project file for the start screen.
#[derive(Clone)]
pub struct ProjectSummary {
    pub name: String,
    pub platform: Platform,
    pub minecraft: String,
    pub blocks: usize,
    pub commands: usize,
    pub modified: std::time::SystemTime,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RightTab {
    Inspector,
    Code,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Console,
    Problems,
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
pub struct View {
    pub pan: egui::Vec2,
    pub zoom: f32,
}

impl Default for View {
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
    pub views: HashMap<usize, View>,
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
    gen_for: Option<Project>,
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
            gen_for: None,
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
    pub fn view(&mut self) -> &mut View {
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
        self.palette = None;
        self.canvas.fit_request = true;
        self.screen = Screen::Editor;
        self.cmd_draft = None;
        self.show_commands = false;
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

    /// Reloads the resource pack info if the path changed (or `force`).
    pub fn refresh_pack(&mut self, force: bool) {
        let s = &self.project.settings.pack;
        if !s.enabled() {
            if self.pack.info.is_some() || force {
                self.pack = PackCache::default();
                crate::resourcepack::set_known_models(vec![]);
            }
            return;
        }
        let path = s.resolve(self.path.as_deref());
        if !force && path == self.pack.path && self.pack.info.is_some() {
            return;
        }
        let info = crate::resourcepack::inspect(&path);
        crate::resourcepack::set_known_models(
            info.as_ref()
                .map(|i| i.item_models.clone())
                .unwrap_or_default(),
        );
        self.pack = PackCache {
            path,
            info: Some(info),
            thumbs: HashMap::new(),
        };
    }

    /// Adds a resource pack .zip to the project (copied next to the project file).
    pub fn add_pack(&mut self, src: PathBuf) {
        match crate::resourcepack::inspect(&src) {
            Err(e) => {
                self.dialog = Dialog::Message {
                    title: t("Resourcepack", "Resource pack").into(),
                    text: e,
                };
            }
            Ok(info) => {
                let mut stored = src.display().to_string();
                if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
                    let name = format!("{}-resourcepack.zip", self.project.settings.id);
                    let target = dir.join(&name);
                    if target != src {
                        match std::fs::copy(&src, &target) {
                            Ok(_) => stored = name,
                            Err(e) => self.log(
                                Level::Warn,
                                format!("Konnte das Pack nicht in den Projektordner kopieren: {e}"),
                            ),
                        }
                    } else {
                        stored = name;
                    }
                }
                self.project.settings.pack.path = stored;
                self.log(
                    Level::Done,
                    format!(
                        "Resourcepack hinzugefügt: {} Modelle, {} Texturen",
                        info.item_models.len(),
                        info.textures.len()
                    ),
                );
                self.refresh_pack(true);
            }
        }
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
            if !self.save(false) {
                return;
            }
        }
        let path = self.path.clone().unwrap();
        let out = codegen::generate(&self.project);
        if out.has_errors() {
            self.problems = out.problems;
            self.show_bottom = true;
            self.bottom_tab = BottomTab::Problems;
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
        self.show_bottom = true;
        self.bottom_tab = BottomTab::Console;
        self.log(
            Level::Info,
            if export_only {
                t("Exportiere Gradle-Projekt…", "Exporting Gradle project…")
            } else {
                t("Build gestartet…", "Build started…")
            },
        );
        self.job_export_only = export_only;
        self.job = Some(build::start(
            self.project.clone(),
            self.path.clone(),
            dir,
            !export_only,
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
    fn refresh_generated(&mut self) {
        if self.gen_for.as_ref() == Some(&self.project) {
            return;
        }
        let out = codegen::generate(&self.project);
        self.problems = out.problems;
        self.code_preview = out.main_java;
        self.gen_for = Some(self.project.clone());
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
        self.poll_jobs();
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

        self.shortcuts(&ctx);
        self.refresh_generated();
        self.refresh_pack(false);

        ui::panels::top_bar(self, ui);
        ui::panels::status_bar(self, ui);
        ui::panels::activity_bar(self, ui);
        if self.show_sidebar {
            ui::panels::sidebar(self, ui);
        }
        if self.show_right {
            ui::panels::right_panel(self, ui);
        }
        if self.show_bottom {
            ui::panels::bottom_panel(self, ui);
        }
        egui::CentralPanel::no_frame().show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui::panels::tabs(self, ui);
            ui::canvas::show(self, ui);
        });
        ui::dialogs::show(self, &ctx);
        if self.show_commands {
            ui::commands::show(self, &ctx);
        }
        ui::onboarding::show(self, &ctx);
        ui::onboarding::tutorial(self, &ctx);

        self.commit_undo(&ctx);
        self.update_title(&ctx);
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
