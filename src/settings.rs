//! Preferences and task-oriented workspace tools. No document is removed by a reset.
use super::*;

pub const CATEGORIES: [&str; 9] = [
    "Général",
    "Apparence",
    "Éditeur",
    "Moodboard",
    "Fichiers et données",
    "Sauvegardes",
    "Synchronisation",
    "Raccourcis",
    "Confidentialité",
];

pub struct State {
    pub compare_source: Option<u64>,
    pub compare_target: Option<u64>,
    pub board_export: Option<String>,
    pub export_target: Option<u64>,
    pub export_scale: f32,
    pub export_canvas: bool,
    pub exported_path: Option<PathBuf>,
    pub undo_trash: Vec<u64>,
    pub trash_query: String,
    pub purge_page: Option<u64>,
    pub shortcut_drafts: std::collections::BTreeMap<String, String>,
    pub shortcut_error: String,
    pub external_write: Option<(u64, PathBuf)>,
    pub allow_overwrite: Option<u64>,
    pub category: usize,
    pub query: String,
    pub modified_only: bool,
    pub save_error: Option<String>,
    pub last_saved: Option<String>,
    pub recovery: Option<String>,
    pub sync_error: Option<String>,
    pub last_sync: Option<String>,
    pub sync_paused: bool,
    pub palette: bool,
    pub palette_query: String,
    pub palette_cursor: usize,
    pub palette_focus: bool,
    pub history: bool,
    pub version_label: String,
    pub version_selection: Option<usize>,
    pub concentration: bool,
    pub search: bool,
    pub search_kind: u8,
    pub search_scope: u8,
    pub favorites_only: bool,
    pub include_trash: bool,
    pub sort: u8,
    pub template_name: String,
    pub last_backup: Instant,
    pub last_version: Instant,
    pub backup_error: Option<String>,
    pub import_pending: Option<Session>,
    pub import_summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (NotepadApp, egui::Context) {
        let ctx = egui::Context::default();
        let app = NotepadApp::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            None,
            std::env::temp_dir().join(format!("settings-test-{}.json", workspace_io::uid())),
        );
        (app, ctx)
    }
    #[test]
    fn changing_encoding_keeps_unsaved_unicode_and_undo_history() {
        let (mut app, ctx) = fixture();
        app.tabs[0] = Tab::open(0, PathBuf::from("original.txt"), b"Original".to_vec()).unwrap();
        app.active_tab_mut().doc.text = "Brouillon é 東京".into();
        app.active_tab_mut().touch();
        let history = app.active_tab().history.states.clone();
        app.handle_action(&ctx, Action::SetEncoding(Encoding::Utf16Le));
        assert_eq!(app.active_tab().doc.text, "Brouillon é 東京");
        assert!(app.active_tab().doc.dirty);
        assert_eq!(app.active_tab().history.states, history);
        app.handle_action(&ctx, Action::ReopenEncoding(Encoding::Ansi));
        assert_eq!(app.active_tab().doc.text, "Brouillon é 東京");
        app.active_tab_mut().doc.reinterpret(Encoding::Utf8);
        assert_eq!(app.active_tab().doc.text, "Brouillon é 東京");
    }
    #[test]
    fn eol_conversion_requires_save_and_wrap_is_page_scoped() {
        let (mut app, ctx) = fixture();
        app.active_tab_mut().doc.text = "a\nb".into();
        app.handle_action(&ctx, Action::SetEol(Eol::Lf));
        assert!(app.active_tab().doc.dirty);
        assert_eq!(app.active_tab().doc.bytes_for_save(), b"a\nb");
        app.handle_action(&ctx, Action::ToggleWrap);
        assert!(!app.active_tab().doc.wrap);
        assert!(app.default_wrap);
    }
    #[test]
    fn corrupted_primary_recovers_backup_and_preserves_it_on_next_save() {
        let dir = std::env::temp_dir().join(format!("recovery-test-{}", workspace_io::uid()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");
        let mut good = Session::default();
        good.tabs.push(session::SessionTab {
            text: Some("Important".into()),
            ..Default::default()
        });
        good.save_to(&path).unwrap();
        good.save_to(&path).unwrap();
        std::fs::write(&path, b"{broken").unwrap();
        let (recovered, warning) = Session::load_report(&path);
        assert!(warning.is_some());
        assert_eq!(recovered.tabs[0].text.as_deref(), Some("Important"));
        recovered.save_to(&path).unwrap();
        assert_eq!(
            Session::load_from(&path.with_extension("json.bak")).tabs[0]
                .text
                .as_deref(),
            Some("Important")
        );
        assert!(std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .any(|e| e.path().extension().is_some_and(|x| x == "corrupt")));
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            std::fs::remove_file(entry.path()).unwrap();
        }
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn external_edit_does_not_get_overwritten() {
        let (mut app, _) = fixture();
        let path = std::env::temp_dir().join(format!("external-{}.txt", workspace_io::uid()));
        std::fs::write(&path, b"Original").unwrap();
        app.tabs[0] = Tab::open(0, path.clone(), b"Original".to_vec()).unwrap();
        app.active_tab_mut().doc.text = "Local".into();
        app.active_tab_mut().touch();
        std::fs::write(&path, b"External").unwrap();
        app.write_active_to(path.clone());
        assert_eq!(std::fs::read(&path).unwrap(), b"External");
        assert_eq!(app.active_tab().doc.text, "Local");
        assert!(app.options.external_write.is_some());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn search_includes_board_notes_and_obeys_trash_and_kind_filters() {
        let (mut app, _) = fixture();
        app.new_moodboard();
        app.active_tab_mut().mood_items[0].text = "Idée forêt".into();
        app.page_search = "forêt".into();
        assert!(app.page_matches(app.active_tab()));
        app.options.search_kind = 1;
        assert!(!app.page_matches(app.active_tab()));
        app.options.search_kind = 3;
        app.active_tab_mut().workspace.trashed = true;
        assert!(!app.page_matches(app.active_tab()));
        app.options.include_trash = true;
        assert!(app.page_matches(app.active_tab()));
    }
    #[test]
    fn regex_and_selection_replacement_preserve_other_text_and_create_recovery() {
        let (mut app, _) = fixture();
        app.active_tab_mut().doc.text = "é abc 123 abc 456".into();
        app.find.needle = "[0-9]+".into();
        app.find.regex = true;
        app.find.replacement = "X".into();
        app.find.selection_only = true;
        app.find.scope = Some((app.active_tab().doc.id, 0, 9));
        app.replace_all();
        assert_eq!(app.active_tab().doc.text, "é abc X abc 456");
        assert_eq!(app.active_tab().workspace.snapshots.len(), 1);
        app.find.needle = "[".into();
        let before = app.active_tab().doc.text.clone();
        app.replace_all();
        assert_eq!(app.active_tab().doc.text, before);
        assert_eq!(
            find::occurrences("chat chats chat", "chat", false, true, false)
                .unwrap()
                .len(),
            2
        );
    }
    #[test]
    fn settings_and_tools_render_at_narrow_and_desktop_widths() {
        for width in [460.0, 1440.0] {
            for category in 0..CATEGORIES.len() {
                let (mut app, ctx) = fixture();
                app.design.settings = true;
                app.options.category = category;
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 800.0),
                        )),
                        ..Default::default()
                    },
                    |_| app.options_ui(&ctx),
                );
                assert!(!out.shapes.is_empty());
                out.textures_delta.clear();
            }
        }
    }
    #[test]
    fn shortcut_conflicts_and_altgr_are_rejected() {
        let mut bindings = std::collections::BTreeMap::new();
        bindings.insert("search".into(), "Ctrl+Maj+J".into());
        assert!(validate_shortcut("history", "Ctrl+Shift+J", &bindings).is_err());
        assert!(validate_shortcut("history", "Ctrl+S", &bindings).is_err());
        assert!(validate_shortcut("history", "Ctrl+,", &bindings).is_err());
        assert!(validate_shortcut("history", "Ctrl+inconnu+J", &bindings).is_err());
        assert!(validate_shortcut("history", "Ctrl+Alt+E", &bindings).is_err());
        assert!(validate_shortcut("history", "Ctrl+Maj+K", &bindings).is_ok());
    }
    #[test]
    fn trash_restore_keeps_parent_child_relationships() {
        let (mut app, _) = fixture();
        let root = app.active_tab().doc.id;
        app.new_tab();
        let child = app.active_tab().doc.id;
        app.active_tab_mut().page_parent = Some(root);
        app.new_tab();
        let unrelated = app.active_tab().doc.id;
        app.trash_page(root);
        assert!(app
            .tabs
            .iter()
            .filter(|t| t.doc.id != unrelated)
            .all(|t| t.workspace.trashed));
        assert!(!app.active_tab().workspace.trashed);
        app.restore_trash(child);
        assert!(app.tabs.iter().all(|t| !t.workspace.trashed));
        assert_eq!(
            app.tabs
                .iter()
                .find(|t| t.doc.id == child)
                .unwrap()
                .page_parent,
            Some(root)
        );
    }
    #[test]
    fn custom_recent_limit_and_explicit_page_kind_are_respected() {
        let (mut app, _) = fixture();
        app.preferences.recent_limit = 25;
        for i in 0..30 {
            app.remember_recent(std::path::Path::new(&format!("{i}.txt")));
        }
        assert_eq!(app.recent.len(), 25);
        app.preferences.default_kind = 2;
        app.new_page_kind(0);
        assert!(!app.active_tab().doc.moodboard);
        app.new_tab();
        assert!(app.active_tab().doc.moodboard);
    }
    #[test]
    fn duplication_and_version_restore_do_not_reuse_file_path_or_identity() {
        let (mut app, _) = fixture();
        app.active_tab_mut().doc.path = Some(PathBuf::from("original.md"));
        app.active_tab_mut().doc.text = "original".into();
        app.create_version("Point sûr");
        let uid = app.active_tab().workspace.uid.clone();
        let snapshot = app.active_tab().workspace.snapshots[0].clone();
        app.duplicate_page();
        app.apply_version(snapshot);
        assert_ne!(app.active_tab().workspace.uid, uid);
        assert!(app.active_tab().doc.path.is_none());
        assert_eq!(
            app.tabs[0].doc.path.as_deref(),
            Some(std::path::Path::new("original.md"))
        );
    }
    #[test]
    fn automatic_versions_include_boards_without_duplicating_unchanged_content() {
        let (mut app, ctx) = fixture();
        app.new_moodboard();
        app.options.last_version = Instant::now() - Duration::from_secs(31);
        app.scheduled_backup(&ctx);
        assert_eq!(app.active_tab().workspace.snapshots.len(), 1);
        assert_eq!(app.active_tab().workspace.snapshots[0].mood_items.len(), 1);
        app.options.last_version = Instant::now() - Duration::from_secs(31);
        app.scheduled_backup(&ctx);
        assert_eq!(app.active_tab().workspace.snapshots.len(), 1);
    }
    #[test]
    fn opening_conflict_comparison_does_not_change_either_document() {
        let (mut app, ctx) = fixture();
        app.active_tab_mut().doc.text = "Version locale".into();
        let target = app.active_tab().doc.id;
        app.new_tab();
        app.active_tab_mut().doc.text = "Version distante".into();
        app.options.compare_source = Some(app.active_tab().doc.id);
        app.options.compare_target = Some(target);
        let before: Vec<_> = app.tabs.iter().map(|t| t.doc.text.clone()).collect();
        let mut output = ctx.run_ui(Default::default(), |_| app.conflict_comparison_ui(&ctx));
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
        assert_eq!(
            app.tabs
                .iter()
                .map(|t| t.doc.text.clone())
                .collect::<Vec<_>>(),
            before
        );
    }
}
impl Default for State {
    fn default() -> Self {
        Self {
            compare_source: None,
            compare_target: None,
            export_target: None,
            board_export: None,
            export_scale: 1.0,
            export_canvas: true,
            exported_path: None,
            undo_trash: Vec::new(),
            trash_query: String::new(),
            purge_page: None,
            shortcut_drafts: Default::default(),
            shortcut_error: String::new(),
            external_write: None,
            allow_overwrite: None,
            category: 0,
            query: String::new(),
            modified_only: false,
            save_error: None,
            last_saved: None,
            recovery: None,
            sync_error: None,
            last_sync: None,
            sync_paused: false,
            palette: false,
            palette_query: String::new(),
            palette_cursor: 0,
            palette_focus: false,
            history: false,
            version_label: String::new(),
            version_selection: None,
            concentration: false,
            search: false,
            search_kind: 0,
            search_scope: 0,
            favorites_only: false,
            include_trash: false,
            sort: 0,
            template_name: String::new(),
            last_backup: Instant::now(),
            last_version: Instant::now(),
            backup_error: None,
            import_pending: None,
            import_summary: String::new(),
        }
    }
}

