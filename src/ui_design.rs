//! Native workspace shell matching the approved Notepad RS design.
use super::*;
use egui_phosphor::regular as icons;
use std::collections::{HashMap, HashSet};

pub const BLUE: egui::Color32 = egui::Color32::from_rgb(37, 99, 235);
pub const INK: egui::Color32 = egui::Color32::from_rgb(23, 32, 51);
pub const MUTED: egui::Color32 = egui::Color32::from_rgb(71, 85, 105);
pub const CHROME: egui::Color32 = egui::Color32::from_rgb(246, 247, 249);
fn secondary(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_gray(190)
    } else {
        MUTED
    }
}

pub struct State {
    pub sidebar: bool,
    pub sidebar_width: f32,
    pub mobile_navigation: bool,
    pub inspector: bool,
    pub inspector_section: Option<&'static str>,
    pub inspector_active_section: &'static str,
    pub settings: bool,
    pub trash: bool,
    pub rename: Option<u64>,
    pub rename_draft: String,
    pub collapsed: HashSet<u64>,
    pub board_zoom: f32,
    pub board_pan: egui::Vec2,
    pub board_hand: bool,
    pub board_fit_requested: bool,
    pub board_import_requested: bool,
    pub textures: HashMap<String, egui::TextureHandle>,
    pub selected_block: Option<usize>,
    pub capture: Option<(String, String, u32)>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            sidebar: true,
            sidebar_width: 248.0,
            mobile_navigation: false,
            inspector: false,
            inspector_section: None,
            inspector_active_section: "Élément sélectionné",
            settings: false,
            trash: false,
            rename: None,
            rename_draft: String::new(),
            collapsed: HashSet::new(),
            board_zoom: 1.0,
            board_pan: egui::Vec2::ZERO,
            board_hand: false,
            board_fit_requested: false,
            board_import_requested: false,
            textures: HashMap::new(),
            selected_block: None,
            capture: None,
        }
    }
}

pub fn apply(ctx: &egui::Context) {
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size.y = 30.0;
        style.spacing.window_margin = egui::Margin::same(16);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(13.0));
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(20.0, egui::FontFamily::Name(DEFAULT_BOLD_FAMILY.into())),
        );
        style.visuals.selection.bg_fill = if style.visuals.dark_mode {
            egui::Color32::from_rgb(27, 47, 78)
        } else {
            egui::Color32::from_rgb(239, 246, 255)
        };
        let accent = if style.visuals.dark_mode {
            egui::Color32::from_rgb(147, 197, 253)
        } else {
            BLUE
        };
        style.visuals.selection.stroke = egui::Stroke::new(1.0, accent);
        style.visuals.hyperlink_color = accent;
        if style.visuals.dark_mode {
            let graphite = egui::Color32::from_rgb(23, 25, 29);
            let panel = egui::Color32::from_rgb(32, 35, 41);
            let control = egui::Color32::from_rgb(42, 46, 54);
            style.visuals.override_text_color = Some(egui::Color32::from_rgb(235, 238, 243));
            style.visuals.panel_fill = panel;
            style.visuals.window_fill = panel;
            style.visuals.faint_bg_color = graphite;
            style.visuals.extreme_bg_color = egui::Color32::from_rgb(18, 20, 24);
            style.visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, egui::Color32::from_rgb(54, 59, 68));
            style.visuals.widgets.inactive.weak_bg_fill = control;
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(52, 58, 68);
            style.visuals.widgets.active.weak_bg_fill = BLUE;
            style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, BLUE);
        } else {
            style.visuals.override_text_color = Some(INK);
            style.visuals.panel_fill = egui::Color32::WHITE;
            style.visuals.window_fill = egui::Color32::WHITE;
            style.visuals.faint_bg_color = CHROME;
            style.visuals.extreme_bg_color = egui::Color32::WHITE;
            style.visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, egui::Color32::from_gray(224));
            style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(239, 246, 255);
            style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, BLUE);
        }
    });
}

pub fn primary(label: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label.into()).color(egui::Color32::WHITE))
        .fill(BLUE)
        .min_size(egui::vec2(0.0, 38.0))
}

