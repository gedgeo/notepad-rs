//! Notepad RS — reconstruction native des comportements observes de Notepad.
//!
//! Les preuves ayant servi a la reconstruction proviennent de REA (Ghidra
//! 12.1.4) sur `C:\Windows\System32\notepad.exe` : imports USER32/GDI32/
//! COMCTL32, `IsTextUnicode`, et les valeurs de registre `lfFaceName`,
//! `iPointSize`, `fWrap`, `fWrapAround`, `DefaultFonts`, `Autosave`.

// Pas de fenetre de console : l'application est une fenetre Win32 classique.
// L'attribut est desactive pour les tests afin de garder leur sortie standard.
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod blocks;
mod board_export;
mod close_dialogs;
mod doc;
mod encoding;
mod find;
mod format;
mod header;
mod markdown;
mod moodboard_panel;
mod commands;
#[cfg(test)]
mod notepad_checks;
mod page;
mod pdf_export;
mod print;
mod session;
mod settings;
mod ui_design;
mod workspace_io;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use close_dialogs::tab_snapshot;
use doc::{char_index_of_line, line_col, Doc, Eol, EOL_ALL};
use eframe::egui;
use encoding::{Encoding, ALL as ENCODINGS};
use session::Session;

fn main() -> eframe::Result<()> {
    // Comme Notepad : `notepad.exe fichier.txt` ouvre le fichier au demarrage.
    // `--new-window` (utilise par « Nouvelle fenetre ») demarre sans restaurer
    // la session precedente.
    let mut restore = true;
    let mut initial_path: Option<PathBuf> = None;
    let mut design_preview = None;
    let mut capture_to = None;
    for arg in std::env::args_os().skip(1) {
        let text = arg.to_string_lossy().to_string();
        if let Some(mode) = text.strip_prefix("--design-preview=") {
            design_preview = Some(mode.to_owned());
            restore = false;
        } else if let Some(path) = text.strip_prefix("--capture-to=") {
            capture_to = Some(path.to_owned());
        } else if text == "--new-window" || text == "--no-restore" {
            restore = false;
        } else if !text.starts_with('-') && initial_path.is_none() {
            initial_path = Some(PathBuf::from(arg));
        }
    }
    let session_file = if restore {
        session::session_path()
    } else {
        session::window_session_path(std::process::id())
    };
    let saved = Session::load_from(&session_file);
    // Un fichier en argument ne restaure pas la session, comme le Bloc-notes
    // ouvert par association de fichiers.
    let restore = restore && initial_path.is_none();
    let size = if design_preview
        .as_deref()
        .map(|mode| mode.contains("narrow"))
        .unwrap_or(false)
    {
        [640.0, 700.0]
    } else if design_preview.is_some() {
        [1440.0, 980.0]
    } else {
        saved
            .window
            .filter(|size| size[0] >= 460.0 && size[1] >= 300.0)
            .map(clamp_window_size)
            .unwrap_or([1200.0, 800.0])
    };

    let mut viewport = egui::ViewportBuilder::default().with_maximized(saved.maximized);
    if design_preview.is_none() {
        if let Some([x, y]) = saved
            .window_position
            .filter(|v| v.iter().all(|f| f.is_finite()))
        {
            viewport = viewport.with_position([x, y]);
        }
    }
    let options = eframe::NativeOptions {
        viewport: viewport
            .with_inner_size(size)
            .with_min_inner_size([460.0, 300.0])
            .with_title("Notepad RS")
            .with_drag_and_drop(true),
        ..Default::default()
    };
    let session = if restore {
        Some(saved)
    } else if design_preview.is_none() {
        let mut preferences = Session::load_from(&session::session_path());
        preferences.tabs.clear();
        preferences.active = 0;
        Some(preferences)
    } else {
        None
    };
    eframe::run_native(
        "Notepad RS",
        options,
        Box::new(move |cc| {
            let mut app = NotepadApp::new(cc, session.clone(), session_file.clone());
            if let Some(mode) = &design_preview {
                app.setup_preview(
                    mode,
                    capture_to
                        .as_deref()
                        .unwrap_or("samples/previews/interface.png"),
                );
            }
            if let Some(path) = initial_path.clone() {
                if path.exists() {
                    app.open_path(path);
                }
            }
            Ok(Box::new(app))
        }),
    )
}

/// Une taille de fenetre restauree ne doit jamais depasser l'ecran : une
/// session enregistree sur un ecran plus large ouvrirait une fenetre coupee.
fn clamp_window_size(size: [f32; 2]) -> [f32; 2] {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
        };
        let (screen_w, screen_h) = unsafe {
            (
                GetSystemMetrics(SM_CXSCREEN) as f32,
                GetSystemMetrics(SM_CYSCREEN) as f32,
            )
        };
        if screen_w >= 640.0 && screen_h >= 480.0 {
            // Laisse la place a la barre des taches et aux bordures.
            return [
                size[0].min(screen_w - 40.0).max(460.0),
                size[1].min(screen_h - 80.0).max(300.0),
            ];
        }
    }
    size
}

// ---------------------------------------------------------------------------
// Historique annuler/retablir
// ---------------------------------------------------------------------------

struct History {
    states: Vec<String>,
    index: usize,
    last_push: Instant,
}

impl History {
    fn new(text: &str) -> History {
        History {
            states: vec![text.to_owned()],
            index: 0,
            last_push: Instant::now() - Duration::from_secs(60),
        }
    }

    /// Enregistre une modification ; les frappes rapprochees sont fusionnees.
    fn note_change(&mut self, text: &str) {
        let now = Instant::now();
        let coalesce = now.duration_since(self.last_push) < Duration::from_millis(600)
            && self.index + 1 == self.states.len();
        if coalesce {
            if let Some(last) = self.states.get_mut(self.index) {
                last.clear();
                last.push_str(text);
            }
        } else {
            if self.index + 1 < self.states.len() {
                self.states.truncate(self.index + 1);
            }
            self.states.push(text.to_owned());
            while self.states.len() > 200 {
                self.states.remove(0);
            }
            self.index = self.states.len() - 1;
        }
        self.last_push = now;

        let mut total: usize = self.states.iter().map(|s| s.len()).sum();
        while total > 96 * 1024 * 1024 && self.states.len() > 2 {
            let removed = self.states.remove(0);
            total -= removed.len();
            self.index = self.index.saturating_sub(1);
        }
    }

    fn undo(&mut self) -> Option<String> {
        if self.index > 0 {
            self.index -= 1;
            Some(self.states[self.index].clone())
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<String> {
        if self.index + 1 < self.states.len() {
            self.index += 1;
            Some(self.states[self.index].clone())
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Etat de recherche
// ---------------------------------------------------------------------------

struct FindState {
    whole_words: bool,
    regex: bool,
    selection_only: bool,
    scope: Option<(u64, usize, usize)>,
    needle: String,
    replacement: String,
    match_case: bool,
    wrap: bool,
    status: String,
    focus_needle: bool,
}

impl Default for FindState {
    fn default() -> Self {
        FindState {
            whole_words: false,
            regex: false,
            selection_only: false,
            scope: None,
            needle: String::new(),
            replacement: String::new(),
            match_case: false,
            wrap: true,
            status: String::new(),
            focus_needle: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Document enrichi de son historique
// ---------------------------------------------------------------------------

struct Tab {
    doc: Doc,
    history: History,
    max_chars: usize,
    mood_items: Vec<session::MoodItem>,
    page_parent: Option<u64>,
    page_title: String,
    workspace: session::PageOptions,
}

impl Tab {
    fn new(id: u64) -> Tab {
        Tab {
            doc: Doc::new(id),
            history: History::new(""),
            max_chars: 0,
            mood_items: Vec::new(),
            page_parent: None,
            page_title: String::new(),
            workspace: session::PageOptions {
                uid: workspace_io::uid(),
                ..Default::default()
            },
        }
    }

    fn open(id: u64, path: PathBuf, bytes: Vec<u8>) -> Result<Tab, String> {
        let disk_hash = Some(workspace_io::hash(&bytes));
        let doc = Doc::from_bytes(id, path, bytes);
        let max_chars = doc
            .text
            .lines()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        Ok(Tab {
            history: History::new(&doc.text),
            doc,
            max_chars,
            mood_items: Vec::new(),
            page_parent: None,
            page_title: String::new(),
            workspace: session::PageOptions {
                disk_hash,
                uid: workspace_io::uid(),
                ..Default::default()
            },
        })
    }

    fn touch(&mut self) {
        self.workspace.modified_at = chrono::Utc::now().to_rfc3339();
        self.doc.dirty = true;
        self.max_chars = self
            .doc
            .text
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        let text = self.doc.text.clone();
        self.history.note_change(&text);
    }
}

// ---------------------------------------------------------------------------
// Application
// ---------------------------------------------------------------------------

enum Action {
    New,
    NewMoodboard,
    NewWindow,
    Open(Option<PathBuf>),
    Save,
    SaveAs,
    SaveAll,
    CloseTab,
    CloseWindow,
    Quit,
    Find,
    FindNext,
    FindPrevious,
    Replace,
    Goto,
    InsertDate,
    DeleteSelection,
    Font,
    SetEncoding(Encoding),
    ReopenEncoding(Encoding),
    SetEol(Eol),
    ToggleWrap,
    SetTheme(ThemeMode),
    ToggleStatus,
    OpenRecent(PathBuf),
    ClearRecent,
    CloseOthers(usize),
    CloseToTheRight(usize),
    CopyPath(usize),
    RevealPath(usize),
    Zoom(i32),
    ResetZoom,
    Undo,
    Redo,
    SelectAll,
    Cut,
    Copy,
    Paste,
    /// Active ou desactive le mode Markdown de l'onglet actif.
    ToggleMarkdown,
    /// Affiche ou masque l'apercu Markdown cote a cote.
    ToggleMarkdownPreview,
    /// Affiche ou masque la barre de mise en forme.
    ToggleFormatBar,
    /// Applique une mise en forme Markdown a la selection.
    Format(FormatKind),
    /// Boite « Mise en page ».
    PageSetup,
    /// Impression du document actif.
    Print,
    /// Apercu avant impression.
    PrintPreview,
    About,
}

/// Mises en forme proposees par la barre d'outils Markdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FormatKind {
    Bold,
    Italic,
    Strike,
    Heading(u8),
    BulletList,
    OrderedList,
    Quote,
    Code,
    CodeBlock,
    Link,
}

/// Preference de theme, comme dans les parametres du Bloc-notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemeMode {
    System,
    Light,
    Dark,
}

impl ThemeMode {
    fn label(self) -> &'static str {
        match self {
            ThemeMode::System => "Système",
            ThemeMode::Light => "Clair",
            ThemeMode::Dark => "Sombre",
        }
    }

    fn preference(self) -> egui::ThemePreference {
        match self {
            ThemeMode::System => egui::ThemePreference::System,
            ThemeMode::Light => egui::ThemePreference::Light,
            ThemeMode::Dark => egui::ThemePreference::Dark,
        }
    }
}

const DEFAULT_FONT_LABEL: &str = "Par défaut";

/// Famille grasse de repli pour les titres Markdown quand le document utilise
/// la police par defaut d'egui (chargée depuis `segoeuib.ttf`).
const DEFAULT_BOLD_FAMILY: &str = "notepad-bold";

/// Fichier gras correspondant a une famille proposee.
fn bold_font_file(name: &str) -> Option<&'static str> {
    match name {
        "Segoe UI" => Some("segoeuib.ttf"),
        "Segoe UI Variable" => Some("SegUIVar.ttf"),
        "Arial" => Some("arialbd.ttf"),
        "Calibri" => Some("calibrib.ttf"),
        "Consolas" => Some("consolab.ttf"),
        "Courier New" => Some("courbd.ttf"),
        "Georgia" => Some("georgiab.ttf"),
        "Tahoma" => Some("tahomabd.ttf"),
        "Times New Roman" => Some("timesbd.ttf"),
        "Verdana" => Some("verdanab.ttf"),
        _ => None,
    }
}

fn windows_fonts_dir() -> PathBuf {
    PathBuf::from(std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into())).join("Fonts")
}

/// Cached installed font faces, including style variants and collection indices.
fn available_system_fonts() -> &'static [(String, PathBuf, u32)] {
    static FONTS: std::sync::OnceLock<Vec<(String, PathBuf, u32)>> = std::sync::OnceLock::new();
    FONTS.get_or_init(|| {
        let mut dirs = vec![windows_fonts_dir()];
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
        }
        let mut fonts = Vec::new();
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if !["ttf", "otf", "ttc", "otc"].contains(&ext.as_str()) {
                    continue;
                }
                let Ok(data) = std::fs::read(&path) else {
                    continue;
                };
                for index in 0..ttf_parser::fonts_in_collection(&data).unwrap_or(1) {
                    let Ok(face) = ttf_parser::Face::parse(&data, index) else {
                        continue;
                    };
                    let name = face.names().into_iter().find_map(|n| {
                        if n.name_id == ttf_parser::name_id::FULL_NAME {
                            n.to_string()
                        } else {
                            None
                        }
                    });
                    if let Some(name) = name {
                        fonts.push((name, path.clone(), index));
                    }
                }
            }
        }
        fonts.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        fonts.dedup_by(|a, b| a.0 == b.0);
        fonts
    })
}

