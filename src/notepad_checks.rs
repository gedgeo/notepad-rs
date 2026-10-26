use super::*;

#[test]
fn legacy_sessions_default_to_current_window_and_recent_files() {
    let session: Session = serde_json::from_str("{}").unwrap();
    assert!(session.preferences.recent_enabled);
    assert!(!session.preferences.open_new_window);
    assert!(!session.maximized);
    assert!(session.window_position.is_none());
}

#[test]
fn disabled_recents_stay_empty_and_preferences_roundtrip() {
    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx.clone());
    let mut app = NotepadApp::new(&cc, None, PathBuf::from("unused-preferences.json"));
    app.preferences.recent_enabled = false;
    app.preferences.open_new_window = true;
    app.remember_recent(std::path::Path::new("private.txt"));
    assert!(app.recent.is_empty());
    let snapshot = app.session_snapshot(&ctx);
    let restored: Session =
        serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
    let app = NotepadApp::new(
        &cc,
        Some(restored),
        PathBuf::from("unused-preferences.json"),
    );
    assert!(!app.preferences.recent_enabled);
    assert!(app.preferences.open_new_window);
    assert!(app.recent.is_empty());
}

#[test]
fn maximized_snapshot_preserves_normal_window_geometry() {
    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx.clone());
    let mut app = NotepadApp::new(&cc, None, PathBuf::from("unused-geometry.json"));
    app.normal_window = Some([80.0, 90.0, 1100.0, 700.0]);
    let mut input = egui::RawInput::default();
    let viewport = input.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
    viewport.maximized = Some(true);
    viewport.inner_rect = Some(egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(1920.0, 1080.0),
    ));
    let mut out = ctx.run_ui(input, |_| {
        let saved = app.session_snapshot(&ctx);
        assert_eq!(saved.window, Some([1100.0, 700.0]));
        assert_eq!(saved.window_position, Some([80.0, 90.0]));
        assert!(saved.maximized);
    });
    out.textures_delta.clear();
}

#[test]
fn installed_font_faces_have_valid_indices_and_unique_names() {
    let fonts = available_system_fonts();
    #[cfg(windows)]
    assert!(!fonts.is_empty());
    let mut names = std::collections::HashSet::new();
    for (name, path, index) in fonts {
        assert!(names.insert(name));
        let bytes = std::fs::read(path).unwrap();
        assert!(ttf_parser::Face::parse(&bytes, *index).is_ok());
    }
}

#[test]
fn selection_count_is_visible_without_opening_details() {
    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx.clone());
    let mut app = NotepadApp::new(&cc, None, PathBuf::from("unused-selection-status.json"));
    app.active_tab_mut().doc.text = "abcde".into();
    app.active_tab_mut().doc.selection_chars = 3;
    let mut output = ctx.run_ui(Default::default(), |ui| app.status_ui(ui, &mut Vec::new()));
    assert!(output.shapes.iter().any(
        |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "3 sur 5 caractère(s)")
    ));
    output.textures_delta.clear();
}