pub fn page_title(tab: &Tab) -> String {
    if tab.page_title.is_empty() {
        tab.doc.title()
    } else {
        tab.page_title.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn main_menus_are_visible_on_one_row_and_page_properties_still_open() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = NotepadApp::new(&cc, None, PathBuf::from("unused-menu-session.json"));
        app.setup_preview("notes", "unused-menu-capture.png");
        let _ = shell(&mut app, &ctx, 1440.0, vec![]);
        let output = shell(&mut app, &ctx, 1440.0, vec![]);
        let locate = |out: &egui::FullOutput, label: &str| {
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        Some(t.pos + egui::vec2(6.0, 6.0))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Menu absent : {label}"))
        };
        let first = locate(&output, "Fichier");
        for label in ["Édition", "Affichage", "Aide", "Page"] {
            let p = locate(&output, label);
            assert!(
                (p.y - first.y).abs() < 3.0,
                "{label} n’est pas sur la même ligne"
            );
        }
        let page = locate(&output, "Page");
        let click = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        shell(
            &mut app,
            &ctx,
            1440.0,
            vec![egui::Event::PointerMoved(page), click(page, true)],
        );
        shell(&mut app, &ctx, 1440.0, vec![click(page, false)]);
        let output = shell(&mut app, &ctx, 1440.0, vec![]);
        let properties = locate(&output, "Propriétés de la page");
        shell(
            &mut app,
            &ctx,
            1440.0,
            vec![
                egui::Event::PointerMoved(properties),
                click(properties, true),
            ],
        );
        shell(&mut app, &ctx, 1440.0, vec![click(properties, false)]);
        assert!(app.design.inspector);
    }
    fn shell(
        app: &mut NotepadApp,
        ctx: &egui::Context,
        width: f32,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 800.0),
            )),
            events,
            ..Default::default()
        };
        let mut frame = eframe::Frame::_new_kittest();
        let mut output = ctx.run_ui(input, |ui| eframe::App::ui(app, ui, &mut frame));
        output.textures_delta.clear();
        output
    }
    #[test]
    fn shell_renders_all_modes_at_desktop_and_small_window_sizes() {
        for mode in ["notes", "blocks", "moodboard"] {
            for width in [640.0, 1100.0, 1440.0] {
                let ctx = egui::Context::default();
                let cc = eframe::CreationContext::_new_kittest(ctx.clone());
                let mut app = NotepadApp::new(&cc, None, PathBuf::from("unused-test-session.json"));
                app.setup_preview(mode, "unused-test-preview.png");
                let before = app
                    .tabs
                    .iter()
                    .map(|t| t.doc.text.clone())
                    .collect::<Vec<_>>();
                let _ = shell(&mut app, &ctx, width, vec![]);
                let result = shell(&mut app, &ctx, width, vec![]);
                assert!(!result.shapes.is_empty());
                assert_eq!(
                    before,
                    app.tabs
                        .iter()
                        .map(|t| t.doc.text.clone())
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    #[test]
    fn page_row_opens_with_single_click_and_sidebar_preferences_restore() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = NotepadApp::new(&cc, None, PathBuf::from("unused-test-session.json"));
        app.setup_preview("notes", "unused-test-preview.png");
        shell(&mut app, &ctx, 1440.0, vec![]);
        let output = shell(&mut app, &ctx, 1440.0, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text().ends_with("Plan du projet") => {
                    Some(t.pos + egui::vec2(8.0, 7.0))
                }
                _ => None,
            })
            .unwrap();
        shell(
            &mut app,
            &ctx,
            1440.0,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        shell(
            &mut app,
            &ctx,
            1440.0,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert_eq!(page_title(app.active_tab()), "Plan du projet");
        app.design.sidebar = false;
        app.design.sidebar_width = 280.0;
        let snapshot = app.session_snapshot(&ctx);
        let restored = NotepadApp::new(
            &cc,
            Some(snapshot),
            PathBuf::from("unused-test-session.json"),
        );
        assert!(!restored.design.sidebar);
        assert_eq!(restored.design.sidebar_width, 280.0);
    }
    #[test]
    fn page_tree_keeps_children_under_their_parent_and_handles_cycles() {
        let mut child = Tab::new(2);
        child.page_parent = Some(1);
        let root = Tab::new(1);
        let other = Tab::new(3);
        let tabs = vec![child, other, root];
        assert_eq!(tree_order(&tabs), vec![(1, 0), (2, 0), (0, 1)]);
        let mut a = Tab::new(4);
        let mut b = Tab::new(5);
        a.page_parent = Some(5);
        b.page_parent = Some(4);
        assert_eq!(tree_order(&[a, b]).len(), 2);
    }
}

/// Depth-first order: preserves sibling order, handles orphans and malformed cycles.
pub fn tree_order(tabs: &[Tab]) -> Vec<(usize, usize)> {
    fn visit(
        i: usize,
        depth: usize,
        tabs: &[Tab],
        seen: &mut HashSet<usize>,
        out: &mut Vec<(usize, usize)>,
    ) {
        if !seen.insert(i) {
            return;
        }
        out.push((i, depth));
        for child in 0..tabs.len() {
            if tabs[child].page_parent == Some(tabs[i].doc.id) {
                visit(child, depth + 1, tabs, seen, out);
            }
        }
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for i in 0..tabs.len() {
        if tabs[i].page_parent.is_none()
            || !tabs.iter().any(|t| Some(t.doc.id) == tabs[i].page_parent)
        {
            visit(i, 0, tabs, &mut seen, &mut out);
        }
    }
    for i in 0..tabs.len() {
        if !seen.contains(&i) {
            visit(i, 0, tabs, &mut seen, &mut out);
        }
    }
    out
}

impl NotepadApp {
    pub(crate) fn horizontal_menu_ui(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        egui::Panel::top("horizontal-main-menu")
            .frame(
                egui::Frame::NONE
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(12, 4)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("main-menu-scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| self.menu_bar(ui, actions));
            });
    }

    pub(crate) fn interface_menu_items(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if ui
            .checkbox(&mut self.design.sidebar, "Navigation des pages")
            .changed()
        {
            self.session_dirty = true;
        }
        ui.separator();
        ui.label("Échelle de l’interface");
        for scale in [1.0, 1.25, 1.5] {
            if ui
                .selectable_label(
                    (ctx.zoom_factor() - scale).abs() < 0.01,
                    format!("{:.0} %", scale * 100.0),
                )
                .clicked()
            {
                ctx.set_zoom_factor(scale);
                self.session_dirty = true;
                ui.close();
            }
        }
    }

    pub(crate) fn status_ui(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        egui::Panel::bottom("workspace-status")
            .frame(
                egui::Frame::NONE
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(16, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let label = if self.options.save_error.is_some() {
                        "Erreur de sauvegarde locale — ouvrir Paramètres"
                    } else if self.session_dirty {
                        "Brouillon local : enregistrement en attente…"
                    } else {
                        "Brouillon conservé localement"
                    };
                    ui.label(
                        egui::RichText::new(format!("{}  {label}", icons::CHECK_CIRCLE)).size(13.0),
                    );
                    ui.label(if self.active_tab().doc.path.is_none() { "Fichier : non associé" } else if self.active_tab().doc.dirty { "Fichier : modifications non enregistrées" } else { "Fichier : enregistré" });
                    if !self.active_tab().doc.moodboard {
                        let doc = &self.active_tab().doc;
                        let total = doc.text.chars().count();
                        let (line, col) = line_col(&doc.text, doc.cursor_char);
                        ui.label(format!("Ln {line}, Col {col}"));
                        ui.label(if doc.selection_chars > 0 {
                            format!("{} sur {total} caractère(s)", doc.selection_chars)
                        } else {
                            format!("{total} caractère(s)")
                        });
                    }
                    if !self.status.is_empty() {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&self.status).size(13.0))
                                .truncate(),
                        )
                        .on_hover_text(&self.status);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button(
                            format!("Détails · {}", self.active_tab().doc.eol.short()),
                            |ui| {
                                let tab = self.active_tab();
                                let (line, col) = line_col(&tab.doc.text, tab.doc.cursor_char);
                                ui.label(format!("Ln {line}, Col {col}"));
                                let total = tab.doc.text.chars().count();
                                if tab.doc.selection_chars > 0 {
                                    ui.label(format!(
                                        "{} sur {total} caractère(s)",
                                        tab.doc.selection_chars
                                    ));
                                } else {
                                    ui.label(format!("{total} caractère(s)"));
                                }
                                let encoding = tab.doc.encoding;
                                let eol = tab.doc.eol;
                                ui.separator();
                                ui.label("Encodage du prochain enregistrement (texte conservé)");
                                for choice in ENCODINGS {
                                    if ui
                                        .selectable_label(choice == encoding, choice.label())
                                        .clicked()
                                    {
                                        actions.push(Action::SetEncoding(choice));
                                        ui.close();
                                    }
                                }
                                ui.menu_button("Rouvrir les octets d’origine avec…", |ui| {
                                    let allowed = !self.active_tab().doc.dirty && self.active_tab().doc.raw.is_some();
                                    if !allowed { ui.label("Enregistrez le brouillon avant de rouvrir les octets du fichier."); }
                                    for choice in ENCODINGS { if ui.add_enabled(allowed, egui::Button::new(choice.label())).clicked() { actions.push(Action::ReopenEncoding(choice)); ui.close(); } }
                                });
                                ui.separator();
                                for choice in EOL_ALL {
                                    if ui.selectable_label(choice == eol, choice.label()).clicked()
                                    {
                                        actions.push(Action::SetEol(choice));
                                        ui.close();
                                    }
                                }
                            },
                        );
                        let zoom = if self.active_tab().doc.moodboard {
                            self.design.board_zoom * 100.0
                        } else {
                            self.active_tab().doc.font_size / 15.0 * 100.0
                        };
                        ui.label(egui::RichText::new(format!("{zoom:.0} %")).size(13.0));
                        if self.sync_directory.is_some() {
                            ui.label(egui::RichText::new(if self.options.sync_paused { "Synchronisation : en pause" } else if self.options.sync_error.is_some() { "Synchronisation : erreur" } else if self.options.last_sync.is_some() { "Synchronisation : dernière vérification réussie" } else { "Synchronisation : en attente" }).size(13.0));
                        }
                    });
                });
            });
    }

    pub(crate) fn document_body(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        actions: &mut Vec<Action>,
    ) -> bool {
        let available = ui.available_width();
        let width = available.min(
            self.active_tab()
                .workspace
                .reading_width
                .unwrap_or(self.preferences.reading_width),
        );
        let margin = ((available - width) / 2.0).max(12.0);
        let mut typed = false;
        ui.horizontal_top(|ui| {
            ui.add_space(margin);
            ui.allocate_ui_with_layout(
                egui::vec2((available - margin * 2.0).max(100.0), ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.add_space(if self.active_tab().workspace.visual_blocks {
                        8.0
                    } else {
                        32.0
                    });
                    ui.label(egui::RichText::new(page_title(self.active_tab())).font(
                        egui::FontId::new(36.0, egui::FontFamily::Name(DEFAULT_BOLD_FAMILY.into())),
                    ));
                    ui.add_space(8.0);
                    if self.design.capture.is_some() {
                        ui.label(
                            egui::RichText::new("26 octobre 2026")
                                .size(14.0)
                                .color(secondary(ui)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(if self.active_tab().workspace.visual_blocks {
                                "Page de blocs"
                            } else {
                                "Page texte"
                            })
                            .size(13.0)
                            .color(secondary(ui)),
                        );
                    }
                    ui.add_space(28.0);
                    if self.active_tab().doc.markdown && self.show_format_bar {
                        self.format_bar(ui, actions);
                    }
                    if self.active_tab().workspace.read_only {
                        ui.label("Lecture seule — déverrouiller dans Propriétés");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.label(&self.active_tab().doc.text);
                        });
                    } else if self.active_tab().workspace.visual_blocks {
                        let id = self.active_tab().doc.id;
                        typed = blocks::editor_selected(
                            ui,
                            id,
                            &mut self.tabs[self.active].doc.text,
                            &mut self.design.selected_block,
                        );
                        if typed {
                            self.active_tab_mut().touch();
                        }
                    } else if self.active_tab().doc.markdown && self.show_markdown_preview {
                        let height = ui.available_height();
                        ui.columns(2, |columns| {
                            columns[0].set_min_height(height);
                            typed = self.editor_ui(&mut columns[0], ctx);
                            self.markdown_preview_ui(&mut columns[1]);
                        });
                    } else {
                        typed = self.editor_ui(ui, ctx);
                    }
                },
            );
        });
        typed
    }

    pub(crate) fn moodboard_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if self.active_tab().workspace.read_only {
            ui.label("Moodboard en lecture seule");
            if ui.button("Autoriser les modifications").clicked() {
                self.active_tab_mut().workspace.read_only = false;
                self.session_dirty = true;
            } else {
                ui.disable();
            }
        }
        let current = self.active_tab().doc.id;
        if self.selected_mood_tab != Some(current) {
            self.selected_mood_tab = Some(current);
            self.selected_mood_item = None;
            self.design.board_pan = egui::Vec2::ZERO;
            self.design.board_zoom = 1.0;
            self.design.inspector = true;
        }
        let before = serde_json::to_vec(&self.active_tab().mood_items).ok();
        let canvas_before = self.active_tab().workspace.canvas_size;
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(page_title(self.active_tab())).font(egui::FontId::new(
                30.0,
                egui::FontFamily::Name(DEFAULT_BOLD_FAMILY.into()),
            )),
        );
        ui.add_space(12.0);
        let mut import = std::mem::take(&mut self.design.board_import_requested);
        let fit = std::mem::take(&mut self.design.board_fit_requested);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add(primary(format!("{} Ajouter", icons::PLUS)))
                .clicked()
            {
                self.execute_command(&ui.ctx().clone(), "board-note");
            }
            import |= ui.button(format!("{} Image…", icons::IMAGE)).clicked();
            ui.selectable_value(&mut self.design.board_hand, false, icons::CURSOR)
                .on_hover_text("Sélectionner");
            ui.selectable_value(&mut self.design.board_hand, true, icons::HAND)
                .on_hover_text("Déplacer le canevas");
            if ui
                .button(format!("{} Canevas", icons::SLIDERS_HORIZONTAL))
                .clicked()
            {
                self.design.inspector = true;
                self.design.inspector_section = Some("Canevas");
                self.design.inspector_active_section = "Canevas";
            }
            if let Some([w, h]) = self.active_tab().workspace.canvas_size {
                ui.colored_label(secondary(ui), format!("{w} × {h} px"));
            }
        });
        ui.add_space(12.0);
        if import {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
                .pick_file()
            {
                self.active_tab_mut().mood_items.push(session::MoodItem {
                    image_path: Some(path.to_string_lossy().into_owned()),
                    text: String::new(),
                    x: 50.0,
                    y: 50.0,
                    width: 360.0,
                    height: 270.0,
                    ..Default::default()
                });
                self.selected_mood_item = Some(self.active_tab().mood_items.len() - 1);
            }
        }
        let (canvas, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let rect = canvas.rect;
        let appearance = self.active_tab().workspace.canvas_appearance.clone();
        let fill = if ui.visuals().dark_mode {
            egui::Color32::from_gray(28)
        } else {
            CHROME
        };
        painter.rect_filled(rect, 0.0, fill);
        if fit {
            let max =
                self.active_tab()
                    .mood_items
                    .iter()
                    .fold(egui::vec2(500.0, 400.0), |v, item| {
                        v.max(egui::vec2(
                            item.x + item.width + 30.0,
                            item.y + item.height + 30.0,
                        ))
                    });
            let max = self
                .active_tab()
                .workspace
                .canvas_size
                .map(|[w, h]| egui::vec2(w as f32, h as f32))
                .unwrap_or(max);
            self.design.board_zoom = ((rect.width() - 30.0) / max.x)
                .min((rect.height() - 30.0) / max.y)
                .clamp(0.02, 1.5);
            self.design.board_pan = egui::vec2(8.0, 8.0);
        }
        if self.design.board_hand && canvas.dragged() {
            self.design.board_pan += canvas.drag_delta();
        }
        if canvas.clicked() {
            self.selected_mood_item = None;
        }
        let zoom = self.design.board_zoom;
        let pan = self.design.board_pan;
        let canvas_fill = appearance
            .background
            .map(|[r, g, b]| egui::Color32::from_rgb(r, g, b))
            .unwrap_or(ui.visuals().panel_fill);
        if self.active_tab().workspace.canvas_size.is_none() {
            painter.rect_filled(rect, 0.0, canvas_fill);
        }
        if let Some([w, h]) = self.active_tab().workspace.canvas_size {
            let bounds =
                egui::Rect::from_min_size(rect.min + pan, egui::vec2(w as f32, h as f32) * zoom);
            painter.rect_filled(bounds, 0.0, canvas_fill);
            if !appearance.hide_border {
                painter.rect_stroke(
                    bounds,
                    0.0,
                    egui::Stroke::new(2.0, BLUE),
                    egui::StrokeKind::Inside,
                );
            }
        }
        let dot = if ui.visuals().dark_mode {
            egui::Color32::from_gray(65)
        } else {
            egui::Color32::from_rgb(204, 213, 226)
        };
        let grid_size = appearance
            .grid_size
            .unwrap_or(self.preferences.grid_size)
            .clamp(8.0, 200.0);
        let step = (grid_size * zoom).max(8.0);
        let mut x = rect.left() + pan.x.rem_euclid(step);
        while appearance.grid.unwrap_or(self.preferences.grid) && x < rect.right() {
            let mut y = rect.top() + pan.y.rem_euclid(step);
            while y < rect.bottom() {
                if !appearance.lines {
                    painter.circle_filled(egui::pos2(x, y), 0.8, dot);
                }
                y += step;
            }
            if appearance.lines {
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(0.5, dot),
                );
            }
            x += step;
        }
        if appearance.lines && appearance.grid.unwrap_or(self.preferences.grid) {
            let mut y = rect.top() + pan.y.rem_euclid(step);
            while y < rect.bottom() {
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    egui::Stroke::new(0.5, dot),
                );
                y += step;
            }
        }
        if self.active_tab().mood_items.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Déposez une image ou ajoutez une note",
                egui::FontId::proportional(18.0),
                secondary(ui),
            );
        }
        let mut selected = None;
        for i in 0..self.tabs[self.active].mood_items.len() {
            if self.tabs[self.active].mood_items[i].hidden {
                continue;
            }
            let path = self.tabs[self.active].mood_items[i].image_path.clone();
            if let Some(path) = &path {
                if !self.design.textures.contains_key(path) {
                    if let Ok(bytes) = std::fs::read(path) {
                        if let Ok(image) = image::load_from_memory(&bytes) {
                            let rgba = image.to_rgba8();
                            let color = egui::ColorImage::from_rgba_unmultiplied(
                                [rgba.width() as usize, rgba.height() as usize],
                                rgba.as_raw(),
                            );
                            let texture = ctx.load_texture(
                                format!("board-{path}"),
                                color,
                                egui::TextureOptions::LINEAR,
                            );
                            self.design.textures.insert(path.clone(), texture);
                        }
                    }
                }
            }
            let texture = path
                .as_ref()
                .and_then(|p| self.design.textures.get(p))
                .map(|t| t.id());
            let active = self.selected_mood_item == Some(i);
            let hand = self.design.board_hand;
            let page_locked = self.active_tab().workspace.read_only;
            let snap = appearance.snap.unwrap_or(self.preferences.snap);
            let item = &mut self.tabs[self.active].mood_items[i];
            let size = egui::vec2(item.width, item.height) * zoom;
            egui::Area::new(egui::Id::unique(("board-item", current, i)))
                .fixed_pos(rect.min + pan + egui::vec2(item.x, item.y) * zoom)
                .constrain(false)
                .movable(false)
                .show(ctx, |ui| {
                    ui.set_clip_rect(rect);
                    ui.set_width(size.x);
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                    ui.visuals_mut().override_text_color = Some(INK);
                    let fill = if texture.is_some() {
                        egui::Color32::WHITE
                    } else {
                        match item.color {
                            1 => egui::Color32::from_rgb(228, 232, 211),
                            2 => egui::Color32::from_rgb(222, 236, 247),
                            3 => egui::Color32::from_rgb(245, 222, 225),
                            _ => egui::Color32::from_rgb(255, 246, 218),
                        }
                    };
                    let frame = egui::Frame::NONE
                        .fill(fill)
                        .corner_radius(5)
                        .inner_margin(egui::Margin::same(if texture.is_some() { 0 } else { 12 }));
                    let result = frame.show(ui, |ui| {
                        let response = if let Some(texture) = texture {
                            ui.add(
                                egui::Image::new((texture, size))
                                    .sense(egui::Sense::click_and_drag()),
                            )
                        } else {
                            ui.set_min_height((size.y - 24.0).max(40.0));
                            let drag = ui.add(
                                egui::Label::new(egui::RichText::new("Déplacer").color(INK))
                                    .sense(egui::Sense::click_and_drag()),
                            );
                            ui.add(
                                egui::TextEdit::multiline(&mut item.text)
                                    .interactive(!item.locked && !page_locked)
                                    .font(egui::FontId::proportional(17.0 * zoom))
                                    .desired_width((size.x - 24.0).max(40.0))
                                    .desired_rows(3)
                                    .frame(egui::Frame::NONE),
                            );
                            drag
                        };
                        if response.clicked() || response.drag_started() {
                            selected = Some(i);
                        }
                        if response.dragged() {
                            if hand || item.locked || page_locked {
                            } else {
                                item.x = (item.x + response.drag_delta().x / zoom).max(0.0);
                                item.y = (item.y + response.drag_delta().y / zoom).max(0.0);
                            }
                        }
                        if response.drag_stopped() && snap && !item.locked && !page_locked {
                            item.x = (item.x / grid_size).round() * grid_size;
                            item.y = (item.y / grid_size).round() * grid_size;
                        }
                        if let Some(link) = &item.link {
                            if !link.is_empty() {
                                ui.hyperlink_to("Ouvrir le lien", link);
                            }
                        }
                        if path.is_some() && texture.is_none() {
                            ui.colored_label(
                                egui::Color32::from_rgb(185, 28, 28),
                                "Image introuvable ou illisible",
                            );
                        }
                    });
                    if active {
                        ui.painter().rect_stroke(
                            result.response.rect,
                            5.0,
                            egui::Stroke::new(2.0, BLUE),
                            egui::StrokeKind::Outside,
                        );
                        let handle = egui::Rect::from_center_size(
                            result.response.rect.right_bottom(),
                            egui::vec2(10.0, 10.0),
                        );
                        let resize = ui.interact(
                            handle,
                            egui::Id::unique(("board-resize", current, i)),
                            egui::Sense::drag(),
                        );
                        ui.painter().rect_filled(handle, 2.0, BLUE);
                        if resize.dragged() && !item.locked && !page_locked {
                            let ratio = item.height / item.width.max(1.0);
                            item.width =
                                (item.width + resize.drag_delta().x / zoom).clamp(20.0, 16384.0);
                            item.height =
                                (item.height + resize.drag_delta().y / zoom).clamp(20.0, 16384.0);
                            if item.keep_ratio {
                                item.height = item.width * ratio;
                            }
                        }
                    }
                });
        }
        if let Some(i) = selected {
            self.selected_mood_item = Some(i);
            self.design.inspector = true;
        }
        egui::Area::new(egui::Id::unique("canvas-zoom"))
            .fixed_pos(rect.center_bottom() - egui::vec2(140.0, 48.0))
            .constrain(false)
            .movable(false)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                egui::Frame::group(ui.style())
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(12, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .button(icons::MINUS)
                                .on_hover_text("Réduire le zoom")
                                .clicked()
                            {
                                self.design.board_zoom = (self.design.board_zoom / 1.15).max(0.02);
                            }
                            ui.label(format!("{} %", (self.design.board_zoom * 100.0).round()));
                            if ui
                                .button(icons::PLUS)
                                .on_hover_text("Agrandir le zoom")
                                .clicked()
                            {
                                self.design.board_zoom = (self.design.board_zoom * 1.15).min(2.5);
                            }
                            if ui.button("Ajuster").clicked() {
                                let max = self.active_tab().mood_items.iter().fold(
                                    egui::vec2(300.0, 200.0),
                                    |v, item| {
                                        v.max(egui::vec2(
                                            item.x + item.width + 24.0,
                                            item.y + item.height + 24.0,
                                        ))
                                    },
                                );
                                let max = self
                                    .active_tab()
                                    .workspace
                                    .canvas_size
                                    .map(|[w, h]| egui::vec2(w as f32, h as f32))
                                    .unwrap_or(max);
                                self.design.board_zoom = ((rect.width() - 24.0) / max.x)
                                    .min((rect.height() - 72.0) / max.y)
                                    .clamp(0.02, 1.5);
                                self.design.board_pan = egui::vec2(8.0, 8.0);
                            }
                        });
                    });
            });
        if before != serde_json::to_vec(&self.active_tab().mood_items).ok()
            || canvas_before != self.active_tab().workspace.canvas_size
        {
            self.active_tab_mut().doc.dirty = true;
            self.session_dirty = true;
        }
    }

    pub(crate) fn setup_preview(&mut self, mode: &str, path: &str) {
        self.theme = ThemeMode::Light;
        if mode.ends_with("dark") {
            self.theme = ThemeMode::Dark;
        }
        self.tabs.clear();
        self.next_id = 0;
        self.new_tab();
        self.active_tab_mut().page_title = "Projet atelier".into();
        self.new_tab();
        {
            let t = self.active_tab_mut();
            t.page_title = "Notes de recherche".into();
            t.page_parent = Some(0);
            t.doc.text="Créer un espace simple pour écrire, organiser et relier les idées.\n\nLes idées à retenir\n\nUn espace d’écriture calme et structuré aide à mieux se concentrer. Il doit rester simple, tout en offrant la possibilité d’organiser les idées et de les relier entre elles naturellement.\n\nL’objectif est de proposer un outil léger et agréable pour un usage quotidien : prise de notes, structuration et exploration visuelle.\n\n1. Garder une interface claire et apaisante.\n2. Faciliter l’organisation hiérarchique des idées.\n3. Passer facilement du texte aux visuels.".into();
        }
        self.new_tab();
        {
            let t = self.active_tab_mut();
            t.page_title = "Plan du projet".into();
            t.page_parent = Some(0);
            t.workspace.visual_blocks = true;
            t.doc.markdown = true;
            t.doc.text="Une idée claire, des étapes concrètes.\n\n## Les prochaines étapes\n\nCette section regroupe les actions prioritaires pour avancer sereinement sur le projet. Chaque étape permet de structurer le travail et de garder une vue d’ensemble.\n\n- [x] Définir les objectifs\n\n- [ ] Rassembler les références\n\n- [ ] Préparer le moodboard\n\n| Étape | Statut |\n| --- | --- |\n| Recherche | En cours |\n| Maquette | À venir |\n".into();
        }
        self.new_tab();
        self.active_tab_mut().page_title = "Réunion".into();
        self.active_tab_mut().workspace.favorite = true;
        self.new_moodboard();
        {
            let t = self.active_tab_mut();
            t.page_title = "Ambiance naturelle".into();
            t.mood_items.clear();
            for (file, x, y, w, h) in [
                ("chaise.png", 30.0, 80.0, 420.0, 300.0),
                ("travertin.png", 480.0, 230.0, 280.0, 210.0),
                ("vase.png", 30.0, 410.0, 420.0, 300.0),
            ] {
                t.mood_items.push(session::MoodItem {
                    image_path: Some(format!(
                        "{}/samples/design-assets/{file}",
                        env!("CARGO_MANIFEST_DIR")
                    )),
                    x,
                    y,
                    width: w,
                    height: h,
                    text: String::new(),
                    ..Default::default()
                });
            }
            t.mood_items.push(session::MoodItem {
                text: "Direction\n\nLumière douce, matières brutes, lignes simples.".into(),
                x: 500.0,
                y: 60.0,
                width: 240.0,
                height: 150.0,
                ..Default::default()
            });
            t.mood_items.push(session::MoodItem {
                text: "À explorer\n\nBois clair · lin · céramique".into(),
                x: 480.0,
                y: 470.0,
                width: 280.0,
                height: 150.0,
                color: 1,
                ..Default::default()
            });
        }
        self.active = if mode.starts_with("blocks") {
            2
        } else if mode.starts_with("moodboard") || mode.starts_with("close-dialog") {
            4
        } else {
            1
        };
        if mode.starts_with("blocks") {
            self.design.selected_block = Some(1);
            self.design.inspector = true;
        }
        if mode.starts_with("moodboard") {
            if mode.contains("canvas") {
                self.active_tab_mut().workspace.canvas_size = Some([1920, 1080]);
            }
            self.selected_mood_tab = Some(self.active_tab().doc.id);
            self.selected_mood_item = Some(if mode.contains("note-selection") {
                3
            } else {
                0
            });
            if mode.contains("properties") {
                self.design.inspector_section = Some("Canevas");
                self.design.inspector_active_section = "Canevas";
            }
            self.design.board_zoom = 0.8;
        }
        for tab in &mut self.tabs {
            tab.doc.dirty = false;
        }
        if mode.starts_with("close-dialog") {
            self.active_tab_mut().doc.dirty = true;
            self.pending_quit = true;
        }
        if mode.starts_with("settings") {
            self.design.settings = true;
            self.options.category = if mode.contains("editor") {
                2
            } else if mode.contains("sync") {
                6
            } else {
                0
            };
        }
        if mode.starts_with("search") {
            self.options.search = true;
            self.page_search = "projet".into();
        }
        if mode.starts_with("palette") {
            self.open_palette();
        }
        self.design.capture = Some((path.into(), mode.into(), 0));
        self.session_dirty = false;
    }

    pub(crate) fn capture_preview(&mut self, ctx: &egui::Context) {
        let Some((path, _, frame)) = &mut self.design.capture else {
            return;
        };
        *frame += 1;
        if *frame == 8 {
            let path = path.clone();
            let ctx = ctx.clone();
            let handle = ctx.clone();
            ctx.request_screenshot(move |capture| {
                let rgba: Vec<u8> = capture.pixels.iter().flat_map(|p| p.to_array()).collect();
                if let Some(parent) = std::path::Path::new(&path).parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(error) = image::save_buffer(
                    &path,
                    &rgba,
                    capture.size[0] as u32,
                    capture.size[1] as u32,
                    image::ColorType::Rgba8,
                ) {
                    let _ = std::fs::write(format!("{path}.error.txt"), error.to_string());
                }
                handle.send_viewport_cmd(egui::ViewportCommand::Close);
            });
        }
        if *frame < 10 {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
    }

    pub(crate) fn navigation_ui(&mut self, ui: &mut egui::Ui) {
        if self.options.concentration {
            return;
        }
        let narrow = ui.ctx().content_rect().width() < 800.0;
        if narrow && !self.design.mobile_navigation {
            return;
        }
        if narrow {
            let ctx = ui.ctx().clone();
            let mut open = true;
            egui::Window::new("Navigation")
                .open(&mut open)
                .default_width(280.0)
                .default_pos(egui::pos2(8.0, 64.0))
                .show(&ctx, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.page_search).hint_text("Rechercher"),
                    );
                    ui.menu_button("Nouvelle page", |ui| {
                        if ui.button("Texte").clicked() {
                            self.new_page_kind(0);
                            self.design.mobile_navigation = false;
                            ui.close();
                        }
                        if ui.button("Blocs").clicked() {
                            self.new_page_kind(1);
                            self.active_tab_mut().workspace.visual_blocks = true;
                            self.design.mobile_navigation = false;
                            ui.close();
                        }
                        if ui.button("Moodboard").clicked() {
                            self.new_moodboard();
                            self.design.mobile_navigation = false;
                            ui.close();
                        }
                    });
                    if ui.button("Recherche avancée et filtres…").clicked() {
                        self.options.search = true;
                    }
                    egui::ScrollArea::vertical()
                        .max_height((ctx.content_rect().height() - 240.0).max(120.0))
                        .show(ui, |ui| {
                            let query = self.page_search.to_lowercase();
                            for (i, depth) in tree_order(&self.tabs) {
                                if self.tabs[i].workspace.trashed {
                                    continue;
                                }
                                if !query.is_empty()
                                    && !page_title(&self.tabs[i]).to_lowercase().contains(&query)
                                    && !self.tabs[i].doc.text.to_lowercase().contains(&query)
                                {
                                    continue;
                                }
                                self.navigation_row(ui, i, depth);
                            }
                        });
                    ui.separator();
                    if ui.button("Corbeille").clicked() {
                        self.design.trash = true;
                    }
                    if ui.button("Paramètres").clicked() {
                        self.design.settings = !self.design.settings;
                        self.design.mobile_navigation = false;
                    }
                });
            if !open {
                self.design.mobile_navigation = false;
            }
            return;
        }
        if !narrow && !self.design.sidebar {
            return;
        }
        let fill = if ui.visuals().dark_mode {
            ui.visuals().faint_bg_color
        } else {
            CHROME
        };
        let response=egui::Panel::left("workspace-navigation").default_size(self.design.sidebar_width).min_size(190.0).max_size(340.0).resizable(true)
            .frame(egui::Frame::NONE.fill(fill).inner_margin(egui::Margin::same(16)))
            .show(ui, |ui| {
                ui.add(egui::TextEdit::singleline(&mut self.page_search).hint_text("Rechercher").desired_width(f32::INFINITY).margin(egui::vec2(10.0, 10.0)));
                if ui.button("Recherche avancée et filtres…").clicked() { self.options.search = true; }
                ui.add_space(8.0);
                ui.scope(|ui| {ui.spacing_mut().interact_size.x=ui.available_width();ui.style_mut().visuals.widgets.inactive.weak_bg_fill=BLUE;ui.style_mut().visuals.widgets.hovered.weak_bg_fill=egui::Color32::from_rgb(29,78,216);
                ui.menu_button(egui::RichText::new(format!("{}  Nouvelle page", icons::PLUS)).color(egui::Color32::WHITE), |ui| {
                    ui.visuals_mut().override_text_color=Some(if ui.visuals().dark_mode{egui::Color32::WHITE}else{INK});
                    ui.spacing_mut().interact_size.x=40.0;
                    if ui.button("Page texte").clicked() { self.new_page_kind(0); ui.close(); }
                    if ui.button("Page de blocs").clicked() { self.new_page_kind(1); self.active_tab_mut().workspace.visual_blocks = true; self.active_tab_mut().doc.markdown = true; ui.close(); }
                    if ui.button("Moodboard").clicked() { self.new_moodboard(); ui.close(); }
                    ui.separator();
                    for (label, content) in [("Notes structurées", "## À retenir\n\n## Actions\n\n- [ ] À faire\n"), ("Projet", "## Objectif\n\nDécrire le projet.\n\n## Tâches\n\n- [ ] Première étape\n"), ("Réunion", "## Participants\n\n## Compte rendu\n\n## Décisions\n")] {
                        if ui.button(label).clicked() { self.new_page_kind(1); let t=self.active_tab_mut(); t.page_title=label.into(); t.doc.text=content.into(); t.doc.markdown=true; t.workspace.visual_blocks=true; t.touch(); ui.close(); }
                    }
                    for template in self.preferences.templates.clone() {
                        if ui.button(&template.name).clicked() { self.new_tab(); let t = self.active_tab_mut(); t.doc.moodboard = false; t.doc.markdown = true; t.workspace.visual_blocks = true; t.page_title = template.name; t.doc.text = template.text; t.touch(); ui.close(); }
                    }
                });});
                ui.add_space(16.0);
                let height = (ui.available_height() - 100.0).max(40.0);
                egui::ScrollArea::vertical().id_salt("pages-list").max_height(height).show(ui, |ui| {
                    if self.page_search.is_empty() {
                        ui.horizontal(|ui|{ui.label(icons::STAR);ui.label(egui::RichText::new("Favoris").font(egui::FontId::new(15.0,egui::FontFamily::Name(DEFAULT_BOLD_FAMILY.into()))));});
                        let favorites: Vec<_> = self.tabs.iter().enumerate().filter(|(_, t)| !t.workspace.trashed && (t.workspace.favorite || t.workspace.pinned)).map(|(i,_)| i).collect();
                        if favorites.is_empty() { ui.small("Épinglez vos pages importantes."); }
                        for i in favorites { self.navigation_row(ui, i, 0); }
                        ui.add_space(16.0); ui.separator(); ui.add_space(8.0);
                    }
                    let query = self.page_search.to_lowercase(); let mut count=0;
                    for (board, heading) in [(false,"Pages"), (true,"Moodboards")] {
                        ui.label(egui::RichText::new(heading).font(egui::FontId::new(15.0,egui::FontFamily::Name(DEFAULT_BOLD_FAMILY.into()))));
                        for (i, depth) in tree_order(&self.tabs) {
                            let t = &self.tabs[i]; if t.workspace.trashed || t.doc.moodboard != board { continue; }
                            if !self.page_matches(t) { continue; }
                            let mut parent=t.page_parent; let mut hidden=false; let mut seen=HashSet::new();
                            while let Some(id)=parent { if !seen.insert(id) { break; } if self.design.collapsed.contains(&id) && query.is_empty() { hidden=true; break; } parent=self.tabs.iter().find(|t| t.doc.id==id).and_then(|t| t.page_parent); }
                            if hidden { continue; } self.navigation_row(ui,i,depth); count+=1;
                        }
                        ui.add_space(16.0);
                    }
                    if count==0 && !query.is_empty() { ui.label("Aucun résultat"); if ui.button("Effacer la recherche").clicked() { self.page_search.clear(); } }
                });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    if ui.button(format!("{}  Paramètres",icons::GEAR)).clicked() { self.design.settings=!self.design.settings; }
                    if ui.button(format!("{}  Corbeille",icons::TRASH)).clicked() { self.design.trash=true; }
                    ui.separator();
                });
            });
        self.design.sidebar_width = response.response.rect.width().clamp(190.0, 340.0);
    }

    fn navigation_row(&mut self, ui: &mut egui::Ui, i: usize, depth: usize) {
        let id = self.tabs[i].doc.id;
        let title = page_title(&self.tabs[i]);
        let has_children = self
            .tabs
            .iter()
            .any(|t| t.page_parent == Some(id) && !t.workspace.trashed);
        ui.push_id(("page-row", id), |ui| {
            ui.horizontal(|ui| {
                ui.add_space((depth.min(5) as f32) * 14.0);
                if has_children {
                    let icon = if self.design.collapsed.contains(&id) {
                        icons::CARET_RIGHT
                    } else {
                        icons::CARET_DOWN
                    };
                    if ui
                        .small_button(icon)
                        .on_hover_text("Afficher ou masquer les sous-pages")
                        .clicked()
                    {
                        if !self.design.collapsed.remove(&id) {
                            self.design.collapsed.insert(id);
                        }
                    }
                }
                let icon = if !self.tabs[i].workspace.icon.is_empty() {
                    self.tabs[i].workspace.icon.as_str()
                } else if self.tabs[i].doc.moodboard {
                    icons::IMAGE
                } else {
                    icons::FILE_TEXT
                };
                if self.design.rename == Some(id) {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.design.rename_draft)
                            .desired_width((ui.available_width() - 12.0).max(80.0)),
                    );
                    r.request_focus();
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        self.design.rename = None;
                    } else if r.lost_focus()
                        || ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        if !self.design.rename_draft.trim().is_empty() {
                            let t = &mut self.tabs[i];
                            if !t.workspace.aliases.contains(&title) {
                                t.workspace.aliases.push(title.clone());
                            }
                            t.page_title = self.design.rename_draft.trim().into();
                            self.session_dirty = true;
                        }
                        self.design.rename = None;
                    }
                } else {
                    let is_active = i == self.active;
                    let color = if is_active {
                        if ui.visuals().dark_mode {
                            egui::Color32::from_rgb(147, 197, 253)
                        } else {
                            BLUE
                        }
                    } else {
                        ui.visuals().text_color()
                    };
                    let fill = if is_active {
                        if ui.visuals().dark_mode {
                            egui::Color32::from_rgb(27, 47, 78)
                        } else {
                            egui::Color32::from_rgb(232, 240, 255)
                        }
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 36.0),
                        egui::Sense::click(),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::Role::Button,
                            ui.is_enabled(),
                            title.clone(),
                        )
                    });
                    ui.painter().rect_filled(
                        rect,
                        5.0,
                        if response.hovered() && !is_active {
                            ui.visuals().widgets.hovered.weak_bg_fill
                        } else {
                            fill
                        },
                    );
                    ui.painter().with_clip_rect(rect).text(
                        rect.left_center() + egui::vec2(10.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        format!("{icon}  {title}"),
                        egui::FontId::proportional(14.0),
                        color,
                    );
                    let marker = self.tabs[i].workspace.color;
                    if marker != [0, 0, 0] {
                        ui.painter().circle_filled(
                            rect.right_center() - egui::vec2(7.0, 0.0),
                            3.0,
                            egui::Color32::from_rgb(marker[0], marker[1], marker[2]),
                        );
                    }
                    if response.has_focus() {
                        ui.painter().rect_stroke(
                            rect,
                            5.0,
                            egui::Stroke::new(1.5, BLUE),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if response.clicked() {
                        self.active = i;
                        self.pending_focus = true;
                        self.design.selected_block = None;
                        self.selected_mood_item = None;
                        self.session_dirty = true;
                    }
                    if response.double_clicked()
                        || (i == self.active && ui.input(|input| input.key_pressed(egui::Key::F2)))
                    {
                        self.design.rename = Some(id);
                        self.design.rename_draft = title.clone();
                    }
                    response.context_menu(|ui| {
                        if ui.button("Renommer").clicked() {
                            self.design.rename = Some(id);
                            self.design.rename_draft = title.clone();
                            ui.close();
                        }
                        if ui.button("Créer une sous-page").clicked() {
                            self.new_tab();
                            self.active_tab_mut().page_parent = Some(id);
                            ui.close();
                        }
                        if ui
                            .button(if self.tabs[i].workspace.favorite {
                                "Retirer des favoris"
                            } else {
                                "Ajouter aux favoris"
                            })
                            .clicked()
                        {
                            self.tabs[i].workspace.favorite = !self.tabs[i].workspace.favorite;
                            self.session_dirty = true;
                            ui.close();
                        }
                        if ui.button("Déplacer vers la corbeille").clicked() {
                            self.trash_page(id);
                            ui.close();
                        }
                    });
                }
            });
        });
    }

    pub(crate) fn document_header(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _actions: &mut Vec<Action>,
    ) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(icons::SIDEBAR)
                .on_hover_text("Afficher ou masquer la navigation")
                .clicked()
            {
                if ctx.content_rect().width() < 800.0 {
                    self.design.mobile_navigation = !self.design.mobile_navigation;
                } else {
                    self.design.sidebar = !self.design.sidebar;
                }
                self.session_dirty = true;
            }
            let parent = self
                .active_tab()
                .page_parent
                .and_then(|id| self.tabs.iter().find(|t| t.doc.id == id))
                .map(page_title)
                .unwrap_or_else(|| "Espace personnel".into());
            if ui.available_width() > 560.0 {
                ui.add(
                    egui::Label::new(egui::RichText::new(parent).color(secondary(ui))).truncate(),
                );
                ui.label("/");
                ui.add(egui::Label::new(page_title(self.active_tab())).truncate());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button(format!("{} Exporter", icons::EXPORT), |ui| {
                    let formats: &[&str] = if self.active_tab().doc.moodboard {
                        &["json", "png", "jpg"]
                    } else {
                        &["md", "html", "pdf"]
                    };
                    for ext in formats {
                        if ui.button(format!("Exporter .{ext}…")).clicked() {
                            self.export_page(ext, ctx);
                            ui.close();
                        }
                    }
                });
                if !self.active_tab().doc.moodboard {
                    let visual = self.active_tab().workspace.visual_blocks;
                    if ui.selectable_label(visual, "Blocs").clicked() {
                        self.active_tab_mut().workspace.visual_blocks = true;
                        self.active_tab_mut().doc.markdown = true;
                        self.session_dirty = true;
                    }
                    if ui.selectable_label(!visual, "Texte").clicked() {
                        self.active_tab_mut().workspace.visual_blocks = false;
                        self.session_dirty = true;
                    }
                }
            });
        });
        ui.add_space(12.0);
        ui.separator();
        if ui.available_width() < 500.0 && self.design.inspector {
            ui.small("Fermez les propriétés pour agrandir le document.");
        }
        let _ = ctx;
    }

    pub(crate) fn export_page(&mut self, ext: &str, _ctx: &egui::Context) {
        if ext == "png" || ext == "jpg" {
            self.options.export_target = Some(self.active_tab().doc.id);
            self.options.board_export = Some(ext.into());
            return;
        }
        if let Some(path) = rfd::FileDialog::new().add_filter(ext, &[ext]).save_file() {
            let t = self.active_tab();
            let result = if ext == "json" {
                workspace_io::portable(Session {
                    tabs: vec![tab_snapshot(t)],
                    ..Default::default()
                })
                .and_then(|s| serde_json::to_vec_pretty(&s.tabs[0]).map_err(std::io::Error::other))
                .and_then(|data| std::fs::write(path, data))
                .map_err(|e| e.to_string())
            } else if ext == "pdf" {
                pdf_export::export(&path, &page_title(t), &t.doc.text)
            } else {
                let content = if ext == "html" {
                    markdown::to_html(&t.doc.text)
                } else {
                    t.doc.text.clone()
                };
                std::fs::write(path, content).map_err(|e| e.to_string())
            };
            self.status = match result {
                Ok(()) => "Export terminé".into(),
                Err(e) => format!("Export impossible : {e}"),
            };
        }
    }

    pub(crate) fn contextual_inspector(&mut self, ui: &mut egui::Ui) {
        let mood = self.active_tab().doc.moodboard;
        if self.options.concentration && !mood {
            return;
        }
        if !self.design.inspector && !mood && self.design.selected_block.is_none() {
            return;
        }
        // At small widths properties use an explicit window instead of squeezing the editor.
        let narrow = !mood && ui.available_width() < 740.0;
        if narrow {
            let ctx = ui.ctx().clone();
            let mut open = true;
            egui::Window::new("Propriétés")
                .order(egui::Order::Foreground)
                .open(&mut open)
                .default_width(320.0)
                .vscroll(true)
                .show(&ctx, |ui| self.inspector_contents(ui));
            if !open {
                self.design.inspector = false;
                self.selected_mood_item = None;
            }
        } else {
            egui::Panel::right("contextual-inspector")
                .default_size(if mood { 340.0 } else { 260.0 })
                .min_size(if mood { 280.0 } else { 240.0 })
                .max_size(if mood { 380.0 } else { 420.0 })
                .resizable(true)
                .frame(
                    egui::Frame::NONE
                        .fill(ui.visuals().panel_fill)
                        .inner_margin(egui::Margin::same(14)),
                )
                .show(ui, |ui| {
                    if mood {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new("PROPRIÉTÉS")
                                    .size(11.0)
                                    .color(secondary(ui)),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Ce moodboard · modifications immédiates")
                                    .size(11.0)
                                    .color(secondary(ui)),
                            );
                        });
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            for (label, title) in [
                                ("Élément", "Élément sélectionné"),
                                ("Canevas", "Canevas"),
                                ("Grille", "Grille et alignement"),
                                ("Vue", "Vue"),
                                ("Calques", "Calques et ajout"),
                                ("Export", "Export"),
                            ] {
                                let selected = self.design.inspector_active_section == title;
                                if ui.selectable_label(selected, label).clicked() {
                                    self.design.inspector_active_section = title;
                                    self.design.inspector_section = Some(title);
                                }
                            }
                        });
                        ui.separator();
                        if let Some(target) = self.design.inspector_section.take() {
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(
                                    egui::Id::unique("moodboard-section-target"),
                                    (target, 4_u8),
                                )
                            });
                        }
                    }
                    egui::ScrollArea::vertical()
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                        )
                        .animated(false)
                        .id_salt("inspector-scroll")
                        .show(ui, |ui| self.inspector_contents(ui));
                });
        }
    }

    fn inspector_contents(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(if self.active_tab().doc.moodboard {
                        "Moodboard"
                    } else if self.design.selected_block.is_some() {
                        "Bloc sélectionné"
                    } else {
                        "Propriétés de la page"
                    })
                    .font(egui::FontId::new(
                        17.0,
                        egui::FontFamily::Name(DEFAULT_BOLD_FAMILY.into()),
                    )),
                )
                .truncate(),
            );
            if !self.active_tab().doc.moodboard
                && ui
                    .small_button(icons::X)
                    .on_hover_text("Fermer les propriétés")
                    .clicked()
            {
                self.design.inspector = false;
                self.selected_mood_item = None;
                self.design.selected_block = None;
            }
        });
        ui.add_space(20.0);
        let mut changed = false;
        if self.active_tab().workspace.read_only {
            ui.colored_label(secondary(ui), "Lecture seule · les réglages sont protégés.");
            if ui
                .button(format!("{} Autoriser les modifications", icons::LOCK_OPEN))
                .clicked()
            {
                self.active_tab_mut().workspace.read_only = false;
                self.session_dirty = true;
            }
            if self.active_tab().doc.moodboard {
                self.moodboard_read_only_overview(ui);
            }
            return;
        }
        if self.active_tab().doc.moodboard {
            ui.small("Réglages appliqués immédiatement · Faites défiler pour tout voir.");
            ui.separator();
            moodboard_panel::section_heading(ui, "Élément sélectionné");
            let Some(i) = self.selected_mood_item else {
                ui.label("Sélectionnez une note ou une image pour modifier sa position, ses dimensions, sa couleur et son lien.");
                self.moodboard_properties_ui(ui);
                return;
            };
            let mut duplicate = false;
            let mut remove = false;
            ui.horizontal_wrapped(|ui| {
                duplicate = ui.button("Dupliquer").clicked();
                remove = ui.button("Supprimer").clicked();
            });
            if let Some(texture) = self.tabs[self.active]
                .mood_items
                .get(i)
                .and_then(|item| item.image_path.as_ref())
                .and_then(|path| self.design.textures.get(path))
            {
                let mut w = ui.available_width().min(220.0);
                let aspect = texture.size()[1] as f32 / texture.size()[0] as f32;
                let mut h = w * aspect;
                if h > 120.0 {
                    h = 120.0;
                    w = h / aspect.max(0.01);
                }
                ui.image((texture.id(), egui::vec2(w, h)));
                ui.add_space(16.0);
            }
            let mut replace = false;
            let mut front = false;
            let mut back = false;
            let canvas_size = self.tabs[self.active].workspace.canvas_size;
            if let Some(item) = self.tabs[self.active].mood_items.get_mut(i) {
                changed |= ui
                    .checkbox(&mut item.locked, "Verrouiller sur la toile")
                    .changed();
                changed |= ui.checkbox(&mut item.hidden, "Masquer l’élément").changed();
                changed |= ui
                    .checkbox(&mut item.keep_ratio, "Conserver les proportions")
                    .changed();
                ui.label("Position");
                ui.horizontal(|ui| {
                    let x_label = ui.label("X");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut item.x)
                                .range(0.0..=16384.0)
                                .suffix(" px"),
                        )
                        .labelled_by(x_label.id)
                        .changed();
                    let y_label = ui.label("Y");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut item.y)
                                .range(0.0..=16384.0)
                                .suffix(" px"),
                        )
                        .labelled_by(y_label.id)
                        .changed();
                });
                ui.label("Dimensions");
                let old_size = [item.width, item.height];
                ui.horizontal(|ui| {
                    let width_label = ui.label("L");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut item.width)
                                .range(20.0..=16384.0)
                                .suffix(" px"),
                        )
                        .labelled_by(width_label.id)
                        .changed();
                    let height_label = ui.label("H");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut item.height)
                                .range(20.0..=16384.0)
                                .suffix(" px"),
                        )
                        .labelled_by(height_label.id)
                        .changed();
                });
                if item.keep_ratio && old_size[0] > 0.0 && old_size[1] > 0.0 {
                    if item.width != old_size[0] {
                        item.height = item.width * old_size[1] / old_size[0];
                    } else if item.height != old_size[1] {
                        item.width = item.height * old_size[0] / old_size[1];
                    }
                }
                changed |= moodboard_panel::element_alignment_ui(ui, item, canvas_size);
                ui.add_space(12.0);
                if item.image_path.is_none() {
                    ui.label("Couleur");
                    ui.horizontal_wrapped(|ui| {
                        for (c, name) in ["Sable", "Sauge", "Ciel", "Rose"].iter().enumerate() {
                            changed |= ui
                                .selectable_value(&mut item.color, c as u8, *name)
                                .changed();
                        }
                    });
                }
                if item.image_path.is_none() {
                    ui.label("Texte de la note");
                    changed |= ui
                        .add(
                            egui::TextEdit::multiline(&mut item.text)
                                .desired_width(f32::INFINITY)
                                .desired_rows(3),
                        )
                        .changed();
                }
                ui.add_space(12.0);
                ui.strong("Lien");
                ui.scope(|ui| {
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(item.link.get_or_insert_with(String::new))
                                .hint_text("https://…")
                                .desired_width(f32::INFINITY),
                        )
                        .changed();
                });
                if item.image_path.is_some() {
                    replace = ui.button("Remplacer l’image…").clicked();
                }
                ui.add_space(20.0);
                ui.label("Disposition");
                ui.horizontal(|ui| {
                    back = ui.button("Arrière-plan").clicked();
                    front = ui.button("Premier plan").clicked();
                });
                ui.add_space(16.0);
            }
            if replace {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
                    .pick_file()
                {
                    if let Some(item) = self.tabs[self.active].mood_items.get_mut(i) {
                        item.image_path = Some(path.to_string_lossy().into_owned());
                        item.embedded_image = None;
                        self.design.textures.clear();
                        changed = true;
                    }
                }
            }
            if duplicate {
                if let Some(mut item) = self.tabs[self.active].mood_items.get(i).cloned() {
                    item.x += 24.0;
                    item.y += 24.0;
                    self.tabs[self.active].mood_items.push(item);
                    self.selected_mood_item = Some(self.tabs[self.active].mood_items.len() - 1);
                    changed = true;
                }
            }
            if remove && i < self.tabs[self.active].mood_items.len() {
                self.tabs[self.active].mood_items.remove(i);
                self.selected_mood_item = None;
                changed = true;
            } else if (front || back) && i < self.tabs[self.active].mood_items.len() {
                let item = self.tabs[self.active].mood_items.remove(i);
                let j = if front {
                    self.tabs[self.active].mood_items.len()
                } else {
                    0
                };
                self.tabs[self.active].mood_items.insert(j, item);
                self.selected_mood_item = Some(j);
                changed = true;
            }
            self.moodboard_properties_ui(ui);
        } else if let Some(index) = self.design.selected_block {
            let mut chunks = blocks::split(&self.active_tab().doc.text);
            if let Some(chunk) = chunks.get_mut(index) {
                ui.label("Type de bloc");
                let mut convert = None;
                ui.horizontal(|ui| {
                    for (name, prefix) in
                        [("Texte", ""), ("H1", "# "), ("H2", "## "), ("H3", "### ")]
                    {
                        let level = chunk.chars().take_while(|c| *c == '#').count();
                        let current = if level == 0 {
                            ""
                        } else if level == 1 {
                            "# "
                        } else if level == 2 {
                            "## "
                        } else {
                            "### "
                        };
                        if ui.selectable_label(current == prefix, name).clicked() {
                            convert = Some(prefix);
                        }
                    }
                });
                if let Some(prefix) = convert {
                    let source = chunk.trim_start_matches('#').trim_start();
                    *chunk = format!("{prefix}{source}");
                    changed = true;
                }
                ui.add_space(16.0);
                ui.label("Disposition");
                let mut movement = None;
                if ui.button("Monter").clicked() && index > 0 {
                    movement = Some(index - 1);
                }
                if ui.button("Descendre").clicked() && index + 1 < chunks.len() {
                    movement = Some(index + 1);
                }
                if let Some(to) = movement {
                    chunks.swap(index, to);
                    self.design.selected_block = Some(to);
                    changed = true;
                }
                if ui.button("Dupliquer le bloc").clicked() {
                    chunks.insert(index + 1, chunks[index].clone());
                    self.design.selected_block = Some(index + 1);
                    changed = true;
                }
                if changed {
                    self.active_tab_mut().doc.text = chunks.concat();
                    self.active_tab_mut().touch();
                }
            } else {
                self.design.selected_block = None;
            }
        } else {
            let tab = &mut self.tabs[self.active];
            changed |= ui.checkbox(&mut tab.workspace.favorite, "Favori").changed();
            changed |= ui
                .checkbox(&mut tab.workspace.read_only, "Lecture seule")
                .changed();
            ui.label("Icône");
            ui.horizontal_wrapped(|ui| {
                for (value, label) in [
                    ("", "Automatique"),
                    (icons::STAR, "Étoile"),
                    (icons::FILE_TEXT, "Note"),
                    (icons::IMAGE, "Image"),
                ] {
                    changed |= ui
                        .selectable_value(&mut tab.workspace.icon, value.into(), label)
                        .changed();
                }
            });
            ui.label("Couleur de repère");
            changed |= ui
                .color_edit_button_srgb(&mut tab.workspace.color)
                .changed();
            let mut own_width = tab.workspace.reading_width.is_some();
            if ui
                .checkbox(&mut own_width, "Largeur de lecture propre à cette page")
                .changed()
            {
                tab.workspace.reading_width = own_width.then_some(self.preferences.reading_width);
                changed = true;
            }
            if let Some(width) = &mut tab.workspace.reading_width {
                changed |= ui
                    .add(
                        egui::DragValue::new(width)
                            .range(300.0..=2400.0)
                            .suffix(" pt"),
                    )
                    .changed();
            }
            ui.add_space(16.0);
            ui.label("Nom de la page");
            ui.label(page_title(tab));
            if ui.button("Renommer…").clicked() {
                self.design.rename = Some(tab.doc.id);
                self.design.rename_draft = page_title(tab);
                self.design.sidebar = true;
                self.design.mobile_navigation = true;
            }
            ui.add_space(16.0);
            ui.label("Liens de cette page");
            let targets = wiki_targets(&tab.doc.text);
            for target in targets {
                if let Some(i) = resolve_page(&self.tabs, &target) {
                    if ui.link(page_title(&self.tabs[i])).clicked() {
                        self.active = i;
                        self.pending_focus = true;
                    }
                }
            }
            ui.separator();
            ui.label("Liens entrants");
            let current = self.active;
            let backlinks: Vec<_> = self
                .tabs
                .iter()
                .enumerate()
                .filter(|(i, t)| {
                    *i != current
                        && !t.workspace.trashed
                        && wiki_targets(&t.doc.text)
                            .iter()
                            .any(|target| resolve_page(&self.tabs, target) == Some(current))
                })
                .map(|(i, t)| (i, page_title(t)))
                .collect();
            if backlinks.is_empty() {
                ui.small("Aucune autre page ne pointe vers cette page.");
            }
            for (i, title) in backlinks {
                if ui.link(title).clicked() {
                    self.active = i;
                    self.pending_focus = true;
                }
            }
        }
        if changed {
            if self.active_tab().doc.moodboard || self.design.selected_block.is_some() {
                self.active_tab_mut().doc.dirty = true;
            }
            self.session_dirty = true;
        }
    }

    pub(crate) fn trash_window(&mut self, ctx: &egui::Context) {
        if !self.design.trash {
            return;
        }
        let mut open = true;
        egui::Window::new("Corbeille")
            .open(&mut open)
            .default_width(430.0)
            .show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.options.trash_query)
                        .hint_text("Rechercher dans la corbeille"),
                );
                if !self.options.undo_trash.is_empty()
                    && ui
                        .button("Annuler la dernière mise à la corbeille")
                        .clicked()
                {
                    self.undo_trash();
                }
                let mut found = false;
                let mut restore = None;
                for t in &self.tabs {
                    if t.workspace.trashed
                        && page_title(t)
                            .to_lowercase()
                            .contains(&self.options.trash_query.to_lowercase())
                    {
                        found = true;
                        ui.horizontal_wrapped(|ui| {
                            ui.label(page_title(t));
                            ui.small(t.workspace.trashed_at.as_deref().unwrap_or("Date inconnue"));
                            if ui.button("Restaurer avec les sous-pages").clicked() {
                                restore = Some(t.doc.id);
                            }
                            if ui.button("Supprimer définitivement…").clicked() {
                                self.options.purge_page = Some(t.doc.id);
                            }
                        });
                    }
                }
                if let Some(id) = restore {
                    self.restore_trash(id);
                }
                if !found {
                    ui.label("Aucune page dans la corbeille pour cette recherche.");
                }
            });
        self.design.trash = open;
        if let Some(id) = self.options.purge_page {
            let ids: Vec<_> = self
                .tabs
                .iter()
                .filter(|t| t.workspace.trashed && self.is_descendant(t.doc.id, id))
                .map(|t| t.doc.id)
                .collect();
            egui::Modal::new(egui::Id::unique("purge-pages-confirmation")).show(ctx, |ui| {
                ui.heading("Suppression définitive");
                ui.label(format!("Supprimer {} page(s) et leur historique de cet espace ? Les fichiers externes, sauvegardes et copies synchronisées ne seront pas effacés. Cette action ne peut pas être annulée.",ids.len()));
                if ui.button("Supprimer définitivement ces pages").clicked() {
                    self.tabs.retain(|t|!ids.contains(&t.doc.id));
                    for t in &mut self.tabs { if t.page_parent.is_some_and(|p|ids.contains(&p)) { t.page_parent = None; } }
                    if self.tabs.is_empty() { self.new_tab(); } self.active = self.active.min(self.tabs.len()-1);
                    self.options.purge_page = None; self.session_dirty = true;
                }
                if ui.button("Annuler").clicked() { self.options.purge_page = None; }
            });
        }
    }
}