struct NotepadApp {
    options: settings::State,
    design: ui_design::State,
    tabs: Vec<Tab>,
    active: usize,
    next_id: u64,
    find: FindState,
    show_find: bool,
    show_replace: bool,
    show_goto: bool,
    show_about: bool,
    show_font: bool,
    goto_line: String,
    theme: ThemeMode,
    status_bar: bool,
    /// Police choisie (None = police par defaut d'egui).
    font_family: Option<String>,
    /// Familles deja enregistrees aupres du contexte egui.
    loaded_fonts: std::collections::HashSet<String>,
    pending_quit: bool,
    close_requested: bool,
    close_confirmed: bool,
    quit_save_pending: bool,
    quit_all_windows: bool,
    /// Identifiants des documents en attente de confirmation de fermeture.
    pending_close: Vec<u64>,
    /// Confirmation avant une conversion ANSI avec perte.
    pending_ansi_save: Option<PathBuf>,
    pending_ansi_tab: Option<u64>,
    session_file: PathBuf,
    /// Placer le curseur dans l'editeur au prochain affichage (restauration de
    /// session, changement d'onglet).
    pending_focus: bool,
    /// Liste des fichiers recents (chemins complets, plus recent en tete).
    recent: Vec<String>,
    preferences: session::NotepadPreferences,
    normal_window: Option<[f32; 4]>,
    /// Onglet en cours de deplacement par glisser (index d'origine).
    dragging_tab: Option<usize>,
    /// Retour a la ligne applique aux nouveaux onglets (preference de session).
    default_wrap: bool,
    /// Taille de police appliquee aux nouveaux onglets.
    default_font_size: f32,
    /// La session doit etre reecrite (frappe, ouverture, fermeture, option).
    session_dirty: bool,
    last_session_save: Instant,
    status: String,
    /// Mode Markdown actif sur l'onglet courant (raccourci d'affichage).
    show_markdown_preview: bool,
    show_format_bar: bool,
    /// Mise en page courante de la fenetre.
    page: page::PageSetup,
    show_page_setup: bool,
    /// Apercu avant impression plein cadre.
    show_print_preview: bool,
    /// Echelle manuelle de l'apercu (1.0 = ajuste a la largeur).
    preview_zoom: f32,
    /// Police grasse par defaut, chargee au demarrage si disponible.
    bold_default: bool,
    /// Familles dont la variante grasse a ete enregistree.
    bold_fonts: std::collections::HashSet<String>,
    /// Brouillon de la boite « Police » : (famille, taille).
    font_draft: (String, f32),
    /// Brouillon de la boite « Mise en page ».
    page_draft: page::PageSetup,
    /// Derniere selection non vide : (document, debut, fin) en caracteres.
    ///
    /// Cliquer sur la barre de mise en forme retire la selection du champ de
    /// texte ; on la retrouve ici pour appliquer le style au bon endroit.
    last_selection: Option<(u64, usize, usize)>,
    selected_mood_item: Option<usize>,
    page_search: String,
    sync_directory: Option<String>,
    sync_baseline: Vec<session::SessionTab>,
    last_sync: Instant,
    selected_mood_tab: Option<u64>,
}

