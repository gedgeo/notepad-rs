//! Explicit close decisions and modal dialogs: cancellation never authorizes exit.
use super::*;

#[derive(Clone, Copy)]
enum Decision {
    Save,
    Discard,
    Cancel,
}

fn confirm(ctx: &egui::Context, id: &str, title: &str, message: &str) -> Option<Decision> {
    let width = (ctx.content_rect().width() - 48.0).clamp(240.0, 460.0);
    let response = egui::Modal::new(egui::Id::unique(id)).show(ctx, |ui| {
        ui.set_width(width);
        ui.set_max_width(width);
        ui.label(egui::RichText::new(title).font(egui::FontId::new(
            20.0,
            egui::FontFamily::Name(DEFAULT_BOLD_FAMILY.into()),
        )));
        ui.add_space(10.0);
        ui.label(message);
        ui.add_space(10.0);
        let mut decision = None;
        ui.horizontal_wrapped(|ui| {
            if ui.button("Enregistrer").clicked() {
                decision = Some(Decision::Save);
            }
            if ui.button("Ne pas enregistrer").clicked() {
                decision = Some(Decision::Discard);
            }
            if ui.button("Annuler").clicked() {
                decision = Some(Decision::Cancel);
            }
        });
        decision
    });
    response
        .inner
        .or_else(|| response.should_close().then_some(Decision::Cancel))
}

impl NotepadApp {
    fn finish_window_close(&mut self, ctx: &egui::Context, discard: bool) {
        let mut snapshot = self.session_snapshot(ctx);
        if discard {
            snapshot.tabs.retain_mut(|page| {
                if !page.dirty {
                    return true;
                }
                let Some(path) = page.path.as_ref().map(PathBuf::from) else {
                    return false;
                };
                if page.moodboard {
                    let Some(mut saved) = std::fs::read(&path)
                        .ok()
                        .and_then(|b| serde_json::from_slice::<session::SessionTab>(&b).ok())
                        .filter(|p| p.moodboard)
                    else {
                        return false;
                    };
                    saved.page_id = page.page_id;
                    saved.parent_id = page.parent_id;
                    saved.path = page.path.clone();
                    saved.dirty = false;
                    *page = saved;
                } else {
                    page.text = None;
                    page.dirty = false;
                }
                true
            });
            snapshot.active = snapshot.active.min(snapshot.tabs.len().saturating_sub(1));
        }
        match snapshot.save_to(&self.session_file) {
            Ok(()) => {
                self.pending_quit = false;
                self.quit_save_pending = false;
                self.close_confirmed = true;
                self.close_requested = true;
                self.session_dirty = false;
            }
            Err(error) => {
                self.status =
                    format!("Fermeture interrompue : sauvegarde de session impossible ({error})");
                self.pending_quit = true;
                self.close_confirmed = false;
                self.close_requested = false;
                self.quit_save_pending = false;
            }
        }
    }

    fn save_before_window_close(&mut self, ctx: &egui::Context) {
        self.quit_save_pending = false;
        let original = self.active;
        for i in 0..self.tabs.len() {
            if !self.tabs[i].doc.dirty {
                continue;
            }
            self.active = i;
            self.save_active(false);
            if self.pending_ansi_save.is_some() {
                self.quit_save_pending = true;
                return;
            }
            // A cancelled file dialog or failed write keeps the confirmation open.
            if self.tabs[i].doc.dirty {
                self.active = original;
                return;
            }
        }
        self.active = original;
        self.finish_window_close(ctx, false);
    }

