//! Notepad RS — reconstruction native des comportements observes de Notepad.
//!
//! Les preuves ayant servi a la reconstruction proviennent de REA (Ghidra
//! 12.1.4) sur `C:\Windows\System32\notepad.exe` : imports USER32/GDI32/
//! COMCTL32, `IsTextUnicode`, et les valeurs de registre `lfFaceName`,
//! `iPointSize`, `fWrap`, `fWrapAround`, `DefaultFonts`, `Autosave`.

// Pas de fenetre de console : l'application est une fenetre Win32 classique.
// L'attribut est desactive pour les tests afin de garder leur sortie standard.
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod doc;
mod encoding;
mod find;
mod format;
mod header;
mod markdown;
mod page;
mod print;
mod session;

use std::path::PathBuf;
use std::time::{Duration, Instant};

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
    for arg in std::env::args_os().skip(1) {
        let text = arg.to_string_lossy().to_string();
        if text == "--new-window" || text == "--no-restore" {
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
    let size = saved
        .window
        .filter(|size| size[0] >= 460.0 && size[1] >= 300.0)
        .map(clamp_window_size)
        .unwrap_or([920.0, 660.0]);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_min_inner_size([460.0, 300.0])
            .with_title("Notepad RS")
            .with_drag_and_drop(true),
        ..Default::default()
    };
    let session = if restore { Some(saved) } else { None };
    eframe::run_native(
        "Notepad RS",
        options,
        Box::new(move |cc| {
            let mut app = NotepadApp::new(cc, session.clone(), session_file.clone());
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
        }
    }

    fn open(id: u64, path: PathBuf, bytes: Vec<u8>) -> Result<Tab, String> {
        let doc = Doc::from_bytes(id, path, bytes);
        let max_chars = doc.text.lines().map(|l| l.chars().count()).max().unwrap_or(0);
        Ok(Tab {
            history: History::new(&doc.text),
            doc,
            max_chars,
            mood_items: Vec::new(),
            page_parent: None,
            page_title: String::new(),
        })
    }

    fn touch(&mut self) {
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

/// Polices systeme proposées par la boîte « Police » (fichier dans
/// `C:\Windows\Fonts`, chargé à la demande).
const SYSTEM_FONTS: &[(&str, &str)] = &[
    ("Segoe UI", "segoeui.ttf"),
    ("Segoe UI Variable", "SegUIVar.ttf"),
    ("Arial", "arial.ttf"),
    ("Calibri", "calibri.ttf"),
    ("Consolas", "consola.ttf"),
    ("Courier New", "cour.ttf"),
    ("Georgia", "georgia.ttf"),
    ("Lucida Console", "lucon.ttf"),
    ("Tahoma", "tahoma.ttf"),
    ("Times New Roman", "times.ttf"),
    ("Verdana", "verdana.ttf"),
];

fn windows_fonts_dir() -> PathBuf {
    PathBuf::from(std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into()))
        .join("Fonts")
}

/// Polices reellement presentes sur la machine.
fn available_system_fonts() -> Vec<(&'static str, PathBuf)> {
    let dir = windows_fonts_dir();
    SYSTEM_FONTS
        .iter()
        .filter_map(|(name, file)| {
            let path = dir.join(file);
            path.exists().then_some((*name, path))
        })
        .collect()
}

struct NotepadApp {
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
    /// Identifiants des documents en attente de confirmation de fermeture.
    pending_close: Vec<u64>,
    /// Confirmation avant une conversion ANSI avec perte.
    pending_ansi_save: Option<PathBuf>,
    session_file: PathBuf,
    /// Placer le curseur dans l'editeur au prochain affichage (restauration de
    /// session, changement d'onglet).
    pending_focus: bool,
    /// Liste des fichiers recents (chemins complets, plus recent en tete).
    recent: Vec<String>,
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
}

impl NotepadApp {
    fn new(cc: &eframe::CreationContext<'_>, session: Option<Session>, session_file: PathBuf) -> NotepadApp {
        cc.egui_ctx.set_theme(egui::ThemePreference::System);
        let mut app = NotepadApp {
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
            theme: ThemeMode::System,
            status_bar: true,
            font_family: None,
            loaded_fonts: std::collections::HashSet::new(),
            pending_quit: false,
            close_requested: false,
            pending_close: Vec::new(),
            pending_ansi_save: None,
            session_file,
            pending_focus: false,
            recent: Vec::new(),
            dragging_tab: None,
            default_wrap: true,
            default_font_size: 15.0,
            session_dirty: false,
            last_session_save: Instant::now(),
            status: String::new(),
            show_markdown_preview: false,
            show_format_bar: true,
            page: page::PageSetup::default(),
            show_page_setup: false,
            show_print_preview: false,
            preview_zoom: 1.0,
            bold_default: false,
            bold_fonts: std::collections::HashSet::new(),
            font_draft: (DEFAULT_FONT_LABEL.to_string(), 15.0),
            page_draft: page::PageSetup::default(),
            last_selection: None,
        };
        if let Some(session) = session {
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
            app.recent = session.recent;
            app.page = session.page.sanitized();
            for saved_tab in session.tabs {
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
        }
        app.register_bold_default(&cc.egui_ctx);
        app
    }

    /// Reconstruit un onglet depuis la session : texte autosauvegarde s'il
    /// existe, sinon relecture du fichier sur disque.
    fn restore_tab(&mut self, saved: session::SessionTab) -> Option<Tab> {
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
                let max_chars = doc.text.lines().map(|l| l.chars().count()).max().unwrap_or(0);
                Tab {
                    history: History::new(&doc.text),
                    doc,
                    max_chars,
                    mood_items: Vec::new(),
            page_parent: None,
            page_title: String::new(),
                }
            }
            (None, None) => return None,
        };
        tab.doc.moodboard = saved.moodboard;
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
        let id = self.next_id;
        self.next_id += 1;
        let mut tab = Tab::new(id);
        tab.doc.wrap = self.default_wrap;
        tab.doc.font_size = self.default_font_size;
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
            tab.doc.path.is_none() && tab.doc.text.is_empty() && !tab.doc.dirty && self.tabs.len() > 0
        };
        let id = if reuse {
            self.tabs[self.active].doc.id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            id
        };
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
    fn remember_recent(&mut self, path: &std::path::Path) {
        session::touch_recent_list(&mut self.recent, path);
        self.session_dirty = true;
    }

    fn save_active(&mut self, save_as: bool) {
        let tab = self.active_tab_mut();
        let target = if save_as || tab.doc.path.is_none() {
            rfd::FileDialog::new()
                .set_title("Enregistrer sous")
                .add_filter("Documents texte", &["txt", "log", "md", "csv", "json"])
                .add_filter("Tous les fichiers", &["*"])
                .save_file()
        } else {
            tab.doc.path.clone()
        };
        let Some(path) = target else { return };
        if tab.doc.encoding.loses_information(&tab.doc.text) {
            self.pending_ansi_save = Some(path);
            return;
        }
        self.write_active_to(path);
    }

    fn write_active_to(&mut self, path: PathBuf) {
        let tab = self.active_tab_mut();
        let bytes = tab.doc.bytes_for_save();
        match std::fs::write(&path, bytes) {
            Ok(()) => {
                tab.doc.path = Some(path.clone());
                tab.doc.dirty = false;
                tab.doc.raw = std::fs::read(&path).ok();
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
            let _ = std::process::Command::new(exe)
                .arg("--new-window")
                .spawn();
        }
    }

    /// Charge une police systeme dans egui (une seule fois par nom).
    fn ensure_font(&mut self, ctx: &egui::Context, name: &str) {
        if self.loaded_fonts.contains(name) {
            return;
        }
        let Some((_, path)) = available_system_fonts()
            .into_iter()
            .find(|(font_name, _)| *font_name == name)
        else {
            return;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let family = egui::FontFamily::Name(name.into());
        ctx.add_font(egui::FontInsert::new(
            name,
            egui::FontData::from_owned(bytes),
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
        let tabs = self
            .tabs
            .iter()
            .filter(|tab| {
                // Un onglet vierge sans nom n'a rien a restaurer.
                tab.doc.path.is_some() || tab.doc.dirty || !tab.doc.text.is_empty()
            })
            .map(|tab| session::SessionTab {
                path: tab
                    .doc
                    .path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                text: tab.doc.dirty.then(|| tab.doc.text.clone()),
                encoding: session::encoding_code(tab.doc.encoding),
                eol: session::eol_code(tab.doc.eol),
                wrap: tab.doc.wrap,
                font_size: tab.doc.font_size,
                cursor: tab.doc.cursor_char,
                dirty: tab.doc.dirty,
                markdown: tab.doc.markdown,
                moodboard: tab.doc.moodboard,
                mood_items: tab.mood_items.clone(),
            })
            .collect();
        let window = ctx.input(|input| {
            input
                .viewport()
                .inner_rect
                .map(|rect| [rect.width(), rect.height()])
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
            window,
            recent: self.recent.clone(),
            page: self.page.clone(),
        }
    }

    fn save_session(&mut self, ctx: &egui::Context) {
        let snapshot = self.session_snapshot(ctx);
        let _ = snapshot.save_to(&self.session_file);
        self.session_dirty = false;
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
        let font = self.editor_font(self.tabs[index].doc.font_size);
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
                .id(editor_id)
                .font(font.clone())
                .desired_width(desired)
                .desired_rows(30)
                .lock_focus(true)
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
    fn moodboard_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let mut remove = None; let mut duplicate = None; let mut export = false;
        ui.horizontal(|ui| {
            ui.heading("Moodboard");
            if ui.button("+ Note").clicked() { let n=self.active_tab().mood_items.len() as f32; self.active_tab_mut().mood_items.push(session::MoodItem{text:"Nouvelle note".into(),image_path:None,x:40.0+n*24.0,y:40.0+n*24.0,width:190.0,height:130.0,color:(n as u8)%4,link:None}); self.selected_mood_item=Some(self.active_tab().mood_items.len()-1); }
            if ui.button("Supprimer").clicked(){remove=self.selected_mood_item;} if ui.button("Dupliquer").clicked(){duplicate=self.selected_mood_item;} if ui.button("Exporter").clicked(){export=true;}
        });
        if export { let path=std::env::current_dir().unwrap_or_default().join("moodboard_export.json"); if let Ok(data)=serde_json::to_vec_pretty(&self.active_tab().mood_items){if std::fs::write(&path,data).is_ok(){self.status=format!("Moodboard exporté : {}",path.display());}}}
        if let Some(i)=remove {if i<self.active_tab().mood_items.len(){self.active_tab_mut().mood_items.remove(i);self.selected_mood_item=None;self.active_tab_mut().doc.dirty=true;self.session_dirty=true;}}
        if let Some(i)=duplicate {if let Some(mut item)=self.active_tab().mood_items.get(i).cloned(){item.x+=24.0;item.y+=24.0;self.active_tab_mut().mood_items.push(item);self.selected_mood_item=Some(self.active_tab().mood_items.len()-1);self.active_tab_mut().doc.dirty=true;self.session_dirty=true;}}
        let (response, _)=ui.allocate_painter(ui.available_size(),egui::Sense::hover()); let tab_id=self.active_tab().doc.id; let selected=self.selected_mood_item; let tab=self.active_tab_mut(); let mut clicked_item = None;
        for (index,item) in tab.mood_items.iter_mut().enumerate(){let id=egui::Id::new(("mood-item",tab_id,index));let shown=egui::Area::new(id).fixed_pos(response.rect.min+egui::vec2(item.x,item.y)).show(ctx,|ui|{
            let fill=match item.color{1=>egui::Color32::from_rgb(220,240,180),2=>egui::Color32::from_rgb(190,225,250),3=>egui::Color32::from_rgb(245,205,220),_=>egui::Color32::from_rgb(250,235,160)};let stroke=if selected==Some(index){egui::Stroke::new(3.0,egui::Color32::from_rgb(40,100,220))}else{egui::Stroke::NONE};
            egui::Frame::NONE.fill(fill).stroke(stroke).inner_margin(egui::Margin::same(10)).show(ui,|ui|{
                let drag=ui.allocate_response(egui::vec2(ui.available_width(),18.0),egui::Sense::drag());ui.label("↕ déplacer");if drag.dragged(){item.x+=drag.drag_delta().x;item.y+=drag.drag_delta().y;}
                ui.horizontal(|ui|{ui.label("Couleur");egui::ComboBox::from_id_salt(("mood-color",tab_id,index)).selected_text(item.color.to_string()).show_ui(ui,|ui|{for c in 0..4u8{ui.selectable_value(&mut item.color,c,c.to_string());}});});
                if let Some(path)=&item.image_path{if let Ok(bytes)=std::fs::read(path){if let Ok(image)=image::load_from_memory(&bytes){let rgba=image.to_rgba8();let color=egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize,rgba.height() as usize],rgba.as_raw());let texture=ctx.load_texture(format!("mood-image-{tab_id}-{index}"),color,egui::TextureOptions::LINEAR);ui.image((texture.id(),egui::vec2(item.width-20.0,item.height-65.0).max(egui::vec2(40.0,40.0))));}}}
                ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut item.width).range(80.0..=1200.0));ui.add(egui::DragValue::new(&mut item.height).range(60.0..=900.0));});ui.add(egui::TextEdit::multiline(&mut item.text).desired_width(item.width-20.0).desired_rows(3));ui.text_edit_singleline(item.link.get_or_insert_with(String::new));if item.link.as_deref()==Some(""){item.link=None;}if let Some(link)=item.link.as_deref(){ui.hyperlink_to("Ouvrir le lien",link);}
            });});if shown.response.clicked(){clicked_item=Some(index);}}
        if clicked_item.is_some(){self.selected_mood_item=clicked_item;}
    }

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
            ui.label(format!("Zoom {} %", (self.preview_zoom * 100.0).round() as i32));
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
                    let rect = rect.translate(egui::vec2(((available.x - size.x) / 2.0).max(0.0), 0.0));
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
                        let y = measured.text_top_pt()
                            + line_index as f32 * measured.line_height_pt;
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
                ui.add(egui::Button::new(egui::RichText::new(label).monospace()).min_size(egui::vec2(24.0, 22.0)))
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
                        egui::RichText::new(
                            "&f fichier · &p page · &P total · &d date · &t heure",
                        )
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

    fn find_next(&mut self, backwards: bool) {
        let needle = self.find.needle.clone();
        if needle.is_empty() {
            self.find.status = "Saisissez un texte a rechercher.".to_string();
            return;
        }
        let match_case = self.find.match_case;
        let wrap = self.find.wrap;
        let (text, from, selection) = {
            let tab = self.active_tab();
            let (start, end) = tab
                .doc
                .pending_selection
                .unwrap_or((0, 0));
            let _ = (start, end);
            (tab.doc.text.clone(), char_to_byte(&tab.doc.text, tab.doc.cursor_char), tab.doc.selection_chars)
        };
        let _ = selection;
        let found = if backwards {
            find::find_previous(&text, &needle, from, match_case, wrap)
        } else {
            find::find_next(&text, &needle, from, match_case, wrap)
        };
        match found {
            Some(found) => {
                let chars_before = count_chars(&text, found.start);
                let chars_len = count_chars(&text[found.start..found.end], 0);
                let tab = self.active_tab_mut();
                tab.doc.pending_selection = Some((chars_before, chars_before + chars_len));
                tab.doc.cursor_char = chars_before + chars_len;
                self.find.status = "1 occurrence selectionnee.".to_string();
            }
            None => self.find.status = "Impossible de trouver le texte.".to_string(),
        }
    }

    fn replace_current(&mut self) {
        let needle = self.find.needle.clone();
        let replacement = self.find.replacement.clone();
        if needle.is_empty() {
            return;
        }
        let (start, end) = {
            let tab = self.active_tab();
            match tab.doc.pending_selection {
                Some(range) if range.1 > range.0 => range,
                _ => {
                    self.find.status = "Aucune selection a remplacer.".to_string();
                    return;
                }
            }
        };
        let match_case = self.find.match_case;
        let tab = self.active_tab_mut();
        let bytes: Vec<usize> = byte_offsets(&tab.doc.text).collect();
        if let (Some(&byte_start), Some(&byte_end)) = (bytes.get(start), bytes.get(end)) {
            let selected = &tab.doc.text[byte_start..byte_end];
            let same = if match_case {
                selected == needle
            } else {
                selected.eq_ignore_ascii_case(&needle)
            };
            if same {
                tab.doc.text.replace_range(byte_start..byte_end, &replacement);
                tab.touch();
                tab.doc.pending_selection = Some((start, start + replacement.chars().count()));
            }
        }
        self.find_next(false);
    }

    fn replace_all(&mut self) {
        let needle = self.find.needle.clone();
        let replacement = self.find.replacement.clone();
        if needle.is_empty() {
            return;
        }
        let match_case = self.find.match_case;
        let tab = self.active_tab_mut();
        let (new_text, count) = find::replace_all(&tab.doc.text, &needle, &replacement, match_case);
        if count > 0 {
            tab.doc.text = new_text;
            tab.touch();
        }
        self.find.status = format!("{count} remplacement(s).");
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
    fn format_lines(
        &mut self,
        ctx: &egui::Context,
        start: usize,
        end: usize,
        kind: FormatKind,
    ) {
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
        match action {
            Action::New => self.new_tab(),
            Action::NewMoodboard => self.new_moodboard(),
            Action::NewWindow => Self::spawn_new_window(),
            Action::Open(path) => {
                let path = path.or_else(|| {
                    rfd::FileDialog::new()
                        .set_title("Ouvrir")
                        .add_filter("Documents texte", &["txt", "log", "md", "csv", "json", "ini"])
                        .add_filter("Tous les fichiers", &["*"])
                        .pick_file()
                });
                if let Some(path) = path {
                    self.open_path(path);
                }
            }
            Action::Save => self.save_active(false),
            Action::SaveAs => self.save_active(true),
            Action::SaveAll => self.save_all(),
            Action::CloseTab => self.maybe_close_tab(self.active),
            Action::CloseWindow => {
                if self.has_unsaved() {
                    self.pending_quit = true;
                } else {
                    self.close_requested = true;
                }
            }
            Action::Quit => {
                close_other_instances();
                if self.has_unsaved() {
                    self.pending_quit = true;
                } else {
                    self.close_requested = true;
                }
            }
            Action::Find => {
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
                tab.doc.reinterpret(encoding);
                tab.max_chars = tab.doc.text.lines().map(|l| l.chars().count()).max().unwrap_or(0);
                tab.history = History::new(&tab.doc.text);
                self.status = format!("Codage : {}", encoding.label());
            }
            Action::SetEol(eol) => {
                self.active_tab_mut().doc.eol = eol;
                self.status = format!("Fin de ligne : {}", eol.label());
            }
            Action::ToggleWrap => {
                let tab = self.active_tab_mut();
                tab.doc.wrap = !tab.doc.wrap;
                self.default_wrap = tab.doc.wrap;
            }
            Action::SetTheme(mode) => {
                self.theme = mode;
                self.status = format!("Thème : {}", mode.label());
            }
            Action::ToggleStatus => self.status_bar = !self.status_bar,
            Action::OpenRecent(path) => {
                if path.exists() {
                    self.open_path(path);
                } else {
                    self.recent.retain(|entry| !entry.eq_ignore_ascii_case(&path.to_string_lossy()));
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
        let wrap = self.active_tab().doc.wrap;
        let theme = self.theme;
        let status_bar = self.status_bar;
        let markdown = self.active_tab().doc.markdown;
        let markdown_preview = self.show_markdown_preview;
        let format_bar = self.show_format_bar;
        ui.horizontal(|ui| {
            ui.menu_button("Fichier", |ui| {
                if ui.button("Nouvel onglet       Ctrl+N").clicked() {
                    actions.push(Action::New);
                    ui.close();
                }
                if ui.button("Nouveau moodboard").clicked() {
                    actions.push(Action::NewMoodboard);
                    ui.close();
                }
                if ui.button("Nouvelle fenêtre    Ctrl+Maj+N").clicked() {
                    actions.push(Action::NewWindow);
                    ui.close();
                }
                if ui.button("Ouvrir...           Ctrl+O").clicked() {
                    actions.push(Action::Open(None));
                    ui.close();
                }
                self.recent_menu(ui, actions);
                ui.separator();
                if ui.button("Enregistrer         Ctrl+S").clicked() {
                    actions.push(Action::Save);
                    ui.close();
                }
                if ui.button("Enregistrer sous    Ctrl+Maj+S").clicked() {
                    actions.push(Action::SaveAs);
                    ui.close();
                }
                if ui.button("Tout enregistrer").clicked() {
                    actions.push(Action::SaveAll);
                    ui.close();
                }
                ui.separator();
                if ui.button("Mise en page...").clicked() {
                    actions.push(Action::PageSetup);
                    ui.close();
                }
                if ui.button("Imprimer...        Ctrl+P").clicked() {
                    actions.push(Action::Print);
                    ui.close();
                }
                if ui.button("Aperçu avant impression").clicked() {
                    actions.push(Action::PrintPreview);
                    ui.close();
                }
                ui.separator();
                if ui.button("Fermer l'onglet     Ctrl+W").clicked() {
                    actions.push(Action::CloseTab);
                    ui.close();
                }
                if ui.button("Fermer la fenêtre   Ctrl+Maj+W").clicked() {
                    actions.push(Action::CloseWindow);
                    ui.close();
                }
                if ui.button("Quitter").clicked() {
                    actions.push(Action::Quit);
                    ui.close();
                }
            });
            ui.menu_button("Edition", |ui| {
                if ui.button("Annuler          Ctrl+Z").clicked() {
                    actions.push(Action::Undo);
                    ui.close();
                }
                if ui.button("Rétablir         Ctrl+Y").clicked() {
                    actions.push(Action::Redo);
                    ui.close();
                }
                ui.separator();
                if ui.button("Couper           Ctrl+X").clicked() {
                    actions.push(Action::Cut);
                    ui.close();
                }
                if ui.button("Copier           Ctrl+C").clicked() {
                    actions.push(Action::Copy);
                    ui.close();
                }
                if ui.button("Coller           Ctrl+V").clicked() {
                    actions.push(Action::Paste);
                    ui.close();
                }
                if ui.button("Supprimer        Suppr").clicked() {
                    actions.push(Action::DeleteSelection);
                    ui.close();
                }
                ui.separator();
                if ui.button("Rechercher...    Ctrl+F").clicked() {
                    actions.push(Action::Find);
                    ui.close();
                }
                if ui.button("Rechercher suivant   F3").clicked() {
                    actions.push(Action::FindNext);
                    ui.close();
                }
                if ui.button("Rechercher précédent Maj+F3").clicked() {
                    actions.push(Action::FindPrevious);
                    ui.close();
                }
                if ui.button("Remplacer...     Ctrl+H").clicked() {
                    actions.push(Action::Replace);
                    ui.close();
                }
                if ui.button("Atteindre...     Ctrl+G").clicked() {
                    actions.push(Action::Goto);
                    ui.close();
                }
                ui.separator();
                if ui.button("Tout sélectionner Ctrl+A").clicked() {
                    actions.push(Action::SelectAll);
                    ui.close();
                }
                if ui.button("Heure/Date       F5").clicked() {
                    actions.push(Action::InsertDate);
                    ui.close();
                }
                ui.separator();
                if ui.button("Police...").clicked() {
                    actions.push(Action::Font);
                    ui.close();
                }
            });
            ui.menu_button("Affichage", |ui| {
                ui.menu_button("Zoom", |ui| {
                    if ui.button("Zoom avant      Ctrl++").clicked() {
                        actions.push(Action::Zoom(1));
                        ui.close();
                    }
                    if ui.button("Zoom arrière    Ctrl+-").clicked() {
                        actions.push(Action::Zoom(-1));
                        ui.close();
                    }
                    if ui.button("Zoom par défaut Ctrl+0").clicked() {
                        actions.push(Action::ResetZoom);
                        ui.close();
                    }
                });
                let mut wrap_checked = wrap;
                if ui.checkbox(&mut wrap_checked, "Retour à la ligne").clicked() {
                    actions.push(Action::ToggleWrap);
                    ui.close();
                }
                let mut status_checked = status_bar;
                if ui.checkbox(&mut status_checked, "Barre d'état").clicked() {
                    actions.push(Action::ToggleStatus);
                    ui.close();
                }
                ui.separator();
                let mut markdown_checked = markdown;
                if ui.checkbox(&mut markdown_checked, "Markdown").clicked() {
                    actions.push(Action::ToggleMarkdown);
                    ui.close();
                }
                let mut preview_checked = markdown_preview;
                if ui
                    .checkbox(&mut preview_checked, "Aperçu Markdown   Ctrl+Maj+M")
                    .clicked()
                {
                    actions.push(Action::ToggleMarkdownPreview);
                    ui.close();
                }
                let mut bar_checked = format_bar;
                if ui
                    .checkbox(&mut bar_checked, "Barre de mise en forme")
                    .clicked()
                {
                    actions.push(Action::ToggleFormatBar);
                    ui.close();
                }
                ui.menu_button("Thème", |ui| {
                    for mode in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
                        if ui.selectable_label(theme == mode, mode.label()).clicked() {
                            actions.push(Action::SetTheme(mode));
                            ui.close();
                        }
                    }
                });
            });
            ui.menu_button("Aide", |ui| {
                if ui.button("À propos de Notepad RS").clicked() {
                    actions.push(Action::About);
                    ui.close();
                }
            });
        });
    }

    /// Sous-menu « Fichiers recents » du menu Fichier.
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
                        .add_enabled(can_close_right, egui::Button::new("Fermer les onglets à droite"))
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
                let target = ui.input(|input| input.pointer.interact_pos()).and_then(|pos| {
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
        if let Some(name) = self.font_family.clone() {
            self.ensure_font(&ctx, &name);
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
            let is_image = path.extension().and_then(|ext| ext.to_str()).map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp")).unwrap_or(false);
            if self.active_tab().doc.moodboard && is_image {
                let n = self.active_tab().mood_items.len() as f32;
                self.active_tab_mut().mood_items.push(session::MoodItem {
                    text: String::new(), image_path: Some(path.to_string_lossy().into_owned()),
                    x: 50.0 + n * 24.0, y: 50.0 + n * 24.0, width: 240.0, height: 180.0, color: 0,
                });
                self.active_tab_mut().doc.dirty = true;
                self.session_dirty = true;
            } else {
                self.open_path(path);
            }
        }

        // Fermeture de la fenetre : demande de confirmation si necessaire.
        if ctx.input(|input| input.viewport().close_requested()) {
            if self.has_unsaved() && !self.pending_quit {
                self.pending_quit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            } else if !self.has_unsaved() {
                self.save_session(&ctx);
            }
        }

        let mut actions = self.shortcuts(&ctx);

        egui::Panel::top("menu").show(ui, |ui| {
            self.menu_bar(ui, &mut actions);
            self.tab_strip(ui, &mut actions);
            if self.tabs[self.active].doc.markdown && self.show_format_bar {
                self.format_bar(ui, &mut actions);
            }
        });

        if self.status_bar {
            let encoding = self.active_tab().doc.encoding;
            let eol = self.active_tab().doc.eol;
            egui::Panel::bottom("status").show(ui, |ui| {
                let tab = self.active_tab();
                let (line, column) = line_col(&tab.doc.text, tab.doc.cursor_char);
                ui.horizontal(|ui| {
                    ui.label(format!("Ln {line}, Col {column}"));
                    ui.separator();
                    let total_chars = tab.doc.text.chars().count();
                    if tab.doc.selection_chars > 0 {
                        ui.label(format!(
                            "{} sur {} caractère(s)",
                            tab.doc.selection_chars, total_chars
                        ));
                    } else {
                        ui.label(format!("{} caractère(s)", total_chars));
                    }
                    ui.separator();
                    ui.label(if tab.doc.markdown {
                        "Markdown"
                    } else {
                        "Texte brut"
                    });
                    ui.separator();
                    // Comme la barre d'etat du Bloc-notes : codage et fin de
                    // ligne ouvrent un menu au clic.
                    ui.menu_button(encoding.label(), |ui| {
                        for choice in ENCODINGS {
                            if ui.selectable_label(choice == encoding, choice.label()).clicked() {
                                actions.push(Action::SetEncoding(choice));
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    ui.menu_button(eol.short(), |ui| {
                        for choice in EOL_ALL {
                            if ui.selectable_label(choice == eol, choice.label()).clicked() {
                                actions.push(Action::SetEol(choice));
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    ui.label(format!(
                        "Zoom {} %",
                        (tab.doc.font_size / 15.0 * 100.0).round() as i32
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.status.is_empty() {
                            ui.add(egui::Label::new(&self.status).truncate());
                        }
                    });
                });
            });
        }

        let mut typed = false;
        // Barre laterale de pages, inspirée de Notion : chaque onglet devient une page navigable.
        let mut page_to_select = None;
        let mut create_page = false;
        let mut create_child = false;
        let mut template: Option<&str> = None;
        let known_page_ids: Vec<u64> = self.tabs.iter().map(|tab| tab.doc.id).collect();
        let mut renamed_page = false;
        egui::SidePanel::left("notion-pages-sidebar").resizable(true).default_width(230.0).show(ctx, |ui| {
            ui.heading("Pages");
            ui.add(egui::TextEdit::singleline(&mut self.page_search).hint_text("Rechercher…"));
            ui.separator();
            if ui.button("+ Nouvelle page").clicked() { create_page = true; }
            if ui.button("+ Sous-page").clicked() { create_child = true; }
            ui.menu_button("Nouveau modèle", |ui| {
                if ui.button("Notes structurées").clicked() { template = Some("# Notes\n\n## À retenir\n\n## Actions\n- "); ui.close(); }
                if ui.button("Projet").clicked() { template = Some("# Projet\n\n## Objectif\n\n## Tâches\n- [ ] "); ui.close(); }
                if ui.button("Réunion").clicked() { template = Some("# Réunion\n\nDate :\nParticipants :\n\n## Compte rendu\n"); ui.close(); }
            });
            ui.add_space(4.0);
            for (index, tab) in self.tabs.iter_mut().enumerate() {
                let query = self.page_search.to_lowercase();
                if !query.is_empty() && !tab.doc.title().to_lowercase().contains(&query) && !tab.doc.text.to_lowercase().contains(&query) { continue; }
                let depth = tab.page_parent.map(|parent| if known_page_ids.contains(&parent) { 1 } else { 0 }).unwrap_or(0);
                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 18.0);
                    let icon = if tab.doc.moodboard { "🎨" } else { "📄" };
                    let mut title = if tab.page_title.is_empty() { tab.doc.title() } else { tab.page_title.clone() };
                    let response = ui.add(egui::TextEdit::singleline(&mut title).desired_width(150.0).hint_text("Nom de la page"));
                    if response.changed() { tab.page_title = title; renamed_page = true; }
                    if ui.small_button(icon).clicked() { page_to_select = Some(index); }
                });
            }
        ui.separator();
        ui.label("Backlinks");
        let active_title = self.active_tab().page_title.clone();
        let active_title = if active_title.is_empty() { self.active_tab().doc.title() } else { active_title };
        let token = format!("[[{}]]", active_title);
        let mut backlink_indices = Vec::new();
        for (index, tab) in self.tabs.iter().enumerate() {
            if index != self.active && tab.doc.text.contains(&token) { backlink_indices.push(index); }
        }
        if backlink_indices.is_empty() { ui.small("Aucune référence"); }
        for index in backlink_indices {
            let label = self.tabs[index].page_title.clone();
            let label = if label.is_empty() { self.tabs[index].doc.title() } else { label };
            if ui.selectable_label(false, format!("↗ {label}")).clicked() { page_to_select = Some(index); }
        }
        });
        if renamed_page { self.session_dirty = true; }
        if create_page { self.new_tab(); }
        if let Some(content) = template { self.new_tab(); self.active_tab_mut().doc.text = content.to_string(); self.active_tab_mut().doc.markdown = true; self.active_tab_mut().doc.dirty = true; self.session_dirty = true; }
        if create_child {
            let parent = self.tabs.get(self.active).map(|tab| tab.doc.id);
            self.new_tab();
            self.tabs[self.active].page_parent = parent;
        }
        if let Some(index) = page_to_select { self.active = index; self.pending_focus = true; }
        egui::CentralPanel::default().show(ui, |ui| {
            if self.show_print_preview {
                self.print_preview_ui(ui);
            } else {
                if self.tabs[self.active].doc.moodboard {
                    self.moodboard_ui(ui, &ctx);
                    return;
                }
                let side_by_side =
                    self.tabs[self.active].doc.markdown && self.show_markdown_preview;
                if side_by_side {
                    let height = ui.available_height();
                    ui.horizontal_top(|ui| {
                        let width = (ui.available_width() - 8.0) / 2.0;
                        ui.allocate_ui_with_layout(
                            egui::vec2(width, height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| typed = self.editor_ui(ui, &ctx),
                        );
                        ui.separator();
                        ui.allocate_ui_with_layout(
                            egui::vec2(width, height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| self.markdown_preview_ui(ui),
                        );
                    });
                } else {
                    typed = self.editor_ui(ui, &ctx);
                }
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
            egui::Window::new(if self.show_replace { "Remplacer" } else { "Rechercher" })
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
                            ui.label("Police :");
                            egui::ComboBox::from_id_salt("font-family")
                                .selected_text(draft.0.clone())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut draft.0,
                                        DEFAULT_FONT_LABEL.to_string(),
                                        DEFAULT_FONT_LABEL,
                                    );
                                    for (name, _) in available_system_fonts() {
                                        ui.selectable_value(&mut draft.0, name.to_string(), name);
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Taille :");
                            ui.add(
                                egui::DragValue::new(&mut draft.1)
                                    .range(8.0..=60.0)
                                    .suffix(" pt"),
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
                self.default_font_size = size;
                if let Some(name) = self.font_family.clone() {
                    self.ensure_font(&ctx, &name);
                }
                self.status = format!("Police : {chosen}, {size} pt");
                open = false;
            }
            if cancel {
                open = false;
            }
            self.show_font = open;
        }

        self.page_setup_window(&ctx);

        if self.pending_quit {
            let mut keep_open = true;
            egui::Window::new("Enregistrer les modifications ?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label("Certains onglets contiennent des modifications non enregistrees.");
                    ui.horizontal(|ui| {
                        if ui.button("Enregistrer").clicked() {
                            for index in 0..self.tabs.len() {
                                if self.tabs[index].doc.dirty {
                                    self.active = index;
                                    self.save_active(false);
                                }
                            }
                            if !self.has_unsaved() {
                                keep_open = false;
                            }
                        }
                        if ui.button("Ne pas enregistrer").clicked() {
                            keep_open = false;
                        }
                        if ui.button("Annuler").clicked() {
                            keep_open = false;
                            self.pending_quit = false;
                        }
                    });
                });
            if !keep_open {
                self.pending_quit = false;
                self.save_session(&ctx);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        if let Some(path) = self.pending_ansi_save.clone() {
            let mut open = true;
            egui::Window::new("Conversion ANSI")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(&ctx, |ui| {
                    ui.label("Certains caractères ne peuvent pas être représentés en ANSI (Windows-1252). Ils seraient remplacés et perdus.");
                    ui.label(format!("Fichier : {}", path.display()));
                    ui.horizontal(|ui| {
                        if ui.button("Enregistrer quand même").clicked() {
                            self.pending_ansi_save = None;
                            self.write_active_to(path.clone());
                        }
                        if ui.button("Annuler").clicked() {
                            self.pending_ansi_save = None;
                        }
                    });
                });
            if !open {
                self.pending_ansi_save = None;
            }
        }

        if let Some(doc_id) = self.pending_close.first().copied() {
            let mut resolved = false;
            let mut save = false;
            let mut cancel = false;
            let title = self
                .tabs
                .iter()
                .find(|tab| tab.doc.id == doc_id)
                .map(|tab| tab.doc.title())
                .unwrap_or_default();
            egui::Window::new("Fermer l'onglet ?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(&ctx, |ui| {
                    ui.label(format!(
                        "« {title} » contient des modifications non enregistrees."
                    ));
                    ui.horizontal(|ui| {
                        if ui.button("Enregistrer").clicked() {
                            save = true;
                            resolved = true;
                        }
                        if ui.button("Ne pas enregistrer").clicked() {
                            resolved = true;
                        }
                        if ui.button("Annuler").clicked() {
                            cancel = true;
                        }
                    });
                });
            if cancel {
                self.pending_close.clear();
            } else if resolved {
                if save {
                    if let Some(index) = self.tabs.iter().position(|tab| tab.doc.id == doc_id) {
                        self.active = index;
                        self.save_active(false);
                    }
                }
                if let Some(index) = self.tabs.iter().position(|tab| tab.doc.id == doc_id) {
                    if !self.tabs[index].doc.dirty {
                        self.close_tab(index);
                    }
                }
                self.pending_close.remove(0);
            }
        }

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

        if self.close_requested {
            self.close_requested = false;
            self.save_session(&ctx);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.session_dirty && self.last_session_save.elapsed() >= Duration::from_secs(3) {
            // Autosauvegarde, comme la tache periodique du Bloc-notes.
            self.save_session(&ctx);
        }

        let active = self.active_tab();
        let name = active.doc.title();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!("{name} - Notepad RS")));
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

fn byte_offsets(text: &str) -> impl Iterator<Item = usize> + '_ {
    text.char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once(text.len()))
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
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, TRUE};
    use windows_sys::core::BOOL;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
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
        let ok = unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) };
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