impl NotepadApp {
    fn sync_workspace(&mut self, ctx: &egui::Context) {
        if self.options.sync_paused {
            return;
        }
        let Some(dir) = self.sync_directory.clone() else {
            return;
        };
        if self.last_sync.elapsed() < Duration::from_secs(self.preferences.sync_seconds) {
            return;
        }
        self.last_sync = Instant::now();
        let result = (|| -> std::io::Result<()> {
            let dir = PathBuf::from(dir);
            let _lock = workspace_io::FolderLock::acquire(&dir)?;
            let path = dir.join("notepad-rs-workspace.json");
            let local = workspace_io::portable(self.session_snapshot(ctx))?;
            let remote = if path.exists() {
                workspace_io::read(&path)?
            } else {
                Session::default()
            };
            let (merged, conflicts) = workspace_io::merge(&local, &remote, &self.sync_baseline);
            // Validate and materialize every incoming image before publishing or
            // replacing live tabs. A bad attachment leaves both copies intact.
            let mut ready = merged.clone();
            for page in &mut ready.tabs {
                workspace_io::materialize(page, &session::data_dir().join("assets"))?;
            }
            let merged_bytes = serde_json::to_vec(&merged.tabs)?;
            let remote_bytes = serde_json::to_vec(&remote.tabs)?;
            if merged_bytes != remote_bytes {
                merged.save_to(&path)?;
            }
            if merged_bytes != serde_json::to_vec(&local.tabs)? {
                let active_uid = self.tabs[self.active].workspace.uid.clone();
                let mut tabs = Vec::new();
                for page in ready.tabs {
                    if let Some(tab) = self.restore_tab(page) {
                        tabs.push(tab);
                    }
                }
                if !tabs.is_empty() {
                    self.tabs = tabs;
                    self.active = self
                        .tabs
                        .iter()
                        .position(|t| t.workspace.uid == active_uid)
                        .unwrap_or(0);
                    self.selected_mood_item = None;
                }
            }
            self.sync_baseline = merged.tabs;
            self.session_dirty = true;
            if conflicts > 0 {
                self.status =
                    format!("Synchronisation : {conflicts} conflit(s), copies conservées");
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.status = format!("Synchronisation suspendue : {error}");
            self.options.sync_error = Some(self.status.clone());
        } else {
            self.options.sync_error = None;
            self.options.last_sync = Some(chrono::Local::now().format("%H:%M:%S").to_string());
        }
        ctx.request_repaint_after(Duration::from_secs(self.preferences.sync_seconds));
    }

    fn new(
        cc: &eframe::CreationContext<'_>,
        session: Option<Session>,
        session_file: PathBuf,
    ) -> NotepadApp {
        cc.egui_ctx.set_theme(egui::ThemePreference::System);
        cc.egui_ctx.set_zoom_factor(1.0);
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "phosphor".into(),
            egui::FontData::from_static(egui_phosphor::Variant::Regular.font_bytes()).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("phosphor".into());
        if let Ok(bytes) = std::fs::read(windows_fonts_dir().join("segoeui.ttf")) {
            fonts
                .font_data
                .insert("segoe-ui".into(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "segoe-ui".into());
        }
        cc.egui_ctx.set_fonts(fonts);
        let mut app = NotepadApp {
            options: settings::State::default(),
            design: ui_design::State::default(),
            tabs: Vec::new(),
            active: 0,
            next_id: 0,
            find: FindState::default(),
            show_find: false,
            show_replace: false,
            show_goto: false,
            show_about: false,
            show_font: false,
            goto_line: String::new(),
            theme: ThemeMode::Light,
            status_bar: true,
            font_family: None,
            loaded_fonts: std::collections::HashSet::new(),
            pending_quit: false,
            close_requested: false,
            close_confirmed: false,
            quit_save_pending: false,
            quit_all_windows: false,
            pending_close: Vec::new(),
            pending_ansi_save: None,
            pending_ansi_tab: None,
            session_file,
            pending_focus: false,
            recent: Vec::new(),
            preferences: session::NotepadPreferences::default(),
            normal_window: None,
            dragging_tab: None,
            default_wrap: true,
            default_font_size: 15.0,
            session_dirty: false,
            last_session_save: Instant::now(),
            status: String::new(),
            show_markdown_preview: false,
            show_format_bar: false,
            page: page::PageSetup::default(),
            show_page_setup: false,
            show_print_preview: false,
            preview_zoom: 1.0,
            bold_default: false,
            bold_fonts: std::collections::HashSet::new(),
            font_draft: (DEFAULT_FONT_LABEL.to_string(), 15.0),
            page_draft: page::PageSetup::default(),
            last_selection: None,
            selected_mood_item: None,
            page_search: String::new(),
            sync_directory: None,
            sync_baseline: Vec::new(),
            last_sync: Instant::now(),
            selected_mood_tab: None,
        };
        if let Some(session) = session {
            let scale = if session.ui_density_version == 0
                && (session.interface_scale - 1.25).abs() < 0.01
            {
                1.0
            } else {
                session.interface_scale
            };
            cc.egui_ctx.set_zoom_factor(scale.clamp(0.8, 1.75));
            app.design.sidebar = session.navigation_visible;
            app.design.sidebar_width = session.navigation_width.clamp(190.0, 340.0);
            app.sync_directory = session.sync_directory.clone();
            app.sync_baseline = session.sync_baseline.clone();
            app.theme = match session.theme {
                1 => ThemeMode::Light,
                2 => ThemeMode::Dark,
                _ => ThemeMode::System,
            };
            app.status_bar = session.status_bar;
            app.default_wrap = session.wrap;
            app.default_font_size = if session.font_size >= 8.0 {
                session.font_size
            } else {
                15.0
            };
            app.font_family = session.font;
            if let Some([w, h]) = session.window {
                let [x, y] = session.window_position.unwrap_or([40.0, 40.0]);
                app.normal_window = Some([x, y, w, h]);
            }
            app.preferences = session.preferences;
            app.preferences.sanitize();
            app.recent = if app.preferences.recent_enabled {
                session.recent
            } else {
                Vec::new()
            };
            app.page = session.page.sanitized();
            let mut used_ids = std::collections::HashSet::new();
            let mut fresh_id = session
                .tabs
                .iter()
                .map(|t| t.page_id)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            for mut saved_tab in session.tabs {
                if !used_ids.insert(saved_tab.page_id) {
                    saved_tab.page_id = fresh_id;
                    used_ids.insert(fresh_id);
                    fresh_id = fresh_id.saturating_add(1);
                }
                if let Some(tab) = app.restore_tab(saved_tab) {
                    app.tabs.push(tab);
                }
            }
            if !app.tabs.is_empty() {
                app.active = session.active.min(app.tabs.len() - 1);
                app.pending_focus = true;
            }
        }
        if app.tabs.is_empty() {
            app.new_tab();
        } else if app.preferences.start_blank {
            app.new_tab();
        }
        app.options.recovery = Session::load_report(&app.session_file).1;
        app.register_bold_default(&cc.egui_ctx);
        app
    }

    /// Reconstruit un onglet depuis la session : texte autosauvegarde s'il
    /// existe, sinon relecture du fichier sur disque.
    fn restore_tab(&mut self, mut saved: session::SessionTab) -> Option<Tab> {
        if let Err(error) =
            workspace_io::materialize(&mut saved, &session::data_dir().join("assets"))
        {
            self.status = format!("Pièce jointe invalide : {error}");
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        let path = saved.path.map(PathBuf::from);
        let encoding = session::encoding_from_code(saved.encoding);
        let eol = session::eol_from_code(saved.eol);
        let mut tab = match (&path, &saved.text) {
            (Some(path), None) => {
                let bytes = std::fs::read(path).ok()?;
                Tab::open(id, path.clone(), bytes).ok()?
            }
            (path, Some(text)) => {
                let mut doc = Doc::new(id);
                doc.path = path.clone();
                doc.text = text.clone();
                doc.encoding = encoding;
                doc.eol = eol;
                doc.dirty = saved.dirty;
                let max_chars = doc
                    .text
                    .lines()
                    .map(|l| l.chars().count())
                    .max()
                    .unwrap_or(0);
                Tab {
                    history: History::new(&doc.text),
                    doc,
                    max_chars,
                    mood_items: Vec::new(),
                    page_parent: None,
                    page_title: String::new(),
                    workspace: Default::default(),
                }
            }
            (None, None) => return None,
        };
        tab.doc.moodboard = saved.moodboard;
        tab.doc.id = saved.page_id;
        self.next_id = self.next_id.max(saved.page_id.saturating_add(1));
        tab.page_parent = saved.parent_id;
        tab.page_title = saved.page_title;
        tab.workspace = saved.workspace;
        tab.workspace.favorite |= tab.workspace.pinned;
        tab.workspace.pinned = false;
        if tab.workspace.uid.is_empty() {
            tab.workspace.uid = workspace_io::uid();
        }
        tab.mood_items = saved.mood_items;
        // Un fichier relu sur disque peut avoir ete reinterprete dans un autre
        // codage sans etre modifie : on reapplique le choix de l'utilisateur.
        if tab.doc.encoding != encoding {
            tab.doc.reinterpret(encoding);
        }
        tab.doc.wrap = saved.wrap;
        tab.doc.font_size = if saved.font_size >= 8.0 {
            saved.font_size
        } else {
            self.default_font_size
        };
        tab.doc.markdown = saved.markdown;
        let cursor = saved.cursor.min(tab.doc.text.chars().count());
        tab.doc.cursor_char = cursor;
        Some(tab)
    }

    fn active_tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    fn active_tab_mut(&mut self) -> &mut Tab {
        let index = self.active;
        &mut self.tabs[index]
    }

    fn new_tab(&mut self) {
        self.new_page_kind(self.preferences.default_kind);
    }

    fn new_page_kind(&mut self, kind: u8) {
        let id = self.next_id;
        self.next_id += 1;
        let mut tab = Tab::new(id);
        tab.workspace.uid = workspace_io::uid();
        tab.doc.wrap = self.default_wrap;
        tab.doc.font_size = self.default_font_size;
        tab.doc.encoding = session::encoding_from_code(self.preferences.default_encoding);
        tab.doc.eol = session::eol_from_code(self.preferences.default_eol);
        tab.workspace.modified_at = chrono::Utc::now().to_rfc3339();
        tab.workspace.visual_blocks = kind == 1;
        tab.doc.markdown = kind == 1;
        tab.doc.moodboard = kind == 2;
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.session_dirty = true;
    }

    fn new_moodboard(&mut self) {
        let mut tab = Tab::new(self.next_id);
        self.next_id += 1;
        tab.doc.moodboard = true;
        tab.doc.dirty = true;
        tab.mood_items.push(session::MoodItem::default());
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.session_dirty = true;
    }

    fn open_path(&mut self, path: PathBuf) {
        // Deja ouvert : on active l'onglet existant plutot que d'ouvrir un
        // doublon (le Bloc-notes 11 se comporte ainsi).
        let text = path.to_string_lossy().to_string();
        if let Some(index) = self.tabs.iter().position(|tab| {
            tab.doc
                .path
                .as_ref()
                .is_some_and(|open| open.to_string_lossy().eq_ignore_ascii_case(&text))
        }) {
            self.active = index;
            self.pending_focus = true;
            self.status = format!("Ouvert : {}", path.display());
            self.remember_recent(&path);
            return;
        }
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("Ouverture impossible : {error}");
                return;
            }
        };
        // Reutilise l'onglet vide courant, comme le fait Notepad.
        let reuse = {
            let tab = &self.tabs[self.active];
            tab.doc.path.is_none()
                && tab.doc.text.is_empty()
                && !tab.doc.dirty
                && self.tabs.len() > 0
        };
        let id = if reuse {
            self.tabs[self.active].doc.id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            id
        };
        if let Ok(mut saved) = serde_json::from_slice::<session::SessionTab>(&bytes) {
            if saved.moodboard {
                saved.page_id = id;
                saved.path = Some(path.to_string_lossy().into_owned());
                saved.dirty = false;
                if saved.text.is_none() {
                    saved.text = Some(String::new());
                }
                if let Some(tab) = self.restore_tab(saved) {
                    if reuse {
                        self.tabs[self.active] = tab;
                    } else {
                        self.tabs.push(tab);
                        self.active = self.tabs.len() - 1;
                    }
                    self.remember_recent(&path);
                    self.session_dirty = true;
                    self.status = format!("Moodboard ouvert : {}", path.display());
                }
                return;
            }
        }
        match Tab::open(id, path.clone(), bytes) {
            Ok(mut tab) => {
                tab.doc.wrap = self.default_wrap;
                tab.doc.font_size = self.default_font_size;
                if reuse {
                    self.tabs[self.active] = tab;
                } else {
                    self.tabs.push(tab);
                    self.active = self.tabs.len() - 1;
                }
                self.status = format!("Ouvert : {}", path.display());
                self.remember_recent(&path);
                self.session_dirty = true;
            }
            Err(error) => self.status = error,
        }
    }

    /// Ajoute un chemin aux recents et demande la reecriture de la session.
    fn open_requested(&mut self, path: PathBuf) {
        if self.preferences.open_new_window {
            let result = std::env::current_exe().and_then(|exe| {
                std::process::Command::new(exe)
                    .arg("--new-window")
                    .arg(&path)
                    .spawn()
            });
            match result {
                Ok(_) => self.remember_recent(&path),
                Err(e) => self.status = format!("Ouverture impossible : {e}"),
            }
        } else {
            self.open_path(path);
        }
    }

    fn remember_recent(&mut self, path: &std::path::Path) {
        if !self.preferences.recent_enabled {
            return;
        }
        if self.preferences.recent_limit == session::RECENT_LIMIT {
            session::touch_recent_list(&mut self.recent, path);
        } else {
            session::touch_recent_list_with_limit(
                &mut self.recent,
                path,
                self.preferences.recent_limit,
            );
        }
        self.session_dirty = true;
    }

    fn save_active(&mut self, save_as: bool) {
        let tab = self.active_tab_mut();
        let target = if save_as || tab.doc.path.is_none() {
            if tab.doc.moodboard {
                rfd::FileDialog::new()
                    .set_title("Enregistrer le moodboard")
                    .set_file_name("Moodboard.json")
                    .add_filter("Moodboard Notepad RS", &["json"])
                    .save_file()
            } else {
                rfd::FileDialog::new()
                    .set_title("Enregistrer sous")
                    .add_filter("Documents texte", &["txt", "log", "md", "csv", "json"])
                    .add_filter("Tous les fichiers", &["*"])
                    .save_file()
            }
        } else {
            tab.doc.path.clone()
        };
        let Some(path) = target else { return };
        if !tab.doc.moodboard && tab.doc.encoding.loses_information(&tab.doc.text) {
            let id = tab.doc.id;
            self.pending_ansi_tab = Some(id);
            self.pending_ansi_save = Some(path);
            return;
        }
        self.write_active_to(path);
    }

    fn write_active_to(&mut self, path: PathBuf) {
        if self.options.allow_overwrite.take() != Some(self.active_tab().doc.id)
            && self.active_tab().doc.path.as_ref() == Some(&path)
        {
            if let Some(expected) = self.active_tab().workspace.disk_hash {
                if std::fs::read(&path).ok().map(|b| workspace_io::hash(&b)) != Some(expected) {
                    self.options.external_write = Some((self.active_tab().doc.id, path));
                    self.status = "Le fichier a changé sur disque. Choisissez une copie ou comparez avant de remplacer.".into();
                    return;
                }
            }
        }
        if self.active_tab().doc.moodboard {
            let mut page = tab_snapshot(self.active_tab());
            page.path = Some(path.to_string_lossy().into_owned());
            page.dirty = false;
            let snapshot = Session {
                tabs: vec![page],
                ..Default::default()
            };
            let result = workspace_io::portable(snapshot)
                .and_then(|s| serde_json::to_vec_pretty(&s.tabs[0]).map_err(std::io::Error::other))
                .and_then(|data| std::fs::write(&path, data));
            match result {
                Ok(()) => {
                    let tab = self.active_tab_mut();
                    tab.doc.path = Some(path.clone());
                    tab.doc.dirty = false;
                    tab.workspace.disk_hash =
                        std::fs::read(&path).ok().map(|b| workspace_io::hash(&b));
                    self.status = format!("Moodboard enregistré : {}", path.display());
                    self.remember_recent(&path);
                }
                Err(error) => self.status = format!("Enregistrement impossible : {error}"),
            }
            return;
        }
        let tab = self.active_tab_mut();
        let bytes = tab.doc.bytes_for_save();
        match std::fs::write(&path, bytes) {
            Ok(()) => {
                tab.doc.path = Some(path.clone());
                tab.doc.dirty = false;
                tab.doc.raw = std::fs::read(&path).ok();
                tab.workspace.disk_hash = tab.doc.raw.as_ref().map(|b| workspace_io::hash(b));
                self.status = format!("Enregistre : {}", path.display());
                self.remember_recent(&path);
            }
            Err(error) => self.status = format!("Enregistrement impossible : {error}"),
        }
    }

    fn close_tab(&mut self, index: usize) {
        if self.tabs.is_empty() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.new_tab();
        }
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        self.pending_focus = true;
        self.session_dirty = true;
    }

    /// Ferme tous les onglets sauf `keep` (demande de confirmation pour les
    /// documents modifies, un par un).
    fn close_others(&mut self, keep: usize) {
        let keep_id = self.tabs.get(keep).map(|tab| tab.doc.id);
        let mut index = self.tabs.len();
        while index > 0 {
            index -= 1;
            if Some(self.tabs[index].doc.id) == keep_id {
                continue;
            }
            self.maybe_close_tab(index);
        }
        // L'onglet conserve peut avoir glisse vers la gauche.
        if let Some(id) = keep_id {
            if let Some(position) = self.tabs.iter().position(|tab| tab.doc.id == id) {
                self.active = position;
            }
        }
    }

    /// Ferme les onglets situes a droite de `index`.
    fn close_to_the_right(&mut self, index: usize) {
        let mut cursor = self.tabs.len();
        while cursor > index + 1 {
            cursor -= 1;
            self.maybe_close_tab(cursor);
        }
        self.active = self.active.min(self.tabs.len() - 1);
    }

    fn maybe_close_tab(&mut self, index: usize) {
        let Some(tab) = self.tabs.get(index) else {
            return;
        };
        if tab.doc.dirty {
            let id = tab.doc.id;
            if !self.pending_close.contains(&id) {
                self.pending_close.push(id);
            }
        } else {
            self.close_tab(index);
        }
    }

    fn has_unsaved(&self) -> bool {
        self.tabs.iter().any(|tab| tab.doc.dirty)
    }

    /// « Tout enregistrer » : chemin connu = ecriture directe, sinon dialogue.
    fn save_all(&mut self) {
        let mut saved = 0usize;
        let mut pending = 0usize;
        for index in 0..self.tabs.len() {
            if !self.tabs[index].doc.dirty {
                continue;
            }
            self.active = index;
            self.save_active(false);
            if self.pending_ansi_save.is_some() {
                self.status =
                    "Confirmation ANSI requise avant de poursuivre l’enregistrement".into();
                return;
            }
            if self.tabs[index].doc.dirty {
                pending += 1;
            } else {
                saved += 1;
            }
        }
        self.status = if pending > 0 {
            format!("{saved} enregistre(s), {pending} sans chemin")
        } else {
            format!("{saved} fichier(s) enregistre(s)")
        };
    }

    /// Supprime la selection courante (« Supprimer » du menu Edition).
    fn delete_selection(&mut self) {
        let tab = self.active_tab_mut();
        let Some((start, end)) = tab.doc.pending_selection else {
            return;
        };
        if end <= start {
            return;
        }
        let byte_start = char_to_byte(&tab.doc.text, start);
        let byte_end = char_to_byte(&tab.doc.text, end);
        tab.doc.text.replace_range(byte_start..byte_end, "");
        tab.touch();
        tab.doc.cursor_char = start;
        tab.doc.selection_chars = 0;
        tab.doc.pending_selection = Some((start, start));
    }

    /// Nouvelle fenetre : un processus separe, comme le Bloc-notes classique.
    /// `--new-window` indique au nouveau processus de ne pas restaurer la
    /// session (il doit demarrer vierge).
    fn spawn_new_window() {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).arg("--new-window").spawn();
        }
    }

    /// Charge une police systeme dans egui (une seule fois par nom).
    fn ensure_font(&mut self, ctx: &egui::Context, name: &str) {
        if self.loaded_fonts.contains(name) {
            return;
        }
        let Some((_, path, face_index)) = available_system_fonts()
            .into_iter()
            .find(|(font_name, _, _)| font_name == name)
        else {
            return;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let family = egui::FontFamily::Name(name.into());
        let mut data = egui::FontData::from_owned(bytes);
        data.index = *face_index;
        ctx.add_font(egui::FontInsert::new(
            name,
            data,
            vec![egui::InsertFontFamily {
                family,
                priority: egui::FontPriority::Highest,
            }],
        ));
        self.loaded_fonts.insert(name.to_owned());

        // Variante grasse, pour l'apercu Markdown.
        if let Some(file) = bold_font_file(name) {
            if let Ok(bytes) = std::fs::read(windows_fonts_dir().join(file)) {
                let bold_family = format!("{name} Bold");
                ctx.add_font(egui::FontInsert::new(
                    &bold_family,
                    egui::FontData::from_owned(bytes),
                    vec![egui::InsertFontFamily {
                        family: egui::FontFamily::Name(bold_family.clone().into()),
                        priority: egui::FontPriority::Highest,
                    }],
                ));
                self.bold_fonts.insert(name.to_owned());
            }
        }
    }

    /// Charge `segoeuib.ttf` comme famille grasse de secours (titres Markdown
    /// quand aucune police n'est choisie).
    fn register_bold_default(&mut self, ctx: &egui::Context) {
        if let Ok(bytes) = std::fs::read(windows_fonts_dir().join("segoeuib.ttf")) {
            ctx.add_font(egui::FontInsert::new(
                DEFAULT_BOLD_FAMILY,
                egui::FontData::from_owned(bytes),
                vec![egui::InsertFontFamily {
                    family: egui::FontFamily::Name(DEFAULT_BOLD_FAMILY.into()),
                    priority: egui::FontPriority::Highest,
                }],
            ));
            self.bold_default = true;
        }
    }

    /// Familles (normale, grasse) de l'apercu Markdown.
    fn preview_fonts(&self) -> (egui::FontFamily, egui::FontFamily) {
        match &self.font_family {
            Some(name) => {
                let regular = egui::FontFamily::Name(name.clone().into());
                let bold = if self.bold_fonts.contains(name) {
                    egui::FontFamily::Name(format!("{name} Bold").into())
                } else {
                    regular.clone()
                };
                (regular, bold)
            }
            None => {
                let bold = if self.bold_default {
                    egui::FontFamily::Name(DEFAULT_BOLD_FAMILY.into())
                } else {
                    egui::FontFamily::Proportional
                };
                (egui::FontFamily::Proportional, bold)
            }
        }
    }

    fn editor_font(&self, size: f32) -> egui::FontId {
        match &self.font_family {
            Some(name) => egui::FontId::new(size, egui::FontFamily::Name(name.clone().into())),
            None => egui::FontId::new(size, egui::FontFamily::Proportional),
        }
    }

    /// Etat a persister : onglets (texte autosauvegarde si modifie),
    /// preferences d'affichage, geometrie et fichiers recents.
    fn session_snapshot(&self, ctx: &egui::Context) -> Session {
        let tabs = self.tabs.iter().map(tab_snapshot).collect();
        // Never call Context again from inside an input closure: it holds the
        // context read lock. Native viewport geometry makes this branch run,
        // unlike test inputs without an inner_rect.
        let interface_scale = ctx.zoom_factor();
        let window = ctx.input(|input| {
            input.viewport().inner_rect.map(|rect| {
                [
                    rect.width() * interface_scale,
                    rect.height() * interface_scale,
                ]
            })
        });
        Session {
            tabs,
            active: self.active,
            theme: match self.theme {
                ThemeMode::System => 0,
                ThemeMode::Light => 1,
                ThemeMode::Dark => 2,
            },
            font: self.font_family.clone(),
            font_size: self.default_font_size,
            wrap: self.default_wrap,
            status_bar: self.status_bar,
            window: if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
                self.normal_window.map(|r| [r[2], r[3]]).or(window)
            } else {
                window
            },
            window_position: if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
                self.normal_window.map(|r| [r[0], r[1]])
            } else {
                ctx.input(|i| {
                    i.viewport()
                        .outer_rect
                        .map(|r| [r.min.x * interface_scale, r.min.y * interface_scale])
                })
            },
            maximized: ctx.input(|i| i.viewport().maximized.unwrap_or(false)),
            preferences: self.preferences.clone(),
            recent: self.recent.clone(),
            page: self.page.clone(),
            sync_directory: self.sync_directory.clone(),
            sync_baseline: self.sync_baseline.clone(),
            navigation_visible: self.design.sidebar,
            navigation_width: self.design.sidebar_width,
            interface_scale,
            ui_density_version: 1,
        }
    }

    fn save_session(&mut self, ctx: &egui::Context) {
        if self.design.capture.is_some() {
            return;
        }
        let snapshot = self.session_snapshot(ctx);
        match snapshot.save_to(&self.session_file) {
            Ok(()) => {
                self.session_dirty = false;
                self.options.save_error = None;
                self.options.last_saved = Some(chrono::Local::now().format("%H:%M:%S").to_string());
            }
            Err(error) => {
                self.status = format!("Session non enregistrée : {error}");
                self.options.save_error = Some(self.status.clone());
            }
        }
        self.last_session_save = Instant::now();
    }

    fn editor_id(id: u64) -> egui::Id {
        egui::Id::unique(("notepad-editor", id))
    }

    fn apply_pending_selection(&mut self, ctx: &egui::Context) {
        let Some((doc_id, range)) = self
            .tabs
            .get(self.active)
            .and_then(|tab| tab.doc.pending_selection.map(|sel| (tab.doc.id, sel)))
        else {
            return;
        };
        let id = Self::editor_id(doc_id);
        {
            let mut state = match egui::text_edit::TextEditState::load(ctx, id) {
                Some(state) => state,
                None => return,
            };
            let range = egui::text::CCursorRange::two(
                egui::text::CCursor::new(range.0),
                egui::text::CCursor::new(range.1),
            );
            state.cursor.set_char_range(Some(range));
            state.store(ctx, id);
            ctx.memory_mut(|memory| memory.request_focus(id));
        }
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.doc.pending_selection = None;
        }
    }

    /// Pose le curseur de l'onglet actif a `doc.cursor_char` (restauration de
    /// session ou changement d'onglet).
    fn focus_active_cursor(&mut self, ctx: &egui::Context) {
        let Some(tab) = self.tabs.get(self.active) else {
            return;
        };
        let id = Self::editor_id(tab.doc.id);
        let cursor = tab.doc.cursor_char;
        let Some(mut state) = egui::text_edit::TextEditState::load(ctx, id) else {
            return;
        };
        let range = egui::text::CCursorRange::one(egui::text::CCursor::new(cursor));
        state.cursor.set_char_range(Some(range));
        state.store(ctx, id);
        ctx.memory_mut(|memory| memory.request_focus(id));
        self.pending_focus = false;
    }

    // -----------------------------------------------------------------------
    // Zone d'edition et apercus (Lot 3)
    // -----------------------------------------------------------------------

    /// Editeur de l'onglet actif. Renvoie `true` si le texte a change.
    fn editor_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> bool {
        let index = self.active;
        let font = self.editor_font(self.tabs[index].doc.font_size * 17.0 / 15.0);
        let line_height = font.size * self.preferences.line_height;
        let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = wrap_width;
            job.append(
                buffer.as_str(),
                0.0,
                egui::TextFormat {
                    font_id: font.clone(),
                    color: ui.visuals().text_color(),
                    line_height: Some(line_height),
                    ..Default::default()
                },
            );
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let tab = &mut self.tabs[index];
        let wrap = tab.doc.wrap;
        let width = ui.available_width();
        let mut typed = false;

        let scroll = if wrap {
            egui::ScrollArea::vertical().auto_shrink([false, false])
        } else {
            egui::ScrollArea::both().auto_shrink([false, false])
        };

        let output = scroll.show(ui, |ui| {
            let desired = if wrap {
                width
            } else {
                let estimated = tab.max_chars as f32 * tab.doc.font_size * 0.55 + 48.0;
                estimated.max(width - 24.0)
            };
            let editor_id = NotepadApp::editor_id(tab.doc.id);
            egui::TextEdit::multiline(&mut tab.doc.text)
                .interactive(!tab.workspace.read_only)
                .id(editor_id)
                .font(font.clone())
                .layouter(&mut layouter)
                .desired_width(desired)
                .desired_rows(30)
                .lock_focus(true)
                .frame(egui::Frame::NONE)
                .hint_text("Commencez à écrire…")
                .show(ui)
        });

        if output.inner.response.response.changed() {
            tab.touch();
            typed = true;
        }
        if let Some(cursor) = output.inner.cursor_range {
            tab.doc.cursor_char = cursor.primary.index.0;
            let (primary, secondary) = (cursor.primary.index.0, cursor.secondary.index.0);
            tab.doc.selection_chars = primary.abs_diff(secondary);
            let doc_id = tab.doc.id;
            if primary != secondary {
                let (start, end) = if primary < secondary {
                    (primary, secondary)
                } else {
                    (secondary, primary)
                };
                tab.doc.pending_selection = Some((start, end));
                self.last_selection = Some((doc_id, start, end));
            } else if output.inner.response.response.clicked() {
                // Un clic dans le texte efface la selection memorisee ; un clic
                // sur la barre de mise en forme, non.
                self.last_selection = None;
            }
        }
        // Toute frappe (touche, saisie, collage) retire la selection du champ :
        // la selection memorisee ne doit pas survivre a une edition.
        let keyboard = ctx.input(|input| {
            input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Key { pressed: true, .. }
                        | egui::Event::Text(_)
                        | egui::Event::Paste(_)
                        | egui::Event::Copy
                        | egui::Event::Cut
                )
            })
        });
        if keyboard {
            self.last_selection = None;
        }

        let zoom = ctx.input(|input| input.zoom_delta());
        if (zoom - 1.0).abs() > f32::EPSILON && output.inner.response.response.hovered() {
            tab.doc.font_size = (tab.doc.font_size * zoom).clamp(8.0, 60.0);
        }
        typed
    }

    /// Apercu Markdown de l'onglet actif, cote a cote avec l'editeur.

    fn markdown_preview_ui(&mut self, ui: &mut egui::Ui) {
        let blocks = markdown::parse(&self.active_tab().doc.text);
        let (regular, bold) = self.preview_fonts();
        let options = markdown::RenderOptions {
            regular,
            bold,
            font_size: self.active_tab().doc.font_size,
        };
        let margin = self.active_tab().doc.font_size * 0.6;
        egui::ScrollArea::vertical()
            .id_salt("markdown-preview")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(margin * 0.5);
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.vertical(|ui| markdown::render(ui, &blocks, &options));
                    ui.add_space(margin);
                });
            });
    }

    /// Apercu avant impression : pages mises en page comme a l'impression.
    fn print_preview_ui(&mut self, ui: &mut egui::Ui) {
        let font_pt = self.print_font_pt();
        let (regular, _) = self.preview_fonts();
        let font_id = egui::FontId::new(font_pt, regular.clone());
        let bands = page::Bands::from_setup(&self.page);
        let measured = {
            let ctx = ui.ctx().clone();
            let font_id = font_id.clone();
            let mut measure = |line: &str| -> f32 {
                ctx.fonts_mut(|fonts| {
                    fonts
                        .layout_no_wrap(line.to_owned(), font_id.clone(), egui::Color32::BLACK)
                        .rect
                        .width()
                })
            };
            page::paginate(
                &self.active_tab().doc.text,
                &self.page,
                font_pt,
                bands,
                &mut measure,
            )
        };

        let mut print = false;
        let mut close = false;
        ui.horizontal(|ui| {
            if ui.button("Imprimer...").clicked() {
                print = true;
            }
            if ui.button("Fermer l'aperçu").clicked() {
                close = true;
            }
            ui.separator();
            if ui.button("−").clicked() {
                self.preview_zoom = (self.preview_zoom - 0.1).clamp(0.4, 3.0);
            }
            ui.label(format!(
                "Zoom {} %",
                (self.preview_zoom * 100.0).round() as i32
            ));
            if ui.button("+").clicked() {
                self.preview_zoom = (self.preview_zoom + 0.1).clamp(0.4, 3.0);
            }
            if ui.button("Ajuster").clicked() {
                self.preview_zoom = 1.0;
            }
            ui.separator();
            ui.label(format!(
                "{} — {} page(s)",
                self.page.paper_label(),
                measured.pages.len()
            ));
        });
        ui.separator();

        let available = ui.available_size();
        let page_w = measured.size_pt.0.max(1.0);
        let page_h = measured.size_pt.1.max(1.0);
        let fit = ((available.x - 48.0) / page_w).max(0.1);
        let scale = (fit * self.preview_zoom).clamp(0.05, 8.0);
        let text_font = egui::FontId::new((font_pt * scale).max(1.0), regular);
        let title = self.active_tab().doc.title();
        let now = chrono::Local::now();
        let date = now.format("%d/%m/%Y").to_string();
        let time = now.format("%H:%M").to_string();

        egui::ScrollArea::vertical()
            .id_salt("print-preview")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(12.0);
                let total = measured.pages.len();
                for (index, lines) in measured.pages.iter().enumerate() {
                    let size = egui::vec2(page_w * scale, page_h * scale);
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    let rect =
                        rect.translate(egui::vec2(((available.x - size.x) / 2.0).max(0.0), 0.0));
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 2.0, egui::Color32::WHITE);
                    painter.rect_stroke(
                        rect,
                        2.0,
                        egui::Stroke::new(1.0, egui::Color32::from_gray(150)),
                        egui::StrokeKind::Outside,
                    );
                    let fields = header::Fields {
                        file: &title,
                        page: index + 1,
                        total,
                        date: &date,
                        time: &time,
                    };
                    let band = |template: &str, top_pt: f32| {
                        let sections = header::resolve(template, &fields);
                        if sections.is_empty() {
                            return;
                        }
                        let y = rect.min.y + top_pt * scale;
                        let left = rect.min.x + measured.margins_pt[3] * scale;
                        let right = rect.max.x - measured.margins_pt[1] * scale;
                        for (text, x, align) in [
                            (&sections.left, left, egui::Align2::LEFT_TOP),
                            (&sections.center, rect.center().x, egui::Align2::CENTER_TOP),
                            (&sections.right, right, egui::Align2::RIGHT_TOP),
                        ] {
                            if text.is_empty() {
                                continue;
                            }
                            painter.text(
                                egui::pos2(x, y),
                                align,
                                text,
                                text_font.clone(),
                                egui::Color32::from_gray(90),
                            );
                        }
                    };
                    band(&self.page.header, measured.margins_pt[0]);
                    band(
                        &self.page.footer,
                        page_h - measured.margins_pt[2] - measured.footer_pt(),
                    );
                    for (line_index, line) in lines.iter().enumerate() {
                        if line.is_empty() {
                            continue;
                        }
                        let y =
                            measured.text_top_pt() + line_index as f32 * measured.line_height_pt;
                        painter.text(
                            rect.min + egui::vec2(measured.margins_pt[3] * scale, y * scale),
                            egui::Align2::LEFT_TOP,
                            line,
                            text_font.clone(),
                            egui::Color32::BLACK,
                        );
                    }
                    ui.add_space(14.0);
                }
            });

        if print {
            self.print_active(ui.ctx());
        }
        if close {
            self.show_print_preview = false;
        }
    }

    /// Barre de mise en forme Markdown (sous les onglets).
    fn format_bar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.horizontal_wrapped(|ui| {
            let button = |ui: &mut egui::Ui, label: &str, tip: &str| -> bool {
                ui.add(
                    egui::Button::new(egui::RichText::new(label).monospace())
                        .min_size(egui::vec2(24.0, 22.0)),
                )
                .on_hover_text(tip)
                .clicked()
            };
            if button(ui, "B", "Gras (Ctrl+B)") {
                actions.push(Action::Format(FormatKind::Bold));
            }
            if button(ui, "I", "Italique (Ctrl+I)") {
                actions.push(Action::Format(FormatKind::Italic));
            }
            if button(ui, "S", "Barré") {
                actions.push(Action::Format(FormatKind::Strike));
            }
            if button(ui, "`", "Code en ligne") {
                actions.push(Action::Format(FormatKind::Code));
            }
            ui.separator();
            for level in 1..=3u8 {
                if button(ui, &format!("H{level}"), "Titre") {
                    actions.push(Action::Format(FormatKind::Heading(level)));
                }
            }
            ui.separator();
            if button(ui, "•", "Liste à puces") {
                actions.push(Action::Format(FormatKind::BulletList));
            }
            if button(ui, "1.", "Liste numérotée") {
                actions.push(Action::Format(FormatKind::OrderedList));
            }
            if button(ui, ">", "Citation") {
                actions.push(Action::Format(FormatKind::Quote));
            }
            if button(ui, "{ }", "Bloc de code") {
                actions.push(Action::Format(FormatKind::CodeBlock));
            }
            if button(ui, "Lien", "Lien") {
                actions.push(Action::Format(FormatKind::Link));
            }
            ui.separator();
            let mut preview = self.show_markdown_preview;
            if ui.checkbox(&mut preview, "Aperçu").clicked() {
                actions.push(Action::ToggleMarkdownPreview);
            }
        });
    }

    /// Fenetre « Mise en page » (papier, orientation, marges).
    fn page_setup_window(&mut self, ctx: &egui::Context) {
        if !self.show_page_setup {
            return;
        }
        let mut open = self.show_page_setup;
        let mut apply = false;
        let mut cancel = false;
        let mut reset = false;
        {
            let setup = &mut self.page_draft;
            egui::Window::new("Mise en page")
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Papier :");
                        egui::ComboBox::from_id_salt("paper")
                            .selected_text(setup.paper_label())
                            .show_ui(ui, |ui| {
                                for (index, (label, _, _)) in page::PAPERS.iter().enumerate() {
                                    let selected = setup.paper as usize == index;
                                    if ui.selectable_label(selected, *label).clicked() {
                                        setup.paper = index as u8;
                                    }
                                }
                            });
                    });
                    ui.checkbox(&mut setup.landscape, "Orientation paysage");
                    ui.separator();
                    ui.label("Marges (mm)");
                    egui::Grid::new("margins").num_columns(2).show(ui, |ui| {
                        let labels = ["Haut", "Droite", "Bas", "Gauche"];
                        for index in 0..4 {
                            ui.label(labels[index]);
                            ui.add(
                                egui::DragValue::new(&mut setup.margins[index])
                                    .speed(0.5)
                                    .range(0.0..=100.0)
                                    .suffix(" mm"),
                            );
                            ui.end_row();
                        }
                    });
                    ui.separator();
                    ui.label("En-tête et pied de page");
                    egui::Grid::new("bands").num_columns(2).show(ui, |ui| {
                        ui.label("En-tête");
                        ui.add(
                            egui::TextEdit::singleline(&mut setup.header)
                                .desired_width(250.0)
                                .hint_text("&f"),
                        );
                        ui.end_row();
                        ui.label("Pied de page");
                        ui.add(
                            egui::TextEdit::singleline(&mut setup.footer)
                                .desired_width(250.0)
                                .hint_text("Page &p"),
                        );
                        ui.end_row();
                    });
                    ui.label(
                        egui::RichText::new("&f fichier · &p page · &P total · &d date · &t heure")
                            .small()
                            .weak(),
                    );
                    ui.label(
                        egui::RichText::new(
                            "&l / &c / &r : gauche / centre / droite · && : esperluette",
                        )
                        .small()
                        .weak(),
                    );
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Valeurs par défaut").clicked() {
                            reset = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("OK").clicked() {
                                apply = true;
                            }
                            if ui.button("Annuler").clicked() {
                                cancel = true;
                            }
                        });
                    });
                });
        }
        if reset {
            self.page_draft.margins = [page::DEFAULT_MARGIN_MM; 4];
            self.page_draft.header = header::DEFAULT_HEADER.to_string();
            self.page_draft.footer = header::DEFAULT_FOOTER.to_string();
        }
        if apply {
            self.page = self.page_draft.sanitized();
            self.show_page_setup = false;
            self.session_dirty = true;
            self.status = format!(
                "Mise en page : {}, marges {:.1} mm.",
                self.page.paper_label(),
                self.page.margins[0]
            );
        } else if cancel {
            self.show_page_setup = false;
        } else {
            self.show_page_setup = open;
        }
    }

    fn find_matches(&self) -> Result<Vec<find::Match>, String> {
        let tab = self.active_tab();
        let found = find::occurrences(
            &tab.doc.text,
            &self.find.needle,
            self.find.match_case,
            self.find.whole_words,
            self.find.regex,
        )?;
        if !self.find.selection_only {
            return Ok(found);
        }
        let Some((id, a, b)) = self.find.scope else {
            return Ok(Vec::new());
        };
        if id != tab.doc.id {
            return Ok(Vec::new());
        }
        let start = char_to_byte(&tab.doc.text, a);
        let end = char_to_byte(&tab.doc.text, b);
        Ok(found
            .into_iter()
            .filter(|m| m.start >= start && m.end <= end)
            .collect())
    }

    fn find_next(&mut self, backwards: bool) {
        let found = match self.find_matches() {
            Ok(found) => found,
            Err(error) => {
                self.find.status = error;
                return;
            }
        };
        let text = &self.active_tab().doc.text;
        let from = char_to_byte(text, self.active_tab().doc.cursor_char);
        let before = self
            .active_tab()
            .doc
            .pending_selection
            .filter(|(a, b)| a < b && *b == self.active_tab().doc.cursor_char)
            .map(|(a, _)| char_to_byte(text, a))
            .unwrap_or(from);
        let selected = if backwards {
            found.iter().rev().find(|m| m.end <= before).or_else(|| {
                if self.find.wrap {
                    found.last()
                } else {
                    None
                }
            })
        } else {
            found.iter().find(|m| m.start >= from).or_else(|| {
                if self.find.wrap {
                    found.first()
                } else {
                    None
                }
            })
        };
        if let Some(m) = selected {
            let a = count_chars(text, m.start);
            let b = count_chars(text, m.end);
            let tab = self.active_tab_mut();
            tab.doc.pending_selection = Some((a, b));
            tab.doc.cursor_char = b;
            self.find.status = format!(
                "Occurrence {} sur {}",
                found.iter().position(|v| v == m).unwrap_or(0) + 1,
                found.len()
            );
        } else {
            self.find.status = "Aucune occurrence dans ce sens et ce perimetre.".into();
        }
    }

    fn replace_current(&mut self) {
        if self.active_tab().workspace.read_only {
            self.find.status = "Page en lecture seule".into();
            return;
        }
        let Some((a, b)) = self.active_tab().doc.pending_selection else {
            return;
        };
        let start = char_to_byte(&self.active_tab().doc.text, a);
        let end = char_to_byte(&self.active_tab().doc.text, b);
        let found = match self.find_matches() {
            Ok(found) => found,
            Err(error) => {
                self.find.status = error;
                return;
            }
        };
        if found.iter().any(|m| m.start == start && m.end == end) {
            let replacement = self.find.replacement.clone();
            self.create_version("Avant remplacement");
            let tab = self.active_tab_mut();
            tab.doc.text.replace_range(start..end, &replacement);
            tab.touch();
            tab.doc.cursor_char = a + replacement.chars().count();
            if let Some((_, _, scope_end)) = &mut self.find.scope {
                *scope_end = scope_end.saturating_sub(b - a) + replacement.chars().count();
            }
            self.session_dirty = true;
        }
        self.find_next(false);
    }

    fn replace_all(&mut self) {
        if self.active_tab().workspace.read_only {
            self.find.status = "Page en lecture seule".into();
            return;
        }
        let found = match self.find_matches() {
            Ok(found) => found,
            Err(error) => {
                self.find.status = error;
                return;
            }
        };
        if found.is_empty() {
            self.find.status = "Aucune occurrence a remplacer".into();
            return;
        }
        self.create_version("Avant remplacement global");
        let replacement = self.find.replacement.clone();
        let mut removed = 0;
        let tab = self.active_tab_mut();
        for m in found.iter().rev() {
            removed += tab.doc.text[m.start..m.end].chars().count();
            tab.doc.text.replace_range(m.start..m.end, &replacement);
        }
        tab.touch();
        tab.doc.pending_selection = None;
        if let Some((_, _, end)) = &mut self.find.scope {
            *end = end.saturating_sub(removed) + found.len() * replacement.chars().count();
        }
        self.session_dirty = true;
        self.find.status = format!(
            "{} remplacement(s). Une version de recuperation a ete conservee.",
            found.len()
        );
    }

    fn goto_line(&mut self) {
        let line = self.goto_line.trim().parse::<usize>().unwrap_or(0);
        let tab = self.active_tab_mut();
        if let Some(index) = char_index_of_line(&tab.doc.text, line) {
            tab.doc.pending_selection = Some((index, index));
            tab.doc.cursor_char = index;
            self.show_goto = false;
            self.goto_line.clear();
        } else {
            self.status = format!("Ligne {line} hors du document.");
        }
    }

    fn insert_date(&mut self) {
        let stamp = chrono::Local::now().format("%H:%M %d/%m/%Y").to_string();
        let tab = self.active_tab_mut();
        let offset = char_to_byte(&tab.doc.text, tab.doc.cursor_char);
        tab.doc.text.insert_str(offset, &stamp);
        tab.touch();
        tab.doc.cursor_char += stamp.chars().count();
        tab.doc.pending_selection = Some((tab.doc.cursor_char, tab.doc.cursor_char));
    }

    fn select_all(&mut self) {
        let tab = self.active_tab_mut();
        let total = tab.doc.text.chars().count();
        tab.doc.pending_selection = Some((0, total));
        tab.doc.cursor_char = total;
    }

    /// Selection de l'editeur actif, en index de caracteres (debut <= fin).
    fn char_selection(&self, ctx: &egui::Context) -> (usize, usize) {
        let tab = self.active_tab();
        let id = Self::editor_id(tab.doc.id);
        if let Some(state) = egui::text_edit::TextEditState::load(ctx, id) {
            if let Some(range) = state.cursor.char_range() {
                let (a, b) = (range.primary.index.0, range.secondary.index.0);
                if a != b {
                    return if a <= b { (a, b) } else { (b, a) };
                }
                // Cliquer sur un bouton de la barre retire la selection du
                // champ, mais l'action doit porter sur le texte selectionne.
                if let Some((doc_id, start, end)) = self.last_selection {
                    if doc_id == tab.doc.id {
                        return (start, end);
                    }
                }
                return (a, b);
            }
        }
        (tab.doc.cursor_char, tab.doc.cursor_char)
    }

    fn selection_text(&self, start: usize, end: usize) -> String {
        self.active_tab()
            .doc
            .text
            .chars()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect()
    }

    /// Remplace `[start_char, end_char)` par `insert` et selectionne `selection`.
    fn apply_edit(
        &mut self,
        ctx: &egui::Context,
        start_char: usize,
        end_char: usize,
        insert: String,
        selection: (usize, usize),
    ) {
        let tab = self.active_tab_mut();
        let start = char_to_byte(&tab.doc.text, start_char);
        let end = char_to_byte(&tab.doc.text, end_char);
        if start > end || end > tab.doc.text.len() {
            return;
        }
        tab.doc.text.replace_range(start..end, &insert);
        tab.doc.cursor_char = selection.1;
        tab.doc.selection_chars = selection.1.saturating_sub(selection.0);
        tab.touch();
        tab.doc.pending_selection = Some(selection);
        self.apply_pending_selection(ctx);
    }

    fn format_selection(&mut self, ctx: &egui::Context, kind: FormatKind) {
        let (start, end) = self.char_selection(ctx);
        match kind {
            FormatKind::Bold => self.wrap_inline(ctx, start, end, "**", "**"),
            FormatKind::Italic => self.wrap_inline(ctx, start, end, "*", "*"),
            FormatKind::Strike => self.wrap_inline(ctx, start, end, "~~", "~~"),
            FormatKind::Code => self.wrap_inline(ctx, start, end, "`", "`"),
            FormatKind::Link => self.wrap_link(ctx, start, end),
            other => self.format_lines(ctx, start, end, other),
        }
    }

    fn wrap_inline(
        &mut self,
        ctx: &egui::Context,
        start: usize,
        end: usize,
        left: &str,
        right: &str,
    ) {
        let selected = self.selection_text(start, end);
        let left_len = left.chars().count();
        let right_len = right.chars().count();
        // Marqueurs deja poses autour de la selection : les retirer.
        if !selected.is_empty()
            && start >= left_len
            && end + right_len <= self.active_tab().doc.text.chars().count()
            && self.selection_text(start - left_len, start) == left
            && self.selection_text(end, end + right_len) == right
        {
            let from = start - left_len;
            let to = end + right_len;
            let inner = (from, from + selected.chars().count());
            self.apply_edit(ctx, from, to, selected, inner);
            return;
        }
        // Selection comprenant les marqueurs : les retirer aussi.
        if !selected.is_empty()
            && selected.chars().count() > left_len + right_len
            && selected.starts_with(left)
            && selected.ends_with(right)
        {
            let inner: String = selected
                .chars()
                .skip(left_len)
                .take(selected.chars().count() - left_len - right_len)
                .collect();
            let to = start + inner.chars().count();
            self.apply_edit(ctx, start, end, inner, (start, to));
            return;
        }
        let insert = format!("{left}{selected}{right}");
        let selection = if selected.is_empty() {
            let inside = start + left_len;
            (inside, inside)
        } else {
            let from = start + left_len;
            (from, from + selected.chars().count())
        };
        self.apply_edit(ctx, start, end, insert, selection);
    }

    fn wrap_link(&mut self, ctx: &egui::Context, start: usize, end: usize) {
        let selected = self.selection_text(start, end);
        if selected.is_empty() {
            let insert = "[texte](https://)".to_string();
            let from = start + "[texte](".chars().count();
            self.apply_edit(ctx, start, end, insert, (from, from + 8));
            return;
        }
        let insert = format!("[{selected}](https://)");
        let from = start + 1 + selected.chars().count() + 2;
        self.apply_edit(ctx, start, end, insert, (from, from + 8));
    }

    /// Mise en forme portant sur les lignes couvertes par la selection.
    fn format_lines(&mut self, ctx: &egui::Context, start: usize, end: usize, kind: FormatKind) {
        let format = match kind {
            FormatKind::Heading(level) => format::LineFormat::Heading(level),
            FormatKind::BulletList => format::LineFormat::Bullet,
            FormatKind::OrderedList => format::LineFormat::Ordered,
            FormatKind::Quote => format::LineFormat::Quote,
            FormatKind::CodeBlock => format::LineFormat::CodeBlock,
            _ => return,
        };
        let text = self.active_tab().doc.text.clone();
        let Some(edit) = format::apply(&text, start, end, format) else {
            return;
        };
        self.apply_edit(ctx, edit.start, edit.end, edit.replacement, edit.selection);
    }

    /// Taille d'impression en points (l'editeur est exprime en pixels logiques).
    fn print_font_pt(&self) -> f32 {
        (self.active_tab().doc.font_size * 0.75).clamp(6.0, 48.0)
    }

    fn print_face(&self) -> (String, bool) {
        match self.font_family.as_deref() {
            Some(name) => (
                name.to_string(),
                matches!(name, "Consolas" | "Courier New" | "Lucida Console"),
            ),
            None => ("Segoe UI".to_string(), false),
        }
    }

    fn print_active(&mut self, ctx: &egui::Context) {
        let (face, monospace) = self.print_face();
        let (start, end) = self.char_selection(ctx);
        let selection = if start < end {
            Some(self.selection_text(start, end))
        } else {
            None
        };
        let tab = self.active_tab();
        let job = print::PrintJob {
            text: tab.doc.text.clone(),
            title: tab.doc.title(),
            setup: self.page.clone(),
            font_pt: self.print_font_pt(),
            face,
            monospace,
            selection,
        };
        match print::print(&job) {
            Ok(print::PrintOutcome::Cancelled) => {}
            Ok(print::PrintOutcome::Printed { pages, copies }) => {
                self.status = format!(
                    "Document envoyé à l'imprimante : {} — {pages} page(s), {copies} copie(s)",
                    job.title
                );
            }
            Err(error) => self.status = error,
        }
    }

    fn undo(&mut self) {
        let tab = self.active_tab_mut();
        if let Some(text) = tab.history.undo() {
            tab.doc.text = text;
            tab.doc.dirty = true;
            tab.doc.cursor_char = tab.doc.cursor_char.min(tab.doc.text.chars().count());
        }
    }

    fn redo(&mut self) {
        let tab = self.active_tab_mut();
        if let Some(text) = tab.history.redo() {
            tab.doc.text = text;
            tab.doc.dirty = true;
            tab.doc.cursor_char = tab.doc.cursor_char.min(tab.doc.text.chars().count());
        }
    }

    fn handle_action(&mut self, ctx: &egui::Context, action: Action) {
        if self.active_tab().workspace.read_only
            && matches!(
                action,
                Action::Undo
                    | Action::Redo
                    | Action::DeleteSelection
                    | Action::InsertDate
                    | Action::Format(_)
                    | Action::SetEncoding(_)
                    | Action::ReopenEncoding(_)
                    | Action::SetEol(_)
            )
        {
            self.status =
                "Cette page est en lecture seule. Déverrouillez-la dans Propriétés.".into();
            return;
        }
        match action {
            Action::New => self.new_tab(),
            Action::NewMoodboard => self.new_moodboard(),
            Action::NewWindow => Self::spawn_new_window(),
            Action::Open(path) => {
                let path = path.or_else(|| {
                    rfd::FileDialog::new()
                        .set_title("Ouvrir")
                        .add_filter(
                            "Documents texte",
                            &["txt", "log", "md", "csv", "json", "ini"],
                        )
                        .add_filter("Tous les fichiers", &["*"])
                        .pick_file()
                });
                if let Some(path) = path {
                    self.open_requested(path);
                }
            }
            Action::Save => self.save_active(false),
            Action::SaveAs => self.save_active(true),
            Action::SaveAll => self.save_all(),
            Action::CloseTab => self.maybe_close_tab(self.active),
            Action::CloseWindow => {
                self.quit_all_windows = false;
                if self.has_unsaved() {
                    self.pending_quit = true;
                } else {
                    self.close_requested = true;
                }
            }
            Action::Quit => {
                self.quit_all_windows = true;
                if self.has_unsaved() {
                    self.pending_quit = true;
                } else {
                    self.close_requested = true;
                }
            }
            Action::Find => {
                let (a, b) = self.char_selection(ctx);
                self.find.scope = (a < b).then_some((self.active_tab().doc.id, a, b));
                self.show_find = true;
                self.show_replace = false;
                self.find.focus_needle = true;
            }
            Action::FindNext => {
                if self.find.needle.is_empty() {
                    self.show_find = true;
                    self.find.focus_needle = true;
                } else {
                    self.find_next(false);
                }
            }
            Action::FindPrevious => {
                if self.find.needle.is_empty() {
                    self.show_find = true;
                    self.find.focus_needle = true;
                } else {
                    self.find_next(true);
                }
            }
            Action::Replace => {
                let (a, b) = self.char_selection(ctx);
                self.find.scope = (a < b).then_some((self.active_tab().doc.id, a, b));
                self.show_find = true;
                self.show_replace = true;
                self.find.focus_needle = true;
            }
            Action::Goto => {
                self.show_goto = true;
            }
            Action::InsertDate => self.insert_date(),
            Action::DeleteSelection => self.delete_selection(),
            Action::Font => {
                self.font_draft = (
                    self.font_family
                        .clone()
                        .unwrap_or_else(|| DEFAULT_FONT_LABEL.to_string()),
                    self.active_tab().doc.font_size,
                );
                self.show_font = true;
            }
            Action::SetEncoding(encoding) => {
                let tab = self.active_tab_mut();
                if tab.doc.encoding != encoding {
                    tab.doc.encoding = encoding;
                    tab.doc.dirty = true;
                }
                self.status = format!(
                    "Encodage à l’enregistrement : {}. Texte conservé.",
                    encoding.label()
                );
            }
            Action::ReopenEncoding(encoding) => {
                if self.active_tab().doc.dirty {
                    self.status = "Réouverture refusée : enregistrez le brouillon ou créez une copie avant de relire le fichier.".into();
                } else if self.active_tab().doc.raw.is_some() {
                    let tab = self.active_tab_mut();
                    tab.doc.reinterpret(encoding);
                    tab.history = History::new(&tab.doc.text);
                    tab.max_chars = tab
                        .doc
                        .text
                        .lines()
                        .map(|l| l.chars().count())
                        .max()
                        .unwrap_or(0);
                    self.status = format!("Fichier rouvert avec {}", encoding.label());
                }
            }
            Action::SetEol(eol) => {
                let doc = &mut self.active_tab_mut().doc;
                if doc.eol != eol {
                    doc.eol = eol;
                    doc.dirty = true;
                }
                self.status = format!("Fin de ligne : {}", eol.label());
            }
            Action::ToggleWrap => {
                let tab = self.active_tab_mut();
                tab.doc.wrap = !tab.doc.wrap;
            }
            Action::SetTheme(mode) => {
                self.theme = mode;
                self.status = format!("Thème : {}", mode.label());
            }
            Action::ToggleStatus => self.status_bar = !self.status_bar,
            Action::OpenRecent(path) => {
                if path.exists() {
                    self.open_requested(path);
                } else {
                    self.recent
                        .retain(|entry| !entry.eq_ignore_ascii_case(&path.to_string_lossy()));
                    self.status = format!("Fichier introuvable : {}", path.display());
                }
            }
            Action::ClearRecent => {
                session::clear_recent_list(&mut self.recent);
                self.status = "Liste des fichiers recents effacee.".to_string();
            }
            Action::CloseOthers(index) => self.close_others(index),
            Action::CloseToTheRight(index) => self.close_to_the_right(index),
            Action::CopyPath(index) => {
                if let Some(path) = self.tabs.get(index).and_then(|tab| tab.doc.path.clone()) {
                    ctx.copy_text(path.display().to_string());
                    self.status = "Chemin complet copie.".to_string();
                }
            }
            Action::RevealPath(index) => {
                if let Some(path) = self.tabs.get(index).and_then(|tab| tab.doc.path.clone()) {
                    #[cfg(windows)]
                    {
                        let argument = format!("/select,\"{}\"", path.display());
                        let _ = std::process::Command::new("explorer").arg(argument).spawn();
                    }
                    #[cfg(not(windows))]
                    {
                        if let Some(dir) = path.parent() {
                            let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
                        }
                    }
                }
            }
            Action::Zoom(delta) => {
                let tab = self.active_tab_mut();
                let next = (tab.doc.font_size + delta as f32 * 2.0).clamp(8.0, 60.0);
                tab.doc.font_size = next;
                self.default_font_size = next;
            }
            Action::ResetZoom => {
                self.active_tab_mut().doc.font_size = 15.0;
                self.default_font_size = 15.0;
            }
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::SelectAll => {
                self.select_all();
                // La selection vit dans `TextEditState` ; sans ce report le
                // champ de texte ne la verrait pas.
                self.apply_pending_selection(ctx);
            }
            Action::Cut => {}
            Action::Copy => {}
            Action::Paste => {}
            Action::ToggleMarkdown => {
                let tab = self.active_tab_mut();
                tab.doc.markdown = !tab.doc.markdown;
                let enabled = tab.doc.markdown;
                if !enabled {
                    self.show_markdown_preview = false;
                    self.active_tab_mut().workspace.visual_blocks = false;
                }
                self.status = if enabled {
                    "Mode Markdown activé.".to_string()
                } else {
                    "Mode Markdown désactivé.".to_string()
                };
            }
            Action::ToggleMarkdownPreview => {
                if !self.active_tab().doc.markdown {
                    self.active_tab_mut().doc.markdown = true;
                    self.show_format_bar = true;
                }
                self.show_markdown_preview = !self.show_markdown_preview;
            }
            Action::ToggleFormatBar => self.show_format_bar = !self.show_format_bar,
            Action::Format(kind) => self.format_selection(ctx, kind),
            Action::PageSetup => {
                self.page_draft = self.page.clone();
                self.show_page_setup = true;
            }
            Action::Print => self.print_active(ctx),
            Action::PrintPreview => {
                self.show_print_preview = true;
                self.preview_zoom = 1.0;
            }
            Action::About => self.show_about = true,
        }
        self.session_dirty = true;
    }

    fn shortcuts(&mut self, ctx: &egui::Context) -> Vec<Action> {
        use egui::Key;
        let mut actions = Vec::new();
        ctx.input_mut(|input| {
            let shift = egui::Modifiers {
                command: true,
                shift: true,
                ..Default::default()
            };
            for (id, binding) in self.preferences.shortcuts.clone() {
                if let Some((modifiers, key)) = settings::shortcut(&binding) {
                    if input.consume_key(modifiers, key) {
                        match id.as_str() {
                            "settings" => self.design.settings = true,
                            "palette" => self.open_palette(),
                            "search" => self.options.search = true,
                            "history" => self.options.history = true,
                            "focus" => self.options.concentration = !self.options.concentration,
                            _ => {}
                        }
                    }
                }
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::Comma) {
                self.design.settings = !self.design.settings;
            }
            if input.consume_key(shift, Key::P) {
                self.open_palette();
            }
            if input.consume_key(shift, Key::F) {
                self.options.search = true;
            }
            if input.consume_key(egui::Modifiers::NONE, Key::Escape) {
                self.options.concentration = false;
            }
        });
        // « Mise en page » est modal : tant qu'elle est ouverte, aucun
        // raccourci du document ne doit s'appliquer (Ctrl+A, Ctrl+Z, F5...).
        if self.show_page_setup {
            return actions;
        }
        // Un champ de saisie auxiliaire (Rechercher, Atteindre, en-tête...)
        // a le focus : la frappe lui appartient. Sans cette garde, Ctrl+A
        // sélectionnerait le document et lui rendrait le focus en pleine
        // saisie.
        let editor = Self::editor_id(self.active_tab().doc.id);
        if ctx.text_edit_focused() && ctx.memory(|memory| memory.focused()) != Some(editor) {
            return actions;
        }
        let markdown = self.active_tab().doc.markdown;
        ctx.input_mut(|input| {
            let command_shift = egui::Modifiers {
                command: true,
                shift: true,
                ..egui::Modifiers::NONE
            };
            let shift = egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            };
            if input.consume_key(command_shift, Key::N) {
                actions.push(Action::NewWindow);
            } else if input.consume_key(egui::Modifiers::COMMAND, Key::N) {
                actions.push(Action::New);
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::O) {
                actions.push(Action::Open(None));
            }
            if input.consume_key(command_shift, Key::S) {
                actions.push(Action::SaveAs);
            } else if input.consume_key(egui::Modifiers::COMMAND, Key::S) {
                actions.push(Action::Save);
            }
            if input.consume_key(command_shift, Key::W) {
                actions.push(Action::CloseWindow);
            } else if input.consume_key(egui::Modifiers::COMMAND, Key::W) {
                actions.push(Action::CloseTab);
            }
            if input.consume_key(shift, Key::F3) {
                actions.push(Action::FindPrevious);
            } else if input.consume_key(egui::Modifiers::NONE, Key::F3) {
                actions.push(Action::FindNext);
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::F) {
                actions.push(Action::Find);
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::H) {
                actions.push(Action::Replace);
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::G) {
                actions.push(Action::Goto);
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::Z) {
                actions.push(Action::Undo);
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::Y) {
                actions.push(Action::Redo);
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::A) {
                actions.push(Action::SelectAll);
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::Plus)
                || input.consume_key(egui::Modifiers::COMMAND, Key::Equals)
            {
                actions.push(Action::Zoom(1));
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::Minus) {
                actions.push(Action::Zoom(-1));
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::Num0) {
                actions.push(Action::ResetZoom);
            }
            if input.consume_key(egui::Modifiers::NONE, Key::F5) {
                actions.push(Action::InsertDate);
            }
            if input.consume_key(egui::Modifiers::COMMAND, Key::P) {
                actions.push(Action::Print);
            }
            if input.consume_key(command_shift, Key::M) {
                actions.push(Action::ToggleMarkdownPreview);
            }
            if markdown {
                if input.consume_key(egui::Modifiers::COMMAND, Key::B) {
                    actions.push(Action::Format(FormatKind::Bold));
                }
                if input.consume_key(egui::Modifiers::COMMAND, Key::I) {
                    actions.push(Action::Format(FormatKind::Italic));
                }
            }
        });
        actions
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        self.catalog_menu_bar(ui, actions);
    }

    fn recent_menu(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.menu_button("Fichiers récents", |ui| {
            if self.recent.is_empty() {
                ui.add_enabled(false, egui::Button::new("Aucun fichier récent"));
                return;
            }
            for (index, entry) in self.recent.iter().enumerate() {
                let path = PathBuf::from(entry);
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| entry.clone());
                let label = format!("{}  {name}", index + 1);
                if ui.button(label).on_hover_text(entry).clicked() {
                    actions.push(Action::OpenRecent(path.clone()));
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Effacer la liste").clicked() {
                actions.push(Action::ClearRecent);
                ui.close();
            }
        });
    }

    fn tab_strip(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        // Copie des informations necessaires : l'iteration ne doit pas tenir
        // un emprunt immutable de `self.tabs` pendant qu'on modifie l'etat.
        let tabs: Vec<(String, Option<PathBuf>)> = self
            .tabs
            .iter()
            .map(|tab| (tab.doc.title(), tab.doc.path.clone()))
            .collect();
        let dragging = self.dragging_tab;
        let active = self.active;

        let mut select: Option<usize> = None;
        let mut close: Option<usize> = None;
        let mut drag_started: Option<usize> = None;
        let mut rects: Vec<egui::Rect> = Vec::new();

        ui.horizontal_wrapped(|ui| {
            for (index, (title, path)) in tabs.iter().enumerate() {
                let response = ui.add(
                    egui::Button::new(title.clone())
                        .selected(index == active)
                        .sense(egui::Sense::click_and_drag()),
                );
                rects.push(response.rect);
                if response.clicked() {
                    select = Some(index);
                }
                if response.middle_clicked() {
                    close = Some(index);
                }
                if response.drag_started() {
                    drag_started = Some(index);
                }
                if dragging == Some(index) {
                    ui.painter().rect_stroke(
                        response.rect,
                        4.0,
                        ui.visuals().widgets.active.bg_stroke,
                        egui::StrokeKind::Inside,
                    );
                }
                if dragging.is_some_and(|from| from != index) {
                    if let Some(pointer) = ui.input(|input| input.pointer.interact_pos()) {
                        if response.rect.contains(pointer) {
                            ui.painter().rect_stroke(
                                response.rect,
                                4.0,
                                egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                }

                let has_path = path.is_some();
                let can_close_right = index + 1 < tabs.len();
                response.context_menu(|ui| {
                    if ui.button("Fermer").clicked() {
                        close = Some(index);
                        ui.close();
                    }
                    if ui.button("Fermer les autres").clicked() {
                        actions.push(Action::CloseOthers(index));
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            can_close_right,
                            egui::Button::new("Fermer les onglets à droite"),
                        )
                        .clicked()
                    {
                        actions.push(Action::CloseToTheRight(index));
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(has_path, egui::Button::new("Copier le chemin complet"))
                        .clicked()
                    {
                        actions.push(Action::CopyPath(index));
                        ui.close();
                    }
                    if ui
                        .add_enabled(has_path, egui::Button::new("Ouvrir l'emplacement"))
                        .clicked()
                    {
                        actions.push(Action::RevealPath(index));
                        ui.close();
                    }
                });
            }
            if ui.button("+").on_hover_text("Nouvel onglet").clicked() {
                select = Some(tabs.len());
            }
        });

        if let Some(index) = select {
            if index < self.tabs.len() {
                if self.active != index {
                    self.active = index;
                    self.pending_focus = true;
                }
            } else {
                self.new_tab();
            }
        }
        if let Some(index) = close {
            self.maybe_close_tab(index);
        }
        if let Some(index) = drag_started {
            self.dragging_tab = Some(index);
        }
        // Deposer l'onglet deplace sur la case survolee.
        if let Some(from) = self.dragging_tab {
            let released = ui.input(|input| input.pointer.any_released());
            if released {
                let target = ui
                    .input(|input| input.pointer.interact_pos())
                    .and_then(|pos| {
                        rects
                            .iter()
                            .position(|rect| rect.contains(pos))
                            .filter(|target| *target < self.tabs.len())
                    });
                if let Some(to) = target {
                    self.move_tab(from, to);
                }
                self.dragging_tab = None;
            }
        }
    }

    /// Deplace un onglet (glisser-deposer), en conservant l'onglet actif.
    fn move_tab(&mut self, from: usize, to: usize) {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        let active_id = self.tabs[self.active].doc.id;
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        self.active = self
            .tabs
            .iter()
            .position(|tab| tab.doc.id == active_id)
            .unwrap_or(0);
        self.session_dirty = true;
    }
}

impl eframe::App for NotepadApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.set_theme(self.theme.preference());
        ui_design::apply(&ctx);
        if let Some(name) = self.font_family.clone() {
            self.ensure_font(&ctx, &name);
        }

        let scale = ctx.zoom_factor();
        let bounds = ctx.input(|i| {
            let v = i.viewport();
            if v.maximized.unwrap_or(false) || v.minimized.unwrap_or(false) {
                return None;
            }
            let inner = v.inner_rect?;
            let outer = v.outer_rect?;
            Some([
                outer.min.x * scale,
                outer.min.y * scale,
                inner.width() * scale,
                inner.height() * scale,
            ])
        });
        if let Some(bounds) = bounds {
            if self.normal_window != Some(bounds) {
                self.normal_window = Some(bounds);
                self.session_dirty = true;
            }
        }
        // Fichiers deposes sur la fenetre.
        let dropped: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect()
        });
        for path in dropped {
            let is_image = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "gif" | "webp"
                    )
                })
                .unwrap_or(false);
            if self.active_tab().doc.moodboard && is_image {
                if self.active_tab().workspace.read_only {
                    self.status = "Import refusé : moodboard en lecture seule".into();
                    continue;
                }
                let n = self.active_tab().mood_items.len() as f32;
                self.active_tab_mut().mood_items.push(session::MoodItem {
                    text: String::new(),
                    image_path: Some(path.to_string_lossy().into_owned()),
                    x: 50.0 + n * 24.0,
                    y: 50.0 + n * 24.0,
                    width: 240.0,
                    height: 180.0,
                    color: 0,
                    link: None,
                    embedded_image: None,
                    ..Default::default()
                });
                self.active_tab_mut().doc.dirty = true;
                self.session_dirty = true;
            } else {
                self.open_path(path);
            }
        }

        // Fermeture de la fenetre : demande de confirmation si necessaire.
        if ctx.input(|input| input.viewport().close_requested()) {
            if self.design.capture.is_some() {
                return;
            }
            if self.close_confirmed {
                return;
            }
            if self.has_unsaved() {
                self.pending_quit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            } else if !self.has_unsaved() {
                self.save_session(&ctx);
            }
        }

        let mut actions = self.shortcuts(&ctx);
        self.horizontal_menu_ui(ui, &mut actions);

        if self.status_bar {
            self.status_ui(ui, &mut actions);
        }

        let mut typed = false;
        self.navigation_ui(ui);
        self.contextual_inspector(ui);
        self.trash_window(&ctx);
        egui::CentralPanel::default().show(ui, |ui| {
            self.document_header(ui, &ctx, &mut actions);
            if self.show_print_preview {
                self.print_preview_ui(ui);
            } else {
                if self.tabs[self.active].doc.moodboard {
                    self.moodboard_ui(ui, &ctx);
                    return;
                }
                typed = self.document_body(ui, &ctx, &mut actions);
            }
        });

        if typed {
            self.session_dirty = true;
        }
        if self.pending_focus {
            self.focus_active_cursor(&ctx);
        }

        // Fenetres flottantes.
        if self.show_find {
            let mut open = self.show_find;
            egui::Window::new(if self.show_replace {
                "Remplacer"
            } else {
                "Rechercher"
            })
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 96.0])
            .show(&ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Rechercher :");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.find.needle).desired_width(220.0),
                    );
                    if self.find.focus_needle {
                        response.request_focus();
                        self.find.focus_needle = false;
                    }
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.find_next(false);
                    }
                });
                if self.show_replace {
                    ui.horizontal(|ui| {
                        ui.label("Remplacer par :");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.find.replacement)
                                .desired_width(220.0),
                        );
                    });
                }
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.find.match_case, "Respecter la casse");
                    ui.checkbox(&mut self.find.wrap, "Boucler");
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.find.whole_words, "Mots entiers");
                    ui.checkbox(&mut self.find.regex, "Expression régulière");
                    ui.add_enabled(
                        self.find
                            .scope
                            .is_some_and(|(id, _, _)| id == self.active_tab().doc.id),
                        egui::Checkbox::new(
                            &mut self.find.selection_only,
                            "Dans la sélection initiale",
                        ),
                    );
                });
                match self.find_matches() {
                    Ok(found) => {
                        ui.label(format!(
                            "{} occurrence(s) — le remplacement est littéral",
                            found.len()
                        ));
                    }
                    Err(error) => {
                        ui.colored_label(egui::Color32::RED, error);
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("Suivant").clicked() {
                        self.find_next(false);
                    }
                    if ui.button("Precedent").clicked() {
                        self.find_next(true);
                    }
                    if self.show_replace {
                        if ui.button("Remplacer").clicked() {
                            self.replace_current();
                        }
                        if ui.button("Remplacer tout").clicked() {
                            self.replace_all();
                        }
                    }
                });
                if !self.find.status.is_empty() {
                    ui.label(&self.find.status);
                }
            });
            self.show_find = open;
            self.apply_pending_selection(&ctx);
        }

        if self.show_goto {
            let mut open = self.show_goto;
            egui::Window::new("Atteindre la ligne")
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Numero de ligne :");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.goto_line).desired_width(90.0),
                        );
                        response.request_focus();
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.goto_line();
                        }
                    });
                    if ui.button("Atteindre").clicked() {
                        self.goto_line();
                    }
                });
            self.show_goto = open;
            self.apply_pending_selection(&ctx);
        }

        if self.show_about {
            let mut open = self.show_about;
            egui::Window::new("A propos")
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .show(&ctx, |ui| {
                    ui.heading("Notepad RS 0.1.0");
                    ui.label("Reconstruction native en Rust (egui) des comportements observes");
                    ui.label("dans C:\\Windows\\System32\\notepad.exe via REA + Ghidra 12.1.4.");
                    ui.separator();
                    ui.label("Codages : ANSI, UTF-8, UTF-8 BOM, UTF-16 LE/BE");
                    ui.label("Fins de ligne : CRLF, LF, CR");
                    ui.label("Recherche / remplacement avec casse et bouclage");
                });
            self.show_about = open;
        }

        if self.show_font {
            let mut open = self.show_font;
            let mut apply = false;
            let mut cancel = false;
            {
                let draft = &mut self.font_draft;
                egui::Window::new("Police")
                    .open(&mut open)
                    .resizable(false)
                    .collapsible(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(&ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Police et style :");
                            egui::ComboBox::from_id_salt("font-family")
                                .selected_text(draft.0.clone())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut draft.0,
                                        DEFAULT_FONT_LABEL.to_string(),
                                        DEFAULT_FONT_LABEL,
                                    );
                                    for (name, _, _) in available_system_fonts() {
                                        ui.selectable_value(
                                            &mut draft.0,
                                            name.to_string(),
                                            name.as_str(),
                                        );
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Taille :");
                            ui.add(
                                egui::DragValue::new(&mut draft.1)
                                    .range(8.0..=60.0)
                                    .suffix(" px"),
                            );
                        });
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("OK").clicked() {
                                apply = true;
                            }
                            if ui.button("Annuler").clicked() {
                                cancel = true;
                            }
                        });
                    });
            }
            if apply {
                let (chosen, size) = self.font_draft.clone();
                self.font_family = if chosen == DEFAULT_FONT_LABEL {
                    None
                } else {
                    Some(chosen.clone())
                };
                self.active_tab_mut().doc.font_size = size;
                // Font size is a page override; new-page defaults live in Settings.
                self.session_dirty = true;
                if let Some(name) = self.font_family.clone() {
                    self.ensure_font(&ctx, &name);
                }
                self.status = format!("Police : {chosen}, {size} px");
                open = false;
            }
            if cancel {
                open = false;
            }
            self.show_font = open;
        }

        self.page_setup_window(&ctx);
        self.options_ui(&ctx);

        self.close_dialogs_ui(&ctx);

        // Couper / Copier / Coller depuis le menu : on rejoue le raccourci
        // natif gere par le widget de texte.
        for action in &actions {
            match action {
                Action::Cut => inject_shortcut(&ctx, egui::Key::X, true),
                Action::Copy => inject_shortcut(&ctx, egui::Key::C, true),
                Action::Paste => inject_shortcut(&ctx, egui::Key::V, true),
                _ => {}
            }
        }

        let actions: Vec<Action> = actions
            .into_iter()
            .filter(|action| !matches!(action, Action::Cut | Action::Copy | Action::Paste))
            .collect();
        for action in actions {
            self.handle_action(&ctx, action);
        }

        self.capture_preview(&ctx);
        if self.design.capture.is_none() && !self.close_confirmed && !self.pending_quit {
            self.sync_workspace(&ctx);
            self.scheduled_backup(&ctx);
        }
        if self.close_requested {
            self.close_requested = false;
            if !self.close_confirmed {
                self.save_session(&ctx);
            }
            if self.quit_all_windows {
                close_other_instances();
                self.quit_all_windows = false;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.session_dirty && self.last_session_save.elapsed() >= Duration::from_secs(3) {
            // Autosauvegarde, comme la tache periodique du Bloc-notes.
            self.save_session(&ctx);
        }

        let active = self.active_tab();
        let name = ui_design::page_title(active);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!("{name} - Notepad RS")));
    }
}

