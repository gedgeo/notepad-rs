//! Task-oriented moodboard inspector. All edits stay scoped to the active board.
use super::*;

pub(crate) fn section_heading(ui: &mut egui::Ui, title: &str) {
    let response = ui.strong(title);
    let id = egui::Id::unique("moodboard-section-target");
    if let Some((target, remaining)) = ui.ctx().data(|d| d.get_temp::<(&'static str, u8)>(id)) {
        if target == title {
            response.scroll_to_me(Some(egui::Align::Min));
            // Reapply briefly while lazy image previews establish their final height.
            ui.ctx().data_mut(|d| {
                if remaining > 1 {
                    d.insert_temp(id, (target, remaining - 1));
                } else {
                    d.remove::<(&'static str, u8)>(id);
                }
            });
        }
    }
}

fn section(ui: &mut egui::Ui, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(12.0);
    ui.separator();
    section_heading(ui, title);
    ui.add_space(6.0);
    ui.push_id(title, contents);
}

impl NotepadApp {
    pub(crate) fn moodboard_read_only_overview(&mut self, ui: &mut egui::Ui) {
        let workspace = &self.active_tab().workspace;
        section(ui, "Canevas", |ui| {
            match workspace.canvas_size {
                Some([w, h]) => {
                    ui.label(format!("Format · {w} × {h} px"));
                }
                None => {
                    ui.label("Format · canevas libre");
                }
            }
            ui.label(if workspace.canvas_appearance.hide_border {
                "Cadre · masqué"
            } else {
                "Cadre · visible"
            });
        });
        let appearance = &workspace.canvas_appearance;
        let grid = appearance.grid.unwrap_or(self.preferences.grid);
        let grid_size = appearance.grid_size.unwrap_or(self.preferences.grid_size);
        section(ui, "Grille et alignement", |ui| {
            ui.label(if grid {
                "Grille · visible"
            } else {
                "Grille · masquée"
            });
            ui.label(format!("Espacement · {grid_size:.0} px"));
        });
        section(ui, "Vue", |ui| {
            let mut percent = self.design.board_zoom * 100.0;
            ui.horizontal(|ui| {
                let label = ui.label("Zoom");
                if ui
                    .add(
                        egui::DragValue::new(&mut percent)
                            .range(2.0..=250.0)
                            .suffix(" %"),
                    )
                    .labelled_by(label.id)
                    .changed()
                {
                    self.design.board_zoom = percent / 100.0;
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("Ajuster le canevas").clicked() {
                    self.design.board_fit_requested = true;
                }
                if ui.button("Vue à 100 %").clicked() {
                    self.design.board_zoom = 1.0;
                    self.design.board_pan = egui::vec2(8.0, 8.0);
                }
            });
        });
        section(ui, "Calques et ajout", |ui| {
            ui.label(format!("{} élément(s)", self.active_tab().mood_items.len()));
            ui.small("Déverrouillez la page pour modifier les calques ou ajouter du contenu.");
        });
        section(ui, "Export", |ui| {
            ui.horizontal_wrapped(|ui| {
                for (label, ext) in [
                    ("Image PNG", "png"),
                    ("Image JPEG", "jpg"),
                    ("Moodboard JSON", "json"),
                ] {
                    if ui.button(label).clicked() {
                        self.export_page(ext, &ui.ctx().clone());
                    }
                }
            });
        });
    }

    pub(crate) fn moodboard_properties_ui(&mut self, ui: &mut egui::Ui) {
        let before = serde_json::to_vec(&self.active_tab().workspace).ok();
        section(ui, "Canevas", |ui| {
            let size = &mut self.tabs[self.active].workspace.canvas_size;
            egui::ComboBox::from_id_salt("board-format")
                .selected_text(match *size {
                    None => "Libre",
                    Some([1080, 1080]) => "Carré",
                    Some([1920, 1080]) => "Paysage",
                    Some([1080, 1920]) => "Portrait",
                    Some([794, 1123]) => "A4 portrait",
                    _ => "Personnalisé",
                })
                .show_ui(ui, |ui| {
                    for (label, value) in [
                        ("Libre", None),
                        ("Carré · 1080 × 1080", Some([1080, 1080])),
                        ("Paysage · 1920 × 1080", Some([1920, 1080])),
                        ("Portrait · 1080 × 1920", Some([1080, 1920])),
                        ("A4 portrait · 794 × 1123", Some([794, 1123])),
                    ] {
                        if ui.selectable_value(size, value, label).changed() {
                            self.design.board_fit_requested = true;
                        }
                    }
                });
            if size.is_none() && ui.button("Définir une taille personnalisée").clicked() {
                *size = Some([1920, 1080]);
                self.design.board_fit_requested = true;
            }
            if let Some([w, h]) = size {
                for (label, value) in [("Largeur", &mut *w), ("Hauteur", &mut *h)] {
                    ui.horizontal(|ui| {
                        let l = ui.label(label);
                        if ui
                            .add(egui::DragValue::new(value).range(100..=16384).suffix(" px"))
                            .labelled_by(l.id)
                            .changed()
                        {
                            self.design.board_fit_requested = true;
                        }
                    });
                }
                if ui.button("Permuter largeur / hauteur").clicked() {
                    std::mem::swap(w, h);
                    self.design.board_fit_requested = true;
                }
            }
            let appearance = &mut self.tabs[self.active].workspace.canvas_appearance;
            ui.horizontal(|ui| {
                ui.label("Fond");
                let mut color = appearance.background.unwrap_or(if ui.visuals().dark_mode {
                    [27, 27, 27]
                } else {
                    [255, 255, 255]
                });
                if ui.color_edit_button_srgb(&mut color).changed() {
                    appearance.background = Some(color);
                }
                if ui.small_button("Automatique").clicked() {
                    appearance.background = None;
                }
            });
            let mut border = !appearance.hide_border;
            if ui.checkbox(&mut border, "Afficher le cadre").changed() {
                appearance.hide_border = !border;
            }
            ui.small("Changer le cadre ne supprime aucun élément.");
        });
        section(ui, "Grille et alignement", |ui| {
            let appearance = &mut self.tabs[self.active].workspace.canvas_appearance;
            let mut grid = appearance.grid.unwrap_or(self.preferences.grid);
            if ui.checkbox(&mut grid, "Afficher la grille").changed() {
                appearance.grid = Some(grid);
            }
            ui.horizontal(|ui| {
                ui.selectable_value(&mut appearance.lines, false, "Points");
                ui.selectable_value(&mut appearance.lines, true, "Lignes");
            });
            let mut step = appearance.grid_size.unwrap_or(self.preferences.grid_size);
            ui.horizontal(|ui| {
                let l = ui.label("Espacement");
                if ui
                    .add(
                        egui::DragValue::new(&mut step)
                            .range(8.0..=200.0)
                            .suffix(" px"),
                    )
                    .labelled_by(l.id)
                    .changed()
                {
                    appearance.grid_size = Some(step);
                }
            });
            let mut snap = appearance.snap.unwrap_or(self.preferences.snap);
            if ui
                .checkbox(&mut snap, "Aligner les déplacements sur la grille")
                .changed()
            {
                appearance.snap = Some(snap);
            }
            if ui.button("Réinitialiser l’affichage du canevas").clicked() {
                *appearance = Default::default();
            }
        });
        section(ui, "Vue", |ui| {
            let mut percent = self.design.board_zoom * 100.0;
            ui.horizontal(|ui| {
                let l = ui.label("Zoom");
                if ui
                    .add(
                        egui::DragValue::new(&mut percent)
                            .range(2.0..=250.0)
                            .suffix(" %"),
                    )
                    .labelled_by(l.id)
                    .changed()
                {
                    self.design.board_zoom = percent / 100.0;
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("Ajuster le canevas").clicked() {
                    self.design.board_fit_requested = true;
                }
                if ui.button("Vue à 100 %").clicked() {
                    self.design.board_zoom = 1.0;
                    self.design.board_pan = egui::vec2(8.0, 8.0);
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.design.board_hand, false, "Sélection");
                ui.selectable_value(&mut self.design.board_hand, true, "Déplacer la vue");
            });
        });
        section(ui, "Calques et ajout", |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Ajouter une note").clicked() {
                    self.execute_command(&ui.ctx().clone(), "board-note");
                }
                if ui.button("Importer une image…").clicked() {
                    self.design.board_import_requested = true;
                }
            });
            let mut changed = false;
            for (i, item) in self.tabs[self.active]
                .mood_items
                .iter_mut()
                .enumerate()
                .rev()
            {
                ui.push_id(i, |ui| {
                    let label = if let Some(path) = &item.image_path {
                        PathBuf::from(path)
                            .file_name()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Image".into())
                    } else {
                        item.text
                            .lines()
                            .next()
                            .unwrap_or("Note")
                            .chars()
                            .take(24)
                            .collect()
                    };
                    if ui
                        .selectable_label(
                            self.selected_mood_item == Some(i),
                            format!("{} · {}", i + 1, label),
                        )
                        .clicked()
                    {
                        self.selected_mood_item = Some(i);
                    }
                    ui.horizontal_wrapped(|ui| {
                        changed |= ui.checkbox(&mut item.hidden, "Masqué").changed();
                        changed |= ui.checkbox(&mut item.locked, "Verrouillé").changed();
                    });
                });
            }
            if changed {
                self.active_tab_mut().doc.dirty = true;
                self.session_dirty = true;
            }
        });
        section(ui, "Export", |ui| {
            ui.horizontal_wrapped(|ui| {
                for (label, ext) in [
                    ("Image PNG", "png"),
                    ("Image JPEG", "jpg"),
                    ("Moodboard JSON", "json"),
                ] {
                    if ui.button(label).clicked() {
                        self.export_page(ext, &ui.ctx().clone());
                    }
                }
            });
        });
        if before != serde_json::to_vec(&self.active_tab().workspace).ok() {
            self.active_tab_mut().doc.dirty = true;
            self.session_dirty = true;
        }
    }
}