    pub(crate) fn close_dialogs_ui(&mut self, ctx: &egui::Context) {
        if self.quit_save_pending && self.pending_ansi_save.is_none() {
            self.save_before_window_close(ctx);
        }
        if self.pending_quit && self.pending_ansi_save.is_none() {
            if let Some(decision) = confirm(
                ctx,
                "window-close-confirmation",
                "Fermer la fenêtre ?",
                "Certains documents ont été modifiés. Enregistrez-les avant de fermer.",
            ) {
                match decision {
                    Decision::Save => self.save_before_window_close(ctx),
                    Decision::Discard => self.finish_window_close(ctx, true),
                    Decision::Cancel => {
                        self.pending_quit = false;
                        self.close_requested = false;
                        self.close_confirmed = false;
                        self.quit_save_pending = false;
                        self.quit_all_windows = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    }
                }
            }
        }
        if let Some(path) = self.pending_ansi_save.clone() {
            let response=egui::Modal::new(egui::Id::unique("ansi-save-confirmation")).show(ctx,|ui|{
                ui.set_width((ctx.content_rect().width()-48.0).clamp(240.0,460.0));
                ui.label(egui::RichText::new("Conversion ANSI").size(20.0));
                ui.label("Certains caractères ne peuvent pas être représentés en ANSI et seraient remplacés.");
                let mut choice=None;
                ui.horizontal_wrapped(|ui|{if ui.button("Enregistrer quand même").clicked(){choice=Some(true);}if ui.button("Annuler").clicked(){choice=Some(false);}});choice
            });
            let choice = response
                .inner
                .or_else(|| response.should_close().then_some(false));
            if let Some(save) = choice {
                self.pending_ansi_save = None;
                if save {
                    self.write_active_to(path);
                    if self.active_tab().doc.dirty {
                        self.quit_save_pending = false;
                    }
                } else {
                    self.quit_save_pending = false;
                }
            }
        }
        if self.pending_quit || self.pending_ansi_save.is_some() || self.close_confirmed {
            return;
        }
        let Some(id) = self.pending_close.first().copied() else {
            return;
        };
        let Some(index) = self.tabs.iter().position(|t| t.doc.id == id) else {
            self.pending_close.remove(0);
            return;
        };
        if !self.tabs[index].doc.dirty {
            self.pending_close.remove(0);
            self.close_tab(index);
            return;
        }
        let title = ui_design::page_title(&self.tabs[index]);
        if let Some(decision) = confirm(
            ctx,
            "tab-close-confirmation",
            "Fermer la page ?",
            &format!("« {title} » contient des modifications non enregistrées."),
        ) {
            match decision {
                Decision::Cancel => self.pending_close.clear(),
                Decision::Discard => {
                    self.pending_close.remove(0);
                    self.close_tab(index);
                }
                Decision::Save => {
                    self.active = index;
                    self.save_active(false);
                    if !self.tabs[index].doc.dirty {
                        self.pending_close.remove(0);
                        self.close_tab(index);
                    }
                }
            }
        }
    }
}