fn wiki_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else { break };
        let target = rest[..end].trim();
        if !target.is_empty() && !targets.iter().any(|existing| existing == target) {
            targets.push(target.to_owned());
        }
        rest = &rest[end + 2..];
    }
    targets
}

fn resolve_page(tabs: &[Tab], target: &str) -> Option<usize> {
    if let Some(stable) = target.strip_prefix("id:") {
        let uid = stable.split('|').next().unwrap_or(stable);
        return tabs
            .iter()
            .position(|t| !t.workspace.trashed && t.workspace.uid == uid);
    }
    let matches = |tab: &&Tab| {
        !tab.workspace.trashed
            && (tab.page_title == target
                || (tab.page_title.is_empty() && tab.doc.title() == target))
    };
    let exact: Vec<usize> = tabs
        .iter()
        .enumerate()
        .filter(|(_, t)| matches(t))
        .map(|(i, _)| i)
        .collect();
    if exact.len() == 1 {
        return Some(exact[0]);
    }
    if !exact.is_empty() {
        return None;
    }
    let aliases: Vec<usize> = tabs
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.workspace.trashed && t.workspace.aliases.iter().any(|a| a == target))
        .map(|(i, _)| i)
        .collect();
    if aliases.len() == 1 {
        Some(aliases[0])
    } else {
        None
    }
}