pub(crate) fn element_alignment_ui(
    ui: &mut egui::Ui,
    item: &mut session::MoodItem,
    canvas: Option<[u32; 2]>,
) -> bool {
    let before = [item.x, item.y, item.width, item.height];
    section(ui, "Aligner dans le canevas", |ui| {
        ui.add_enabled_ui(canvas.is_some(), |ui| {
            let [w, h] = canvas.unwrap_or([1920, 1080]);
            ui.horizontal_wrapped(|ui| {
                if ui.button("Gauche").clicked() {
                    item.x = 0.0;
                }
                if ui.button("Centre horizontal").clicked() {
                    item.x = ((w as f32 - item.width) / 2.0).max(0.0);
                }
                if ui.button("Droite").clicked() {
                    item.x = (w as f32 - item.width).max(0.0);
                }
                if ui.button("Haut").clicked() {
                    item.y = 0.0;
                }
                if ui.button("Centre vertical").clicked() {
                    item.y = ((h as f32 - item.height) / 2.0).max(0.0);
                }
                if ui.button("Bas").clicked() {
                    item.y = (h as f32 - item.height).max(0.0);
                }
            });
            if ui.button("Adapter l’élément au canevas").clicked() {
                let scale = (w as f32 / item.width.max(1.0)).min(h as f32 / item.height.max(1.0));
                item.width *= scale;
                item.height *= scale;
                item.x = (w as f32 - item.width) / 2.0;
                item.y = (h as f32 - item.height) / 2.0;
            }
        });
        if canvas.is_none() {
            ui.small("Choisissez un format de canevas pour activer l’alignement.");
        }
    });
    before != [item.x, item.y, item.width, item.height]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (NotepadApp, egui::Context) {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = NotepadApp::new(&cc, None, PathBuf::from("unused-board-panel.json"));
        app.new_moodboard();
        app.active_tab_mut().workspace.canvas_size = Some([1920, 1080]);
        app.active_tab_mut().doc.dirty = false;
        (app, ctx)
    }
    fn frame(
        app: &mut NotepadApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(420.0, 1600.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.moodboard_properties_ui(ui));
        output.textures_delta.clear();
        output
    }
    fn click(app: &mut NotepadApp, ctx: &egui::Context, label: &str) {
        frame(app, ctx, vec![]);
        let out = frame(app, ctx, vec![]);
        let pos = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + egui::vec2(5.0, 5.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Contrôle absent : {label}"));
        for pressed in [true, false] {
            frame(
                app,
                ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
    }
    #[test]
    fn canvas_controls_are_available_without_selected_item() {
        let (mut app, ctx) = fixture();
        app.selected_mood_item = None;
        let out = frame(&mut app, &ctx, vec![]);
        for label in [
            "Largeur",
            "Hauteur",
            "Fond",
            "Afficher la grille",
            "Espacement",
            "Zoom",
            "Ajouter une note",
            "Image PNG",
        ] {
            assert!(
                out.shapes
                    .iter()
                    .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==label)),
                "{label}"
            );
        }
    }
    #[test]
    fn swapping_canvas_keeps_elements_and_marks_board_dirty() {
        let (mut app, ctx) = fixture();
        let items = serde_json::to_vec(&app.active_tab().mood_items).unwrap();
        click(&mut app, &ctx, "Permuter largeur / hauteur");
        assert_eq!(app.active_tab().workspace.canvas_size, Some([1080, 1920]));
        assert_eq!(
            serde_json::to_vec(&app.active_tab().mood_items).unwrap(),
            items
        );
        assert!(app.active_tab().doc.dirty && app.design.board_fit_requested);
    }
    #[test]
    fn board_appearance_roundtrips_without_affecting_another_board() {
        let (mut app, ctx) = fixture();
        app.active_tab_mut().workspace.canvas_appearance = session::CanvasAppearance {
            background: Some([30, 60, 90]),
            grid: Some(false),
            snap: Some(true),
            grid_size: Some(48.0),
            lines: true,
            hide_border: false,
        };
        let saved = app.session_snapshot(&ctx);
        let restored: Session =
            serde_json::from_slice(&serde_json::to_vec(&saved).unwrap()).unwrap();
        let value = &restored.tabs[app.active].workspace.canvas_appearance;
        assert_eq!(value.background, Some([30, 60, 90]));
        assert_eq!(value.grid_size, Some(48.0));
        app.new_moodboard();
        assert_eq!(
            app.active_tab().workspace.canvas_appearance,
            Default::default()
        );
    }
    #[test]
    fn old_pages_keep_inherited_display_preferences() {
        let page: session::PageOptions = serde_json::from_str("{}").unwrap();
        assert_eq!(page.canvas_appearance, Default::default());
    }
    #[test]
    fn image_export_uses_the_board_background() {
        let font = std::fs::read(windows_fonts_dir().join("segoeui.ttf")).unwrap();
        let image =
            board_export::render(&[], Some([100, 100]), 1.0, &font, Some([30, 60, 90])).unwrap();
        assert_eq!(image.get_pixel(99, 99).0, [30, 60, 90, 255]);
    }
}