pub(crate) fn tab_snapshot(tab: &Tab) -> session::SessionTab {
    session::SessionTab {
        path: tab
            .doc
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
        text: Some(tab.doc.text.clone()),
        encoding: session::encoding_code(tab.doc.encoding),
        eol: session::eol_code(tab.doc.eol),
        wrap: tab.doc.wrap,
        font_size: tab.doc.font_size,
        cursor: tab.doc.cursor_char,
        dirty: tab.doc.dirty,
        markdown: tab.doc.markdown,
        moodboard: tab.doc.moodboard,
        mood_items: tab.mood_items.clone(),
        page_id: tab.doc.id,
        parent_id: tab.page_parent,
        page_title: tab.page_title.clone(),
        workspace: tab.workspace.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (NotepadApp, egui::Context, PathBuf) {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let dir = std::env::temp_dir().join(format!("notepad-rs-close-{}", workspace_io::uid()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = NotepadApp::new(&cc, None, dir.join("session.json"));
        let path = dir.join("note.txt");
        std::fs::write(&path, "original").unwrap();
        let tab = app.active_tab_mut();
        tab.doc.path = Some(path);
        tab.doc.text = "modifié".into();
        tab.doc.encoding = Encoding::Utf8;
        tab.doc.dirty = true;
        (app, ctx, dir)
    }
    fn frame(
        app: &mut NotepadApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 700.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            if app.active_tab().doc.moodboard {
                app.moodboard_ui(ui, ctx);
            } else {
                ui.label("Document de test");
            }
            app.close_dialogs_ui(ctx);
        });
        output.textures_delta.clear();
        output
    }
    fn click(app: &mut NotepadApp, ctx: &egui::Context, label: &str) {
        frame(app, ctx, vec![]);
        let output = frame(app, ctx, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + egui::vec2(6.0, 6.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Bouton absent : {label}"));
        frame(
            app,
            ctx,
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
        frame(
            app,
            ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
    }
    fn cleanup(dir: PathBuf) {
        for file in std::fs::read_dir(&dir).unwrap() {
            std::fs::remove_file(file.unwrap().path()).unwrap();
        }
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn cancel_window_keeps_dirty_content_and_cancels_quit_all() {
        let (mut app, ctx, dir) = fixture();
        app.pending_quit = true;
        app.quit_all_windows = true;
        click(&mut app, &ctx, "Annuler");
        assert!(
            !app.pending_quit
                && !app.close_requested
                && !app.close_confirmed
                && !app.quit_all_windows
        );
        assert!(app.active_tab().doc.dirty);
        assert_eq!(app.active_tab().doc.text, "modifié");
        assert!(!dir.join("session.json").exists());
        cleanup(dir);
    }
    #[test]
    fn save_window_writes_file_then_authorizes_exit() {
        let (mut app, ctx, dir) = fixture();
        app.pending_quit = true;
        click(&mut app, &ctx, "Enregistrer");
        assert!(app.close_confirmed && app.close_requested);
        assert!(!app.has_unsaved());
        assert_eq!(
            std::fs::read_to_string(dir.join("note.txt")).unwrap(),
            "modifié"
        );
        assert!(dir.join("session.json").exists());
        cleanup(dir);
    }
    #[test]
    fn discard_window_does_not_write_dirty_text_back_to_session() {
        let (mut app, ctx, dir) = fixture();
        app.pending_quit = true;
        click(&mut app, &ctx, "Ne pas enregistrer");
        assert!(app.close_confirmed && app.close_requested);
        assert_eq!(
            std::fs::read_to_string(dir.join("note.txt")).unwrap(),
            "original"
        );
        let saved = Session::load_from(&dir.join("session.json"));
        assert_eq!(saved.tabs[0].text, None);
        assert!(!saved.tabs[0].dirty);
        cleanup(dir);
    }
    #[test]
    fn discard_tab_actually_removes_dirty_tab() {
        let (mut app, ctx, dir) = fixture();
        let id = app.active_tab().doc.id;
        app.pending_close.push(id);
        click(&mut app, &ctx, "Ne pas enregistrer");
        assert!(!app.tabs.iter().any(|t| t.doc.id == id));
        assert!(app.pending_close.is_empty());
        assert_eq!(
            std::fs::read_to_string(dir.join("note.txt")).unwrap(),
            "original"
        );
        cleanup(dir);
    }
    #[test]
    fn failed_save_keeps_tab_and_confirmation() {
        let (mut app, ctx, dir) = fixture();
        app.active_tab_mut().doc.path = Some(dir.clone());
        let id = app.active_tab().doc.id;
        app.pending_close.push(id);
        click(&mut app, &ctx, "Enregistrer");
        assert!(app.active_tab().doc.dirty);
        assert_eq!(app.pending_close, vec![id]);
        assert!(!app.close_confirmed);
        cleanup(dir);
    }
    #[test]
    fn modal_buttons_work_above_moodboard_and_cancel_keeps_items() {
        let (mut app, ctx, dir) = fixture();
        app.new_moodboard();
        app.pending_quit = true;
        let before = app.active_tab().mood_items[0].text.clone();
        click(&mut app, &ctx, "Annuler");
        assert!(!app.pending_quit && !app.close_requested);
        assert_eq!(app.active_tab().mood_items[0].text, before);
        cleanup(dir);
    }
    #[test]
    fn ansi_cancel_never_authorizes_window_close() {
        let (mut app, ctx, dir) = fixture();
        app.active_tab_mut().doc.text = "Γ".into();
        app.active_tab_mut().doc.encoding = Encoding::Ansi;
        app.pending_quit = true;
        app.save_before_window_close(&ctx);
        assert!(app.pending_ansi_save.is_some());
        click(&mut app, &ctx, "Annuler");
        assert!(app.pending_quit && !app.close_confirmed && !app.quit_save_pending);
        assert!(app.active_tab().doc.dirty);
        assert_eq!(
            std::fs::read_to_string(dir.join("note.txt")).unwrap(),
            "original"
        );
        cleanup(dir);
    }
    #[test]
    fn saved_moodboard_roundtrips_as_board_instead_of_empty_text() {
        let (mut app, ctx, dir) = fixture();
        app.new_moodboard();
        app.active_tab_mut().mood_items[0].text = "Note importante".into();
        let path = dir.join("board.json");
        app.active_tab_mut().doc.path = Some(path.clone());
        app.pending_quit = true;
        click(&mut app, &ctx, "Enregistrer");
        assert!(app.close_confirmed);
        let mut restored = NotepadApp::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            None,
            dir.join("unused.json"),
        );
        restored.open_path(path);
        assert!(restored.active_tab().doc.moodboard);
        assert_eq!(restored.active_tab().mood_items[0].text, "Note importante");
        cleanup(dir);
    }
}