fn normalized(text: &str) -> String {
    text.to_lowercase()
        .replace(['é', 'è', 'ê', 'ë'], "e")
        .replace(['à', 'â'], "a")
        .replace('ç', "c")
        .replace(['î', 'ï'], "i")
        .replace('ô', "o")
        .replace(['ù', 'û', 'ü'], "u")
}
pub(crate) fn matches(query: &str, label: &str) -> bool {
    normalized(label).contains(&normalized(query.trim()))
}
pub const CUSTOM_COMMANDS: [(&str, &str); 5] = [
    ("settings", "Paramètres"),
    ("palette", "Palette de commandes"),
    ("search", "Recherche dans l’espace"),
    ("history", "Historique de la page"),
    ("focus", "Mode concentration"),
];
pub fn shortcut(value: &str) -> Option<(egui::Modifiers, egui::Key)> {
    let mut modifiers = egui::Modifiers::default();
    let mut key = None;
    for part in value.split('+').map(|p| p.trim().to_ascii_uppercase()) {
        match part.as_str() {
            "CTRL" => {
                modifiers.ctrl = true;
                modifiers.command = true;
            }
            "ALT" => modifiers.alt = true,
            "MAJ" | "SHIFT" => modifiers.shift = true,
            text => {
                if key.is_some() {
                    return None;
                }
                key = Some(match text {
                    "COMMA" => egui::Key::Comma,
                    "MINUS" => egui::Key::Minus,
                    _ => egui::Key::from_name(text)?,
                });
            }
        }
    }
    if !modifiers.ctrl || (modifiers.alt && modifiers.ctrl) {
        return None;
    } // Reserve AltGr for text on international keyboards.
    key.map(|key| (modifiers, key))
}

pub fn validate_shortcut(
    command: &str,
    value: &str,
    existing: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    let pair = shortcut(value).ok_or(
        "Utilisez Ctrl + une touche, avec Maj si nécessaire. Ctrl+Alt est réservé à AltGr.",
    )?;
    for reserved in [
        "Ctrl+N",
        "Ctrl+Maj+N",
        "Ctrl+O",
        "Ctrl+S",
        "Ctrl+Maj+S",
        "Ctrl+W",
        "Ctrl+Maj+W",
        "Ctrl+F",
        "Ctrl+Maj+F",
        "Ctrl+H",
        "Ctrl+G",
        "Ctrl+P",
        "Ctrl+Maj+P",
        "Ctrl+Comma",
        "Ctrl+0",
        "Ctrl+Minus",
        "Ctrl+Z",
        "Ctrl+Y",
        "Ctrl+A",
        "Ctrl+C",
        "Ctrl+V",
        "Ctrl+X",
        "Ctrl+Maj+M",
        "Ctrl+B",
        "Ctrl+I",
        "Ctrl+U",
    ] {
        if shortcut(reserved) == Some(pair) {
            return Err(format!(
                "{reserved} est déjà réservé par une commande standard."
            ));
        }
    }
    if existing
        .iter()
        .any(|(id, key)| id != command && shortcut(key) == Some(pair))
    {
        return Err("Cette combinaison est déjà affectée.".into());
    }
    Ok(())
}
fn row(
    ui: &mut egui::Ui,
    query: &str,
    modified_only: bool,
    label: &str,
    help: &str,
    modified: bool,
    content: impl FnOnce(&mut egui::Ui),
) {
    if (modified_only && !modified) || !matches(query, &format!("{label} {help}")) {
        return;
    }
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new(label).strong());
        ui.label(help);
        content(ui);
    });
    ui.add_space(4.0);
}
fn bool_setting(
    ui: &mut egui::Ui,
    query: &str,
    modified: bool,
    label: &str,
    help: &str,
    value: &mut bool,
    default: bool,
) {
    row(ui, query, modified, label, help, *value != default, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(value, "Activé");
            if ui.small_button("Réinitialiser").clicked() {
                *value = default;
            }
        });
    });
}
fn number(
    ui: &mut egui::Ui,
    query: &str,
    modified: bool,
    label: &str,
    help: &str,
    value: &mut f32,
    default: f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) {
    row(ui, query, modified, label, help, *value != default, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::DragValue::new(value).range(range).suffix(suffix));
            if ui.small_button("Réinitialiser").clicked() {
                *value = default;
            }
        });
    });
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Profile {
    version: u32,
    preferences: session::NotepadPreferences,
    theme: u8,
    font: Option<String>,
    font_size: f32,
    wrap: bool,
    status_bar: bool,
    scale: f32,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            version: 1,
            preferences: Default::default(),
            theme: 0,
            font: None,
            font_size: 15.0,
            wrap: true,
            status_bar: true,
            scale: 1.0,
        }
    }
}