#[cfg(test)]
mod workspace_tests {
    use super::*;

    #[test]
    fn session_save_with_native_window_geometry_does_not_reenter_context_lock() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let path = std::env::temp_dir().join(format!(
            "notepad-rs-native-geometry-{}.json",
            workspace_io::uid()
        ));
        let mut app = NotepadApp::new(&cc, None, path.clone());
        app.active_tab_mut().doc.text = "Brouillon à conserver".into();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .inner_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1200.0, 800.0),
        ));
        let mut output = ctx.run_ui(input, |_| {
            let scale = ctx.zoom_factor();
            let expected = ctx.input(|input| {
                let rect = input.viewport().inner_rect.unwrap();
                [rect.width() * scale, rect.height() * scale]
            });
            app.save_session(&ctx);
            let saved = Session::load_from(&path);
            assert_eq!(saved.window, Some(expected));
            assert_eq!(saved.tabs[0].text.as_deref(), Some("Brouillon à conserver"));
        });
        output.textures_delta.clear();
        std::fs::remove_file(&path).unwrap();
        let backup = path.with_extension("json.bak");
        if backup.exists() {
            std::fs::remove_file(backup).unwrap();
        }
    }

    #[test]
    fn two_instances_sync_concurrent_edits_without_losing_either_text() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut base = Session::default();
        let mut page = session::SessionTab::default();
        page.page_id = 1;
        page.text = Some("Base".into());
        page.workspace.uid = "shared-stable-id".into();
        base.tabs.push(page);
        let dir =
            std::env::temp_dir().join(format!("notepad-rs-sync-test-{}", workspace_io::uid()));
        let mut a = NotepadApp::new(&cc, Some(base.clone()), dir.join("unused-a.json"));
        let mut b = NotepadApp::new(&cc, Some(base), dir.join("unused-b.json"));
        for app in [&mut a, &mut b] {
            app.sync_directory = Some(dir.to_string_lossy().into_owned());
            app.last_sync = Instant::now() - Duration::from_secs(6);
            app.sync_workspace(&ctx);
        }
        a.active_tab_mut().doc.text = "Modification A".into();
        b.active_tab_mut().doc.text = "Modification B".into();
        for app in [&mut a, &mut b] {
            app.last_sync = Instant::now() - Duration::from_secs(6);
            app.sync_workspace(&ctx);
        }
        a.last_sync = Instant::now() - Duration::from_secs(6);
        a.sync_workspace(&ctx);
        let remote = workspace_io::read(&dir.join("notepad-rs-workspace.json")).unwrap();
        assert!(remote
            .tabs
            .iter()
            .any(|p| p.text.as_deref() == Some("Modification A")));
        assert!(remote
            .tabs
            .iter()
            .any(|p| p.text.as_deref() == Some("Modification B")));
        assert_eq!(remote.tabs.len(), 2);
        assert_eq!(a.tabs.len(), 2);
        for entry in std::fs::read_dir(&dir).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }
        std::fs::remove_dir(dir).unwrap();
    }

    fn mood_frame(
        app: &mut NotepadApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1100.0, 800.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            app.contextual_inspector(ui);
            app.moodboard_ui(ui, ctx);
        });
        output.textures_delta.clear();
        output
    }

    fn label_position(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + egui::vec2(5.0, 5.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing label: {label}"))
    }

    fn pointer(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        }
    }

    #[test]
    fn moodboard_selection_duplicate_delete_and_drag_survive_restore() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = NotepadApp::new(
            &cc,
            None,
            std::env::temp_dir().join("unused-notepad-rs-test-session.json"),
        );
        app.new_moodboard();
        mood_frame(&mut app, &ctx, vec![]);
        let output = mood_frame(&mut app, &ctx, vec![]);
        let header = label_position(&output, "Déplacer");
        mood_frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(header), pointer(header, true)],
        );
        mood_frame(&mut app, &ctx, vec![pointer(header, false)]);
        assert_eq!(app.selected_mood_item, Some(0));
        let output = mood_frame(&mut app, &ctx, vec![]);
        let duplicate = label_position(&output, "Dupliquer");
        mood_frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(duplicate),
                pointer(duplicate, true),
            ],
        );
        mood_frame(&mut app, &ctx, vec![pointer(duplicate, false)]);
        assert_eq!(app.active_tab().mood_items.len(), 2);
        let output = mood_frame(&mut app, &ctx, vec![]);
        let delete = label_position(&output, "Supprimer");
        mood_frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(delete), pointer(delete, true)],
        );
        mood_frame(&mut app, &ctx, vec![pointer(delete, false)]);
        assert_eq!(app.active_tab().mood_items.len(), 1);
        let output = mood_frame(&mut app, &ctx, vec![]);
        let header = label_position(&output, "Déplacer");
        let before = app.active_tab().mood_items[0].x;
        mood_frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(header), pointer(header, true)],
        );
        let end = header + egui::vec2(50.0, 30.0);
        mood_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        mood_frame(&mut app, &ctx, vec![pointer(end, false)]);
        let moved = app.active_tab().mood_items[0].x;
        assert!(moved > before);
        let snapshot = app.session_snapshot(&ctx);
        let restored = NotepadApp::new(
            &cc,
            Some(snapshot),
            std::env::temp_dir().join("unused-notepad-rs-test-session.json"),
        );
        assert_eq!(restored.active_tab().mood_items[0].x, moved);
    }

    #[test]
    fn stable_links_and_old_names_survive_page_rename() {
        let mut tab = Tab::new(1);
        tab.workspace.uid = "stable".into();
        tab.page_title = "Nouveau nom".into();
        tab.workspace.aliases.push("Ancien nom".into());
        let tabs = vec![tab];
        assert_eq!(resolve_page(&tabs, "id:stable|Ancien nom"), Some(0));
        assert_eq!(resolve_page(&tabs, "Ancien nom"), Some(0));
        assert_eq!(resolve_page(&tabs, "Nouveau nom"), Some(0));
    }

    #[test]
    fn ambiguous_names_and_trashed_targets_are_not_resolved() {
        let mut a = Tab::new(1);
        a.page_title = "Même nom".into();
        let mut b = Tab::new(2);
        b.page_title = "Même nom".into();
        let mut tabs = vec![a, b];
        assert_eq!(resolve_page(&tabs, "Même nom"), None);
        tabs[1].workspace.trashed = true;
        assert_eq!(resolve_page(&tabs, "Même nom"), Some(0));
        tabs[0].workspace.trashed = true;
        assert_eq!(resolve_page(&tabs, "Même nom"), None);
    }

    #[test]
    fn links_trim_deduplicate_and_ignore_unclosed_targets() {
        assert_eq!(
            wiki_targets("[[ Projet ]] [[Projet]] [[]] [[Réunion]] [[incomplet"),
            vec!["Projet", "Réunion"]
        );
    }

    #[test]
    fn workspace_metadata_survives_session_roundtrip() {
        let mut page = session::SessionTab::default();
        page.page_id = 42;
        page.parent_id = Some(7);
        page.page_title = "Projet".into();
        page.workspace.favorite = true;
        page.workspace.trashed = true;
        page.workspace.versions.push("Avant modification".into());
        let bytes = serde_json::to_vec(&page).unwrap();
        let restored: session::SessionTab = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored.parent_id, Some(7));
        assert_eq!(restored.page_title, "Projet");
        assert!(restored.workspace.favorite && restored.workspace.trashed);
        assert_eq!(restored.workspace.versions, vec!["Avant modification"]);
    }

    #[test]
    fn html_export_renders_markup_and_escapes_injected_html() {
        let html = markdown::to_html("# Bonjour\n\n<script>alert(1)</script>\n\n**gras**");
        assert!(html.contains("<h1>Bonjour</h1>"));
        assert!(html.contains("<strong>gras</strong>"));
        assert!(!html.contains("<script>"));
    }
}