impl NotepadApp {
    fn conflict_comparison_ui(&mut self, ctx: &egui::Context) {
        let Some(source_id) = self.options.compare_source else {
            return;
        };
        let Some(source) = self
            .tabs
            .iter()
            .find(|t| t.doc.id == source_id)
            .map(tab_snapshot)
        else {
            self.options.compare_source = None;
            return;
        };
        let mut open = true;
        egui::Window::new("Comparer les copies de conflit").open(&mut open).default_width(800.0).default_height(600.0).show(ctx, |ui| {
            ui.label("Les deux copies restent conservées. Vous pouvez copier le contenu de gauche vers une page cible après comparaison.");
            egui::ComboBox::from_id_salt("conflict-target").selected_text(self.options.compare_target.and_then(|id|self.tabs.iter().find(|t|t.doc.id == id)).map(ui_design::page_title).unwrap_or_else(||"Choisir la page cible".into())).show_ui(ui, |ui| {
                for tab in self.tabs.iter().filter(|t|t.doc.id != source_id && !t.workspace.trashed && t.doc.moodboard == source.moodboard) { ui.selectable_value(&mut self.options.compare_target,Some(tab.doc.id),ui_design::page_title(tab)); }
            });
            let target = self.options.compare_target.and_then(|id|self.tabs.iter().find(|t|t.doc.id == id)).map(tab_snapshot);
            egui::ScrollArea::vertical().max_height(390.0).show(ui, |ui| { ui.columns(2, |cols| {
                cols[0].heading(&source.page_title); cols[0].label(source.text.as_deref().unwrap_or(""));
                for item in &source.mood_items { cols[0].label(format!("{} · {:.0} × {:.0} · ({:.0}, {:.0})",if item.image_path.is_some() { "Image" } else { &item.text },item.width,item.height,item.x,item.y)); }
                if let Some(target) = &target { cols[1].heading(&target.page_title); cols[1].label(target.text.as_deref().unwrap_or("")); for item in &target.mood_items { cols[1].label(format!("{} · {:.0} × {:.0} · ({:.0}, {:.0})",if item.image_path.is_some() { "Image" } else { &item.text },item.width,item.height,item.x,item.y)); } }
            }); });
            let can_copy = target.as_ref().is_some_and(|t|!t.workspace.read_only);
            if !can_copy { ui.label("Choisissez une cible du même type, non verrouillée."); }
            if ui.add_enabled(can_copy,egui::Button::new("Copier vers la cible et conserver sa version actuelle")).clicked() {
                if let Some(index) = self.tabs.iter().position(|t|Some(t.doc.id) == self.options.compare_target) {
                    self.active = index; self.create_version("Avant résolution de conflit");
                    self.apply_version(session::PageVersion {created_at:chrono::Local::now().to_rfc3339(),label:"Copie de conflit".into(),text:source.text.clone().unwrap_or_default(),mood_items:source.mood_items.clone(),canvas_size:source.workspace.canvas_size});
                    self.status = "Contenu copié. L’ancienne version et la copie de conflit restent conservées.".into();
                }
            }
            if ui.button("Conserver les deux copies et marquer comme traité").clicked() {
                if let Some(tab) = self.tabs.iter_mut().find(|t|t.doc.id == source_id) {
                    tab.workspace.conflict_of = None;
                    if let Some(base) = tab.page_title.strip_suffix(" — conflit") { tab.page_title = format!("{base} — copie conservée"); }
                    self.session_dirty = true;
                }
                self.options.compare_source = None;
            }
        });
        if !open {
            self.options.compare_source = None;
        }
    }
    fn reset_settings_category(&mut self, ctx: &egui::Context) {
        let defaults = session::NotepadPreferences::default();
        match self.options.category {
            0 => {
                self.preferences.start_blank = false;
                self.preferences.open_new_window = false;
                self.preferences.default_kind = 0;
            }
            1 => {
                self.theme = ThemeMode::System;
                ctx.set_zoom_factor(1.0);
                self.design.sidebar = true;
                self.status_bar = true;
                self.options.concentration = false;
            }
            2 => {
                self.preferences.reading_width = defaults.reading_width;
                self.preferences.line_height = defaults.line_height;
                self.font_family = None;
                self.default_font_size = 15.0;
                self.default_wrap = true;
            }
            3 => {
                self.preferences.grid = true;
                self.preferences.snap = false;
                self.preferences.grid_size = 24.0;
            }
            4 => {
                self.preferences.default_encoding = 1;
                self.preferences.default_eol = 0;
                self.preferences.recent_enabled = true;
                self.preferences.recent_limit = 10;
            }
            5 => {
                self.preferences.backup_directory = None;
                self.preferences.backup_minutes = 15;
                self.preferences.version_limit = 50;
            }
            6 => {
                self.sync_directory = None;
                self.preferences.sync_seconds = 5;
                self.options.sync_paused = false;
            }
            7 => {
                self.preferences.shortcuts.clear();
                self.options.shortcut_drafts.clear();
            }
            _ => {}
        }
        self.session_dirty = true;
        self.status =
            "Catégorie réinitialisée. Documents, modèles, versions et sauvegardes conservés."
                .into();
    }
    fn board_export_ui(&mut self, ctx: &egui::Context) {
        let Some(ext) = self.options.board_export.clone() else {
            return;
        };
        let Some(index) = self
            .tabs
            .iter()
            .position(|t| Some(t.doc.id) == self.options.export_target)
        else {
            self.options.board_export = None;
            return;
        };
        let mut open = true;
        egui::Window::new("Exporter le moodboard en image")
            .open(&mut open)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.label(format!(
                    "{} · {}",
                    ui_design::page_title(&self.tabs[index]),
                    ext.to_uppercase()
                ));
                ui.radio_value(
                    &mut self.options.export_canvas,
                    true,
                    "Cadre de la toile (les objets hors cadre seront coupés)",
                );
                ui.radio_value(
                    &mut self.options.export_canvas,
                    false,
                    "Tous les objets visibles",
                );
                ui.add(
                    egui::DragValue::new(&mut self.options.export_scale)
                        .range(0.25..=4.0)
                        .speed(0.25)
                        .suffix(" ×"),
                );
                let tab = &self.tabs[index];
                let canvas = if self.options.export_canvas {
                    tab.workspace.canvas_size
                } else {
                    None
                };
                let dimensions =
                    board_export::dimensions(&tab.mood_items, canvas, self.options.export_scale);
                match &dimensions {
                    Ok([w, h]) => {
                        ui.label(format!(
                            "{w} × {h} px · fond blanc · calques masqués exclus"
                        ));
                    }
                    Err(e) => {
                        ui.colored_label(egui::Color32::RED, e);
                    }
                }
                if ui
                    .add_enabled(
                        dimensions.is_ok(),
                        egui::Button::new("Choisir la destination et exporter…"),
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(format!("moodboard.{ext}"))
                        .add_filter(ext.to_uppercase(), &[ext.as_str()])
                        .save_file()
                    {
                        let result = std::fs::read(windows_fonts_dir().join("segoeui.ttf"))
                            .map_err(|e| format!("Police d’export inaccessible : {e}"))
                            .and_then(|font| {
                                board_export::render(
                                    &tab.mood_items,
                                    canvas,
                                    self.options.export_scale,
                                    &font,
                                    tab.workspace.canvas_appearance.background,
                                )
                            })
                            .and_then(|image| {
                                if ext == "jpg" {
                                    image::DynamicImage::ImageRgba8(image)
                                        .to_rgb8()
                                        .save_with_format(&path, image::ImageFormat::Jpeg)
                                } else {
                                    image.save_with_format(&path, image::ImageFormat::Png)
                                }
                                .map_err(|e| e.to_string())
                            });
                        match result {
                            Ok(()) => {
                                self.status = format!("Image exportée : {}", path.display());
                                self.options.exported_path = Some(path);
                            }
                            Err(e) => self.status = format!("Export impossible : {e}"),
                        }
                    }
                }
                ui.label(&self.status);
                if let Some(path) = &self.options.exported_path {
                    if ui.button("Afficher le fichier dans son dossier").clicked() {
                        let _ = std::process::Command::new("explorer.exe")
                            .arg(format!("/select,{}", path.display()))
                            .spawn();
                    }
                }
            });
        if !open {
            self.options.board_export = None;
            self.options.exported_path = None;
        }
    }
    pub(crate) fn is_descendant(&self, child: u64, parent: u64) -> bool {
        let mut next = Some(child);
        let mut seen = std::collections::HashSet::new();
        while let Some(id) = next {
            if id == parent {
                return true;
            }
            if !seen.insert(id) {
                break;
            }
            next = self
                .tabs
                .iter()
                .find(|t| t.doc.id == id)
                .and_then(|t| t.page_parent);
        }
        false
    }
    pub(crate) fn move_page_menu(&mut self, ui: &mut egui::Ui) {
        let id = self.active_tab().doc.id;
        if ui.button("À la racine de l’espace").clicked() {
            self.active_tab_mut().page_parent = None;
            self.session_dirty = true;
            ui.close();
        }
        let candidates: Vec<_> = self
            .tabs
            .iter()
            .filter(|t| !t.workspace.trashed && !self.is_descendant(t.doc.id, id))
            .map(|t| (t.doc.id, ui_design::page_title(t)))
            .collect();
        for (parent, label) in candidates {
            if ui.button(label).clicked() {
                self.active_tab_mut().page_parent = Some(parent);
                self.session_dirty = true;
                ui.close();
            }
        }
    }
    pub(crate) fn trash_page(&mut self, id: u64) {
        let ids: Vec<_> = self
            .tabs
            .iter()
            .filter(|t| !t.workspace.trashed && self.is_descendant(t.doc.id, id))
            .map(|t| t.doc.id)
            .collect();
        for tab in &mut self.tabs {
            if ids.contains(&tab.doc.id) {
                tab.workspace.trashed = true;
                tab.workspace.trashed_at = Some(chrono::Local::now().to_rfc3339());
            }
        }
        self.status = format!("{} page(s) et sous-page(s) déplacée(s) dans la corbeille. Annuler depuis Page ou Corbeille.",ids.len());
        self.options.undo_trash = ids;
        self.session_dirty = true;
    }
    pub(crate) fn undo_trash(&mut self) {
        for tab in &mut self.tabs {
            if self.options.undo_trash.contains(&tab.doc.id) {
                tab.workspace.trashed = false;
                tab.workspace.trashed_at = None;
            }
        }
        self.options.undo_trash.clear();
        self.session_dirty = true;
    }
    pub(crate) fn restore_trash(&mut self, id: u64) {
        let ids: Vec<_> = self
            .tabs
            .iter()
            .filter(|t| self.is_descendant(t.doc.id, id))
            .map(|t| t.doc.id)
            .collect();
        // Restore ancestors too so the result is visible in the navigation tree.
        let ancestors: Vec<_> = self
            .tabs
            .iter()
            .filter(|t| self.is_descendant(id, t.doc.id))
            .map(|t| t.doc.id)
            .collect();
        for tab in &mut self.tabs {
            if ids.contains(&tab.doc.id) || ancestors.contains(&tab.doc.id) {
                tab.workspace.trashed = false;
                tab.workspace.trashed_at = None;
            }
        }
        self.session_dirty = true;
    }
    fn external_change_ui(&mut self, ctx: &egui::Context) {
        let Some((id, path)) = self.options.external_write.clone() else {
            return;
        };
        let Some(index) = self.tabs.iter().position(|t| t.doc.id == id) else {
            self.options.external_write = None;
            return;
        };
        let mut choice = 0;
        egui::Modal::new(egui::Id::unique("external-file-change")).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 48.0).clamp(280.0, 680.0));
            ui.heading("Le fichier a changé sur disque");
            ui.label(
                "Votre brouillon est conservé. Enregistrer une copie protège les deux versions.",
            );
            let disk = std::fs::read(&path).ok();
            if !self.tabs[index].doc.moodboard {
                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .show(ui, |ui| {
                        ui.columns(2, |cols| {
                            cols[0].label("Votre brouillon");
                            cols[0].label(&self.tabs[index].doc.text);
                            cols[1].label("Fichier sur disque");
                            cols[1].label(
                                disk.as_ref()
                                    .map(|b| self.tabs[index].doc.encoding.decode(b))
                                    .unwrap_or_else(|| "Fichier absent ou inaccessible".into()),
                            );
                        });
                    });
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button("Enregistrer une copie…").clicked() {
                    choice = 1;
                }
                if ui.button("Remplacer le fichier sur disque").clicked() {
                    choice = 2;
                }
                if ui.button("Annuler").clicked() {
                    choice = 3;
                }
            });
        });
        if choice != 0 {
            self.options.external_write = None;
            self.active = index;
            if choice == 1 {
                self.save_active(true);
            }
            if choice == 2 {
                self.options.allow_overwrite = Some(id);
                self.write_active_to(path);
            }
        }
    }
    pub(crate) fn options_ui(&mut self, ctx: &egui::Context) {
        self.conflict_comparison_ui(ctx);
        self.board_export_ui(ctx);
        self.external_change_ui(ctx);
        if !self.design.settings {
            if let Some(error) = self.options.save_error.clone() {
                egui::Window::new("Sauvegarde locale en échec").default_width(460.0).show(ctx, |ui| {
                    ui.label(error);
                    ui.label("Le contenu reste ouvert. Enregistrez une copie avant de quitter si l’erreur persiste.");
                    if ui.button("Réessayer").clicked() { self.save_session(ctx); }
                    if ui.button("Sauvegardes et récupération").clicked() { self.options.category = 5; self.design.settings = true; }
                });
            }
        }
        if let Some(message) = self.options.recovery.clone() {
            egui::Window::new("Récupération de session")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(message);
                    if ui.button("Ouvrir les sauvegardes").clicked() {
                        self.design.settings = true;
                        self.options.category = 5;
                    }
                    if ui.button("J’ai compris").clicked() {
                        self.options.recovery = None;
                    }
                });
        }
        if self.design.settings {
            let mut open = true;
            let before = serde_json::to_vec(&self.preferences).ok();
            egui::Window::new("Paramètres").id(egui::Id::unique("settings-center")).collapsible(false)
                .open(&mut open).default_width(780.0).default_height(650.0)
                .max_width((ctx.content_rect().width() - 32.0).max(280.0))
                .max_height((ctx.content_rect().height() - 110.0).max(180.0)).show(ctx, |ui| {
                ui.label("Les préférences sont appliquées automatiquement. Les pages et fichiers restent conservés.");
                ui.add(egui::TextEdit::singleline(&mut self.options.query).hint_text("Rechercher un réglage : thème, sauvegarde, police…").desired_width(f32::INFINITY));
                ui.checkbox(&mut self.options.modified_only, "Réglages personnalisés seulement");
                ui.horizontal_wrapped(|ui| { for (i, label) in CATEGORIES.iter().enumerate() {
                    if ui.selectable_label(self.options.category == i, *label).clicked() { self.options.category = i; self.options.query.clear(); }
                } });
                ui.separator();
                let body_height = (ui.available_height() - 100.0).max(80.0);
                egui::ScrollArea::vertical().id_salt("settings-body").max_height(body_height).show(ui, |ui| {
                    let categories: Vec<_> = if self.options.query.trim().is_empty() { vec![self.options.category] } else { (0..CATEGORIES.len()).collect() };
                    for category in categories { ui.heading(CATEGORIES[category]); self.settings_category(ui, ctx, category); }
                    if self.options.modified_only || !self.options.query.is_empty() { ui.small("Si aucun réglage ne correspond, effacez la recherche ou le filtre Personnalisés."); }
                });
                if self.options.query.is_empty() && ui.button("Réinitialiser cette catégorie (conserver les documents)").clicked() { self.reset_settings_category(ctx); }
                ui.separator();
                if let Some(error) = self.options.save_error.clone() {
                    ui.colored_label(egui::Color32::RED, error);
                    if ui.button("Réessayer l’enregistrement").clicked() { self.save_session(ctx); }
                } else { ui.label(if self.session_dirty { "Modifications en attente d’enregistrement local" } else { "Préférences enregistrées localement" }); }
            });
            self.design.settings = open;
            if before != serde_json::to_vec(&self.preferences).ok() {
                self.preferences.sanitize();
                self.session_dirty = true;
            }
        }
        self.palette_ui(ctx);
        self.search_window(ctx);
        self.history_ui(ctx);
        self.import_confirmation(ctx);
    }

    fn settings_category(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, category: usize) {
        let q = self.options.query.clone();
        let m = self.options.modified_only;
        let defaults = session::NotepadPreferences::default();
        if m && ((category == 6 && self.sync_directory.is_none())
            || (category == 7 && self.preferences.shortcuts.is_empty())
            || category == 8)
        {
            return;
        }
        match category {
            0 => {
                bool_setting(ui, &q, m, "Démarrer sur une page vide", "Application · Les anciennes pages restent dans l’espace ; seule la page affichée change.", &mut self.preferences.start_blank, false);
                bool_setting(
                    ui,
                    &q,
                    m,
                    "Ouvrir dans une nouvelle fenêtre",
                    "Application · Sinon, ouvrir les fichiers dans un onglet de cette fenêtre.",
                    &mut self.preferences.open_new_window,
                    false,
                );
                row(
                    ui,
                    &q,
                    m,
                    "Type des nouvelles pages",
                    "Valeur par défaut · Aucun document existant n’est converti.",
                    self.preferences.default_kind != 0,
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (i, name) in ["Texte", "Blocs", "Moodboard"].iter().enumerate() {
                                ui.radio_value(&mut self.preferences.default_kind, i as u8, *name);
                            }
                        });
                    },
                );
                row(ui, &q, m, "Profils de travail", "Application · Change la largeur de lecture et le retour à la ligne des nouvelles pages, sans convertir le contenu.", self.preferences.reading_width != defaults.reading_width || !self.default_wrap, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (label, width, wrap) in [("Écriture", 660.0, true), ("Technique", 2400.0, false), ("Visuel", 1000.0, true)] {
                            if ui.button(label).clicked() { self.preferences.reading_width = width; self.default_wrap = wrap; self.session_dirty = true; }
                        }
                    });
                });
            }
            1 => {
                row(
                    ui,
                    &q,
                    m,
                    "Thème",
                    "Application · Suit Windows ou utilise une apparence fixe.",
                    self.theme != ThemeMode::System,
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for value in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
                                if ui
                                    .radio_value(&mut self.theme, value, value.label())
                                    .changed()
                                {
                                    self.session_dirty = true;
                                }
                            }
                        });
                    },
                );
                row(ui, &q, m, "Échelle de l’interface", "Application · Agrandit tous les contrôles, indépendamment du zoom du document.", ctx.zoom_factor() != 1.0, |ui| {
                    let mut scale = ctx.zoom_factor() * 100.0;
                    if ui.add(egui::DragValue::new(&mut scale).range(80.0..=175.0).suffix(" %")).changed() { ctx.set_zoom_factor(scale / 100.0); self.session_dirty = true; }
                    if ui.button("Réinitialiser à 100 %").clicked() { ctx.set_zoom_factor(1.0); self.session_dirty = true; }
                });
                row(
                    ui,
                    &q,
                    m,
                    "Navigation et barre d’état",
                    "Application · Les erreurs restent accessibles dans Paramètres.",
                    !self.design.sidebar || !self.status_bar,
                    |ui| {
                        self.session_dirty |= ui
                            .checkbox(&mut self.design.sidebar, "Navigation des pages")
                            .changed();
                        self.session_dirty |=
                            ui.checkbox(&mut self.status_bar, "Barre d’état").changed();
                    },
                );
                row(
                    ui,
                    &q,
                    m,
                    "Mode concentration",
                    "Fenêtre · Masque la navigation et les propriétés. Échap quitte ce mode.",
                    self.options.concentration,
                    |ui| {
                        ui.checkbox(&mut self.options.concentration, "Concentration");
                    },
                );
            }
            2 => {
                number(
                    ui,
                    &q,
                    m,
                    "Largeur de lecture",
                    "Espace · Les pages peuvent utiliser leur propre largeur dans Propriétés.",
                    &mut self.preferences.reading_width,
                    defaults.reading_width,
                    300.0..=2400.0,
                    " pt",
                );
                number(
                    ui,
                    &q,
                    m,
                    "Interligne",
                    "Espace · Éditeur de texte ; ne modifie pas le fichier.",
                    &mut self.preferences.line_height,
                    1.3,
                    1.0..=2.5,
                    " ×",
                );
                row(ui, &q, m, "Police et taille", "La famille s’applique à l’application ; la taille du dialogue s’applique à cette page.", self.font_family.is_some() || self.default_font_size != 15.0 || !self.default_wrap, |ui| {
                    if ui.button("Choisir la police…").clicked() { self.handle_action(ctx, Action::Font); }
                    ui.label(egui::RichText::new("Aa — Une écriture claire, 0123456789").size(self.default_font_size));
                    if ui.add(egui::DragValue::new(&mut self.default_font_size).range(8.0..=60.0).prefix("Nouvelles pages : ").suffix(" px")).changed() { self.session_dirty = true; }
                    if ui.checkbox(&mut self.default_wrap, "Retour à la ligne pour les nouvelles pages").changed() { self.session_dirty = true; }
                });
                row(
                    ui,
                    &q,
                    m,
                    "Modèles personnalisés",
                    "Espace · Enregistrez le texte de la page active comme modèle réutilisable.",
                    !self.preferences.templates.is_empty(),
                    |ui| {
                        ui.text_edit_singleline(&mut self.options.template_name);
                        if ui
                            .add_enabled(
                                !self.options.template_name.trim().is_empty()
                                    && !self.active_tab().doc.moodboard,
                                egui::Button::new("Créer un modèle depuis cette page"),
                            )
                            .clicked()
                        {
                            self.preferences.templates.push(session::Template {
                                name: self.options.template_name.trim().into(),
                                text: self.active_tab().doc.text.clone(),
                            });
                            self.options.template_name.clear();
                        }
                        let mut remove = None;
                        for (i, template) in self.preferences.templates.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(&template.name);
                                if ui.small_button("Retirer le modèle").clicked() {
                                    remove = Some(i);
                                }
                            });
                        }
                        if let Some(i) = remove {
                            self.preferences.templates.remove(i);
                        }
                    },
                );
            }
            3 => {
                bool_setting(
                    ui,
                    &q,
                    m,
                    "Afficher la grille",
                    "Espace · Repère visuel indépendant du magnétisme.",
                    &mut self.preferences.grid,
                    true,
                );
                bool_setting(
                    ui,
                    &q,
                    m,
                    "Alignement magnétique",
                    "Espace · Aligne les objets sur la grille après un déplacement.",
                    &mut self.preferences.snap,
                    false,
                );
                number(
                    ui,
                    &q,
                    m,
                    "Pas de grille",
                    "Espace · Distance logique entre les repères.",
                    &mut self.preferences.grid_size,
                    24.0,
                    8.0..=200.0,
                    " px",
                );
                ui.label("Dimensions de toile : barre du moodboard. Verrouillage, proportions et calques : propriétés de l’élément.");
            }
            4 => {
                row(ui, &q, m, "Encodage par défaut", "Nouveaux fichiers uniquement · Pour convertir un fichier ouvert, utiliser Détails.", self.preferences.default_encoding != 1, |ui| {
                    egui::ComboBox::from_id_salt("default-encoding").selected_text(session::encoding_from_code(self.preferences.default_encoding).label()).show_ui(ui, |ui| { for value in ENCODINGS { ui.selectable_value(&mut self.preferences.default_encoding, session::encoding_code(value), value.label()); } });
                });
                row(
                    ui,
                    &q,
                    m,
                    "Fins de ligne par défaut",
                    "Nouveaux fichiers uniquement.",
                    self.preferences.default_eol != 0,
                    |ui| {
                        for value in EOL_ALL {
                            ui.radio_value(
                                &mut self.preferences.default_eol,
                                session::eol_code(value),
                                value.label(),
                            );
                        }
                    },
                );
                let was = self.preferences.recent_enabled;
                bool_setting(
                    ui,
                    &q,
                    m,
                    "Conserver les fichiers récents",
                    "Désactiver efface la liste, pas les fichiers ni les brouillons.",
                    &mut self.preferences.recent_enabled,
                    true,
                );
                if was && !self.preferences.recent_enabled {
                    self.recent.clear();
                }
                row(
                    ui,
                    &q,
                    m,
                    "Nombre de fichiers récents",
                    "Application · Entre 1 et 100 entrées.",
                    self.preferences.recent_limit != 10,
                    |ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.preferences.recent_limit).range(1..=100),
                        );
                        if ui.button("Effacer la liste des récents").clicked() {
                            self.recent.clear();
                            self.session_dirty = true;
                        }
                    },
                );
                ui.label(format!("Session locale : {}", self.session_file.display()));
            }
            5 => {
                row(ui, &q, m, "Sauvegarde portable", "Contient les pages, images, corbeille et versions de l’espace. Vérifiez le destinataire avant de partager.", false, |ui| {
                    if ui.button("Exporter l’espace de travail…").clicked() { if let Some(path) = rfd::FileDialog::new().set_file_name("notepad-rs-backup.json").save_file() { self.backup_to(ctx, &path); } }
                    if ui.button("Importer les pages d’une sauvegarde…").clicked() { self.choose_import(); }
                    if ui.button("Récupérer les pages du fichier .bak").clicked() { self.prepare_import(self.session_file.with_extension("json.bak")); }
                });
                row(ui, &q, m, "Sauvegardes planifiées", "Une nouvelle archive est créée périodiquement. Les archives existantes ne sont jamais supprimées automatiquement.", self.preferences.backup_directory.is_some(), |ui| {
                    ui.label(self.preferences.backup_directory.as_deref().unwrap_or("Désactivées"));
                    if ui.button("Choisir le dossier…").clicked() { if let Some(path) = rfd::FileDialog::new().pick_folder() { self.preferences.backup_directory = Some(path.to_string_lossy().into()); } }
                    if ui.button("Désactiver la planification").clicked() { self.preferences.backup_directory = None; }
                    ui.add(egui::DragValue::new(&mut self.preferences.backup_minutes).range(1..=1440).suffix(" min"));
                    if let Some(error) = &self.options.backup_error { ui.label(error); }
                });
                bool_setting(ui,&q,m,"Versions automatiques","Conserve périodiquement les pages modifiées, y compris les objets des moodboards.",&mut self.preferences.automatic_versions,true);
                row(ui, &q, m, "Versions par page", "À la prochaine création de version, les plus anciennes au-delà de la limite seront retirées. Les archives de sauvegarde restent conservées.", self.preferences.version_limit != 50 || self.preferences.version_seconds != 30, |ui| { ui.add(egui::DragValue::new(&mut self.preferences.version_limit).range(1..=200).suffix(" versions")); ui.add(egui::DragValue::new(&mut self.preferences.version_seconds).range(30..=3600).suffix(" s entre versions")); if ui.button("Historique de cette page").clicked() { self.options.history = true; } });
                ui.label(format!(
                    "Dernière sauvegarde locale : {}",
                    self.options
                        .last_saved
                        .as_deref()
                        .unwrap_or("Pas encore enregistrée dans cette fenêtre")
                ));
            }
            6 => {
                ui.label(
                    "Synchronisation par dossier partagé. Les conflits sont conservés en copies.",
                );
                ui.label(format!(
                    "Destination : {}",
                    self.sync_directory.as_deref().unwrap_or("Non configurée")
                ));
                if ui
                    .button("Choisir le dossier de synchronisation…")
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.sync_directory = Some(path.to_string_lossy().into());
                        self.sync_baseline.clear();
                        self.options.sync_paused = false;
                        self.session_dirty = true;
                    }
                }
                if self.sync_directory.is_some() {
                    ui.checkbox(
                        &mut self.options.sync_paused,
                        "Mettre en pause dans cette fenêtre",
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.preferences.sync_seconds)
                            .range(5..=3600)
                            .suffix(" s entre vérifications"),
                    );
                    if ui.button("Synchroniser maintenant / Réessayer").clicked() {
                        self.options.sync_paused = false;
                        self.last_sync = Instant::now() - Duration::from_secs(3601);
                        self.sync_workspace(ctx);
                    }
                    if ui.button("Désactiver la synchronisation").clicked() {
                        self.sync_directory = None;
                        self.session_dirty = true;
                    }
                }
                ui.label(format!(
                    "Dernière réussite : {}",
                    self.options
                        .last_sync
                        .as_deref()
                        .unwrap_or("Aucune dans cette fenêtre")
                ));
                if let Some(error) = &self.options.sync_error {
                    ui.colored_label(egui::Color32::RED, error);
                }
                ui.separator();
                ui.label("Copies de conflit — ouvrir pour comparer avant de supprimer :");
                let conflicts: Vec<_> = self
                    .tabs
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| {
                        !t.workspace.trashed
                            && (t.workspace.conflict_of.is_some()
                                || t.page_title.to_lowercase().ends_with(" — conflit")
                                || t.page_title.to_lowercase().ends_with(" conflict"))
                    })
                    .map(|(i, t)| (i, ui_design::page_title(t)))
                    .collect();
                if conflicts.is_empty() {
                    ui.label("Aucune copie de conflit identifiée.");
                }
                for (i, title) in conflicts {
                    if ui.button(title).clicked() {
                        self.options.compare_source = Some(self.tabs[i].doc.id);
                        self.options.compare_target = self
                            .tabs
                            .iter()
                            .find(|t| {
                                Some(&t.workspace.uid)
                                    == self.tabs[i].workspace.conflict_of.as_ref()
                            })
                            .map(|t| t.doc.id);
                    }
                }
            }
            7 => {
                ui.label("Raccourcis supplémentaires · Saisir par exemple Ctrl+Maj+J. Les raccourcis standards restent disponibles.");
                for (id, label) in CUSTOM_COMMANDS {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(label);
                        let draft = self
                            .options
                            .shortcut_drafts
                            .entry(id.into())
                            .or_insert_with(|| {
                                self.preferences
                                    .shortcuts
                                    .get(id)
                                    .cloned()
                                    .unwrap_or_default()
                            });
                        ui.add(
                            egui::TextEdit::singleline(draft)
                                .desired_width(170.0)
                                .hint_text("Ctrl+Maj+J"),
                        );
                        if ui.button("Affecter").clicked() {
                            match validate_shortcut(id, draft, &self.preferences.shortcuts) {
                                Ok(()) => {
                                    self.preferences.shortcuts.insert(id.into(), draft.clone());
                                    self.options.shortcut_error.clear();
                                }
                                Err(error) => self.options.shortcut_error = error,
                            }
                        }
                        if ui.small_button("Retirer").clicked() {
                            self.preferences.shortcuts.remove(id);
                            draft.clear();
                        }
                    });
                }
                if !self.options.shortcut_error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.options.shortcut_error);
                }
                ui.label("Ctrl+, : Paramètres · Ctrl+Maj+P : Palette · Ctrl+Maj+F : Recherche dans l’espace · Échap : quitter le mode concentration.");
                for text in [
                    "Ctrl+N — Nouvelle page",
                    "Ctrl+O — Ouvrir",
                    "Ctrl+S — Enregistrer le fichier",
                    "Ctrl+Maj+S — Enregistrer sous",
                    "Ctrl+F — Rechercher dans le document",
                    "Ctrl+H — Remplacer",
                    "Ctrl+G — Atteindre une ligne",
                    "Ctrl+Z / Ctrl+Y — Annuler / Rétablir",
                    "Ctrl+Maj+M — Aperçu Markdown",
                    "F2 — Renommer la page",
                    "Ctrl+P — Imprimer",
                ] {
                    if matches(&q, text) {
                        ui.label(text);
                    }
                }
                if ui.button("Ouvrir la palette de commandes").clicked() {
                    self.open_palette();
                }
            }
            _ => {
                ui.label("Les brouillons, versions et chemins sont conservés dans la session locale. Désactiver les récents ne désactive pas cette conservation. Le dossier de synchronisation reçoit une copie portable de l’espace.");
                if ui.button("Exporter les préférences uniquement…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("notepad-rs-preferences.json")
                        .save_file()
                    {
                        let profile = Profile {
                            preferences: self.preferences.clone(),
                            theme: match self.theme {
                                ThemeMode::System => 0,
                                ThemeMode::Light => 1,
                                ThemeMode::Dark => 2,
                            },
                            font: self.font_family.clone(),
                            font_size: self.default_font_size,
                            wrap: self.default_wrap,
                            status_bar: self.status_bar,
                            scale: ctx.zoom_factor(),
                            ..Default::default()
                        };
                        self.status = match serde_json::to_vec_pretty(&profile)
                            .map_err(std::io::Error::other)
                            .and_then(|b| std::fs::write(path, b))
                        {
                            Ok(()) => "Préférences exportées ; aucune page exportée".into(),
                            Err(e) => e.to_string(),
                        };
                    }
                }
                if ui.button("Importer les préférences…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        match std::fs::read(path)
                            .map_err(|e| e.to_string())
                            .and_then(|b| {
                                serde_json::from_slice::<Profile>(&b).map_err(|e| e.to_string())
                            }) {
                            Ok(mut p) if p.version == 1 => {
                                p.preferences.sanitize();
                                self.preferences = p.preferences;
                                self.font_family = p.font;
                                self.default_font_size = p.font_size.clamp(8.0, 60.0);
                                self.default_wrap = p.wrap;
                                self.status_bar = p.status_bar;
                                self.theme = match p.theme {
                                    1 => ThemeMode::Light,
                                    2 => ThemeMode::Dark,
                                    _ => ThemeMode::System,
                                };
                                if p.scale.is_finite() {
                                    ctx.set_zoom_factor(p.scale.clamp(0.8, 1.75));
                                }
                                self.session_dirty = true;
                            }
                            Ok(_) => {
                                self.status = "Version de préférences non prise en charge".into()
                            }
                            Err(e) => self.status = format!("Préférences non importées : {e}"),
                        }
                    }
                }
                if ui
                    .button("Réinitialiser les préférences (conserver les pages)")
                    .clicked()
                {
                    self.preferences = Default::default();
                    self.theme = ThemeMode::System;
                    self.font_family = None;
                    self.default_font_size = 15.0;
                    self.default_wrap = true;
                    self.status_bar = true;
                    self.design.sidebar = true;
                    ctx.set_zoom_factor(1.0);
                    self.session_dirty = true;
                }
                if ui
                    .button("Copier un diagnostic sans contenu ni chemins")
                    .clicked()
                {
                    ctx.copy_text(format!("Notepad-RS {}\nPages : {}\nSynchronisation configurée : {}\nErreur de session : {}\nErreur de synchronisation : {}", env!("CARGO_PKG_VERSION"), self.tabs.len(), self.sync_directory.is_some(), self.options.save_error.is_some(), self.options.sync_error.is_some()));
                }
            }
        }
    }

    pub(crate) fn open_palette(&mut self) {
        self.options.palette = true;
        self.options.palette_focus = true;
        self.options.palette_query.clear();
        self.options.palette_cursor=0;
    }
    fn palette_ui(&mut self, ctx: &egui::Context) {
        if !self.options.palette { return; }
        let commands=self.command_catalogue();
        let mut open=true;
        let mut picked=None;
        egui::Window::new("Palette de commandes").open(&mut open).default_width(620.0)
            .max_height((ctx.content_rect().height()-48.0).max(200.0)).show(ctx,|ui| {
                let field=ui.add(egui::TextEdit::singleline(&mut self.options.palette_query).hint_text("Commande, réglage ou page…").desired_width(f32::INFINITY));
                if self.options.palette_focus { field.request_focus(); self.options.palette_focus=false; }
                if field.changed() { self.options.palette_cursor=0; }
                let filtered:Vec<_>=commands.iter().filter(|c|c.matches(&self.options.palette_query)).collect();
                ui.label(format!("{} résultats · les options indisponibles indiquent leur condition",filtered.len()));
                let enabled:Vec<_>=filtered.iter().filter(|c|c.reason.is_none()).map(|c|c.id.as_str()).collect();
                let mut moved=false;
                if !enabled.is_empty() {
                    self.options.palette_cursor=self.options.palette_cursor.min(enabled.len()-1);
                    if ui.input(|i|i.key_pressed(egui::Key::ArrowDown)) { self.options.palette_cursor=(self.options.palette_cursor+1)%enabled.len(); moved=true; }
                    if ui.input(|i|i.key_pressed(egui::Key::ArrowUp)) { self.options.palette_cursor=(self.options.palette_cursor+enabled.len()-1)%enabled.len(); moved=true; }
                }
                let current=enabled.get(self.options.palette_cursor).copied();
                egui::ScrollArea::vertical().max_height((ctx.content_rect().height()-180.0).max(100.0)).show(ui,|ui| {
                    for c in &filtered {
                        ui.push_id(&c.id,|ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.small(c.group);
                                let active=current==Some(c.id.as_str());
                                let response=ui.add_enabled(c.reason.is_none(),egui::Button::new(format!("{}{}{}",if c.checked {"✓ "} else {""},c.label,if c.shortcut.is_empty(){String::new()}else{format!("    {}",c.shortcut)})).selected(active));
                                if active && moved {response.scroll_to_me(Some(egui::Align::Center));}
                                if response.clicked() {picked=Some(c.id.clone());}
                            });
                            if let Some(reason)=c.reason { ui.small(reason); }
                        });
                    }
                    if filtered.is_empty() { ui.label("Aucun résultat. Essayez canevas, encodage, sauvegarde ou le titre d’une page."); }
                });
                if ui.input(|i|i.key_pressed(egui::Key::Enter)) {picked=picked.take().or_else(||current.map(str::to_owned));}
                if ui.input(|i|i.key_pressed(egui::Key::Escape)) {self.options.palette=false;}
            });
        self.options.palette &= open;
        if let Some(id)=picked { self.options.palette=false; self.execute_command(ctx,&id); }
    }

    pub(crate) fn page_matches(&self, t: &Tab) -> bool {
        if t.workspace.trashed && !self.options.include_trash {
            return false;
        }
        if self.options.favorites_only && !(t.workspace.favorite || t.workspace.pinned) {
            return false;
        }
        let kind = if t.doc.moodboard {
            3
        } else if t.workspace.visual_blocks {
            2
        } else {
            1
        };
        if self.options.search_kind != 0 && self.options.search_kind != kind {
            return false;
        }
        let q = self.page_search.to_lowercase();
        (self.options.search_scope != 2 && ui_design::page_title(t).to_lowercase().contains(&q))
            || (self.options.search_scope != 1
                && (t.doc.text.to_lowercase().contains(&q)
                    || t.mood_items
                        .iter()
                        .any(|item| item.text.to_lowercase().contains(&q))))
    }
    fn search_window(&mut self, ctx: &egui::Context) {
        if !self.options.search {
            return;
        }
        let mut open = true;
        egui::Window::new("Recherche dans l’espace").open(&mut open).default_width(650.0).default_height(650.0).max_height((ctx.content_rect().height()-48.0).max(200.0)).show(ctx, |ui| {
            ui.add(egui::TextEdit::singleline(&mut self.page_search).hint_text("Titres, texte et notes des moodboards…").desired_width(f32::INFINITY));
            ui.horizontal_wrapped(|ui| { for (i,label) in ["Tous", "Texte", "Blocs", "Moodboards"].iter().enumerate() { ui.selectable_value(&mut self.options.search_kind,i as u8,*label); } });
            ui.horizontal_wrapped(|ui| { for (i,label) in ["Titre et contenu", "Titre", "Contenu"].iter().enumerate() { ui.radio_value(&mut self.options.search_scope,i as u8,*label); } });
            ui.horizontal_wrapped(|ui| { ui.checkbox(&mut self.options.favorites_only,"Favoris"); ui.checkbox(&mut self.options.include_trash,"Inclure la corbeille"); });
            ui.horizontal_wrapped(|ui| { for (i,label) in ["Ordre des pages", "Titre A–Z", "Modification récente"].iter().enumerate() { ui.selectable_value(&mut self.options.sort,i as u8,*label); } });
            if ui.button("Effacer recherche et filtres").clicked() { self.page_search.clear(); self.options.search_kind = 0; self.options.search_scope = 0; self.options.favorites_only = false; self.options.include_trash = false; }
            let mut results: Vec<_> = self.tabs.iter().enumerate().filter(|(_,t)| self.page_matches(t)).map(|(i,t)| (i,ui_design::page_title(t),t.workspace.modified_at.clone())).collect();
            if self.options.sort == 1 { results.sort_by_key(|r|r.1.to_lowercase()); } else if self.options.sort == 2 { results.sort_by(|a,b| b.2.cmp(&a.2)); }
            ui.label(format!("{} résultat(s)",results.len()));
            if results.is_empty() { ui.label("Aucune page dans ce périmètre. Élargissez les filtres ou effacez la recherche."); }
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| { for (i,title,_) in results {
                ui.group(|ui| {
                    if ui.button(&title).clicked() { self.active = i; self.pending_focus = true; }
                    let t = &self.tabs[i];
                    if t.workspace.trashed { ui.label("Dans la corbeille"); }
                    let source = if t.doc.moodboard { t.mood_items.iter().map(|m|m.text.as_str()).collect::<Vec<_>>().join("\n") } else { t.doc.text.clone() };
                    let line = source.lines().find(|l| matches(&self.page_search,l)).unwrap_or("");
                    ui.label(line.chars().take(180).collect::<String>());
                });
            } });
        });
        self.options.search = open;
    }

    pub(crate) fn duplicate_page(&mut self) {
        let mut saved = tab_snapshot(self.active_tab());
        saved.page_id = self.next_id;
        saved.path = None;
        saved.dirty = true;
        saved.text = Some(self.active_tab().doc.text.clone());
        saved.page_title = format!("{} — copie", ui_design::page_title(self.active_tab()));
        saved.workspace.uid = workspace_io::uid();
        saved.workspace.disk_hash = None;
        saved.workspace.conflict_of = None;
        saved.workspace.aliases.clear();
        saved.workspace.trashed = false;
        saved.workspace.read_only = false;
        if let Some(tab) = self.restore_tab(saved) {
            self.tabs.push(tab);
            self.active = self.tabs.len() - 1;
            self.session_dirty = true;
        }
    }
    pub(crate) fn create_version(&mut self, label: &str) {
        let limit = self.preferences.version_limit;
        let tab = self.active_tab_mut();
        let version = session::PageVersion {
            created_at: chrono::Local::now().to_rfc3339(),
            label: label.into(),
            text: tab.doc.text.clone(),
            mood_items: tab.mood_items.clone(),
            canvas_size: tab.workspace.canvas_size,
        };
        tab.workspace.snapshots.push(version);
        if tab.workspace.snapshots.len() > limit {
            tab.workspace
                .snapshots
                .drain(..tab.workspace.snapshots.len() - limit);
        }
        self.session_dirty = true;
    }
    fn history_ui(&mut self, ctx: &egui::Context) {
        if !self.options.history {
            return;
        }
        let mut open = true;
        egui::Window::new(format!("Historique · {}", ui_design::page_title(self.active_tab()))).id(egui::Id::unique("history-center")).open(&mut open).default_width(720.0).show(ctx, |ui| {
            ui.horizontal(|ui| { ui.add(egui::TextEdit::singleline(&mut self.options.version_label).hint_text("Nom de la version (facultatif)")); if ui.button("Créer une version").clicked() { let label = std::mem::take(&mut self.options.version_label); self.create_version(&label); } });
            // Migrate legacy text versions only when the user opens history.
            if !self.active_tab().workspace.versions.is_empty() {
                let old = std::mem::take(&mut self.active_tab_mut().workspace.versions);
                for (i,text) in old.into_iter().enumerate() { self.active_tab_mut().workspace.snapshots.push(session::PageVersion { created_at: "Date inconnue (ancienne version)".into(), label: format!("Version héritée {}",i+1), text, mood_items: Vec::new(), canvas_size: None }); }
                self.session_dirty = true;
            }
            let versions = self.active_tab().workspace.snapshots.clone();
            if versions.is_empty() { ui.label("Aucune version conservée. Créez une version avant une modification importante."); }
            egui::ScrollArea::vertical().id_salt("version-list").max_height(160.0).show(ui, |ui| { for (i,v) in versions.iter().enumerate().rev() {
                if ui.selectable_label(self.options.version_selection == Some(i),format!("{} · {}",v.created_at,v.label)).clicked() { self.options.version_selection = Some(i); }
            } });
            if let Some(version) = self.options.version_selection.and_then(|i|versions.get(i)).cloned() {
                ui.separator();
                ui.label(format!("Version : {} caractères, {} objets · Actuel : {} caractères, {} objets",version.text.chars().count(),version.mood_items.len(),self.active_tab().doc.text.chars().count(),self.active_tab().mood_items.len()));
                egui::ScrollArea::vertical().id_salt("version-preview").max_height(240.0).show(ui, |ui| { ui.columns(2,|cols| { cols[0].label("Version sélectionnée"); cols[0].label(&version.text); cols[1].label("Contenu actuel"); cols[1].label(&self.active_tab().doc.text); }); });
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Restaurer en copie").clicked() { self.duplicate_page(); self.apply_version(version.clone()); self.options.history = false; }
                    if ui.add_enabled(!self.active_tab().workspace.read_only,egui::Button::new("Restaurer (conserver une version de l’actuel)")).clicked() { self.create_version("Avant restauration"); self.apply_version(version); }
                });
            }
        });
        self.options.history &= open;
    }
    fn apply_version(&mut self, version: session::PageVersion) {
        let tab = self.active_tab_mut();
        tab.doc.text = version.text;
        if tab.doc.moodboard {
            tab.mood_items = version.mood_items;
            tab.workspace.canvas_size = version.canvas_size;
        }
        tab.touch();
        self.session_dirty = true;
        self.selected_mood_item = None;
    }
    fn backup_to(&mut self, ctx: &egui::Context, path: &std::path::Path) {
        let result =
            workspace_io::portable(self.session_snapshot(ctx)).and_then(|s| s.save_to(path));
        self.options.backup_error = result
            .as_ref()
            .err()
            .map(|e| format!("Sauvegarde impossible : {e}"));
        self.status = self
            .options
            .backup_error
            .clone()
            .unwrap_or_else(|| format!("Sauvegarde créée : {}", path.display()));
    }
    pub(crate) fn scheduled_backup(&mut self, ctx: &egui::Context) {
        if self.design.capture.is_some() {
            return;
        }
        if self.preferences.automatic_versions
            && self.options.last_version.elapsed()
                >= Duration::from_secs(self.preferences.version_seconds)
        {
            for tab in &mut self.tabs {
                if !tab.doc.dirty {
                    continue;
                }
                let unchanged = tab.workspace.snapshots.last().is_some_and(|v| {
                    v.text == tab.doc.text
                        && v.canvas_size == tab.workspace.canvas_size
                        && serde_json::to_vec(&v.mood_items).ok()
                            == serde_json::to_vec(&tab.mood_items).ok()
                });
                if unchanged {
                    continue;
                }
                tab.workspace.snapshots.push(session::PageVersion {
                    created_at: chrono::Local::now().to_rfc3339(),
                    label: "Version automatique".into(),
                    text: tab.doc.text.clone(),
                    mood_items: tab.mood_items.clone(),
                    canvas_size: tab.workspace.canvas_size,
                });
                let excess = tab
                    .workspace
                    .snapshots
                    .len()
                    .saturating_sub(self.preferences.version_limit);
                if excess > 0 {
                    tab.workspace.snapshots.drain(..excess);
                }
                self.session_dirty = true;
            }
            self.options.last_version = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_secs(
            self.preferences.version_seconds.min(60),
        ));
        if let Some(dir) = self.preferences.backup_directory.clone() {
            if self.options.last_backup.elapsed()
                >= Duration::from_secs(self.preferences.backup_minutes.saturating_mul(60))
            {
                let path = PathBuf::from(dir).join(format!(
                    "notepad-rs-{}.json",
                    chrono::Local::now().format("%Y%m%d-%H%M%S-%f")
                ));
                self.backup_to(ctx, &path);
                self.options.last_backup = Instant::now();
            }
        }
    }
    fn choose_import(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Sauvegarde JSON", &["json", "bak"])
            .pick_file()
        {
            self.prepare_import(path);
        }
    }
    fn prepare_import(&mut self, path: PathBuf) {
        match workspace_io::read(&path) {
            Ok(saved) => {
                self.options.import_summary = format!("Importer {} pages et {} objets. Les pages locales seront conservées. Les identifiants seront renouvelés pour éviter les collisions.",saved.tabs.len(),saved.tabs.iter().map(|t|t.mood_items.len()).sum::<usize>());
                self.options.import_pending = Some(saved);
            }
            Err(e) => self.status = format!("Import impossible : {e}"),
        }
    }
    fn import_confirmation(&mut self, ctx: &egui::Context) {
        if self.options.import_pending.is_none() {
            return;
        }
        let mut apply = false;
        let mut cancel = false;
        egui::Window::new("Importer les pages d’une sauvegarde")
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(&self.options.import_summary);
                ui.horizontal(|ui| {
                    apply = ui.button("Importer ces pages").clicked();
                    cancel = ui.button("Annuler").clicked();
                });
            });
        if cancel {
            self.options.import_pending = None;
        }
        if apply {
            if let Some(mut saved) = self.options.import_pending.take() {
                let mut seen_ids = std::collections::HashSet::new();
                let mut seen_uids = std::collections::HashSet::new();
                if saved.tabs.iter().any(|p| {
                    !seen_ids.insert(p.page_id)
                        || (!p.workspace.uid.is_empty()
                            && !seen_uids.insert(p.workspace.uid.clone()))
                }) {
                    self.status =
                        "Import annulé : identifiants de pages dupliqués dans la sauvegarde."
                            .into();
                    return;
                }
                let mut ids = std::collections::HashMap::new();
                let mut uids = std::collections::HashMap::new();
                for page in &saved.tabs {
                    ids.insert(page.page_id, self.next_id);
                    self.next_id += 1;
                    if !page.workspace.uid.is_empty() {
                        uids.insert(page.workspace.uid.clone(), workspace_io::uid());
                    }
                }
                for page in &mut saved.tabs {
                    if page.text.is_none() && !page.moodboard {
                        self.status = "Import annulé : une page ne contient pas son texte. Exportez une sauvegarde portable depuis son application d’origine.".into();
                        return;
                    }
                    page.path = None;
                    page.dirty = true;
                    page.workspace.disk_hash = None;
                    page.page_id = ids[&page.page_id];
                    page.parent_id = page.parent_id.and_then(|p| ids.get(&p).copied());
                    page.workspace.uid = uids
                        .get(&page.workspace.uid)
                        .cloned()
                        .unwrap_or_else(workspace_io::uid);
                    page.workspace.conflict_of = page
                        .workspace
                        .conflict_of
                        .as_ref()
                        .and_then(|uid| uids.get(uid))
                        .cloned();
                    for text in page
                        .text
                        .iter_mut()
                        .chain(page.workspace.snapshots.iter_mut().map(|v| &mut v.text))
                    {
                        for (old, new) in &uids {
                            if !old.is_empty() {
                                *text =
                                    text.replace(&format!("[[id:{old}|"), &format!("[[id:{new}|"));
                            }
                        }
                    }
                    if let Err(e) =
                        workspace_io::materialize(page, &session::data_dir().join("assets"))
                    {
                        self.status =
                            format!("Import interrompu avant modification des pages : {e}");
                        return;
                    }
                }
                let mut ready = Vec::new();
                for page in saved.tabs {
                    if let Some(tab) = self.restore_tab(page) {
                        ready.push(tab);
                    } else {
                        self.status = "Import annulé : une page est illisible. Les pages locales sont conservées.".into();
                        return;
                    }
                }
                self.status = format!("{} page(s) importée(s)", ready.len());
                self.tabs.extend(ready);
                self.session_dirty = true;
            }
        }
    }
}