fn inject_shortcut(ctx: &egui::Context, key: egui::Key, command: bool) {
    let modifiers = if command {
        egui::Modifiers::COMMAND
    } else {
        egui::Modifiers::NONE
    };
    ctx.input_mut(|input| {
        input.events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        });
        input.events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        });
    });
}

fn count_chars(text: &str, byte_index: usize) -> usize {
    text[..byte_index].chars().count()
}

fn char_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(offset, _)| offset)
        .unwrap_or(text.len())
}

/// Demande la fermeture des autres fenetres Notepad RS (« Quitter »).
///
/// Chaque fenetre etant un processus distinct, on poste un `WM_CLOSE` aux
/// fenetres dont l'image executable est la notre : chaque instance affiche
/// donc son propre dialogue « Enregistrer les modifications ? ».
#[cfg(windows)]
fn close_other_instances() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, TRUE};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE,
    };

    struct Search {
        own_image: Option<String>,
        self_pid: u32,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` pointe vers le `Search` vivant de l'appelant, et
        // le callback ne vit pas au-dela de `EnumWindows`.
        let search = unsafe { &*(lparam as *const Search) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid == 0 || pid == search.self_pid {
            return TRUE;
        }
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() {
            return TRUE;
        }
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        let ok =
            unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) };
        unsafe { CloseHandle(process) };
        if ok == 0 {
            return TRUE;
        }
        let image = OsString::from_wide(&buffer[..length as usize])
            .to_string_lossy()
            .to_lowercase();
        if search.own_image.as_deref() == Some(image.as_str()) {
            unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
        }
        TRUE
    }

    let own_image = std::env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().to_lowercase());
    let search = Search {
        own_image,
        self_pid: std::process::id(),
    };
    unsafe { EnumWindows(Some(visit), &search as *const Search as LPARAM) };
}

#[cfg(not(windows))]
fn close_other_instances() {}
