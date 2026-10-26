//! Visual block editing backed by lossless Markdown chunks.
use eframe::egui;

pub fn split(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chunk = String::new();
    let mut fence: Option<&str> = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let marker = if trimmed.starts_with("```") {
                "```"
            } else {
                "~~~"
            };
            if fence == Some(marker) {
                fence = None;
            } else if fence.is_none() {
                fence = Some(marker);
            }
        }
        chunk.push_str(line);
        if line.trim().is_empty() && fence.is_none() {
            result.push(std::mem::take(&mut chunk));
        }
    }
    if !chunk.is_empty() {
        result.push(chunk);
    }
    result
}

pub fn editor(ui: &mut egui::Ui, id: u64, text: &mut String) -> bool {
    let mut blocks = split(text);
    let mut changed = false;
    let mut remove = None;
    let mut movement = None;
    egui::ScrollArea::vertical()
        .id_salt(("block-scroll", id))
        .show(ui, |ui| {
            for (i, block) in blocks.iter_mut().enumerate() {
                ui.push_id((id, i), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("Bloc {}", i + 1));
                            if ui.small_button("↑").clicked() && i > 0 {
                                movement = Some((i, i - 1));
                            }
                            if ui.small_button("↓").clicked() {
                                movement = Some((i, i + 1));
                            }
                            if ui.small_button("Supprimer").clicked() {
                                remove = Some(i);
                            }
                        });
                        let prefix = if block.starts_with("### ") {
                            "### "
                        } else if block.starts_with("## ") {
                            "## "
                        } else if block.starts_with("# ") {
                            "# "
                        } else if block.starts_with("> ") {
                            "> "
                        } else {
                            ""
                        };
                        if block.starts_with("- [ ] ") || block.starts_with("- [x] ") {
                            let mut done = block.starts_with("- [x]");
                            if ui.checkbox(&mut done, "Terminée").changed() {
                                block.replace_range(0..6, if done { "- [x] " } else { "- [ ] " });
                                changed = true;
                            }
                        }
                        if block.starts_with('|')
                            && block
                                .lines()
                                .nth(1)
                                .map(|line| line.contains("---"))
                                .unwrap_or(false)
                        {
                            let mut rows: Vec<Vec<String>> = block
                                .lines()
                                .filter(|line| !line.trim().is_empty())
                                .map(|line| {
                                    line.trim()
                                        .trim_matches('|')
                                        .split('|')
                                        .map(|cell| cell.trim().to_owned())
                                        .collect()
                                })
                                .collect();
                            let mut table_changed = false;
                            egui::Grid::new(("table-block", id, i))
                                .striped(true)
                                .show(ui, |ui| {
                                    for (r, row) in rows.iter_mut().enumerate() {
                                        if r == 1 {
                                            continue;
                                        }
                                        for cell in row {
                                            table_changed |= ui
                                                .add(
                                                    egui::TextEdit::singleline(cell)
                                                        .desired_width(90.0),
                                                )
                                                .changed();
                                        }
                                        ui.end_row();
                                    }
                                });
                            if ui.small_button("+ Ligne").clicked() {
                                rows.push(vec![String::new(); rows[0].len()]);
                                table_changed = true;
                            }
                            if table_changed {
                                *block = rows
                                    .iter()
                                    .map(|row| format!("| {} |\n", row.join(" | ")))
                                    .collect::<String>()
                                    + "\n";
                                changed = true;
                            }
                        } else if prefix.is_empty() {
                            changed |= ui
                                .add(
                                    egui::TextEdit::multiline(block)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(2),
                                )
                                .changed();
                        } else {
                            let trailing = block[block.trim_end_matches('\n').len()..].to_owned();
                            let mut content =
                                block[prefix.len()..].trim_end_matches('\n').to_owned();
                            let edit = egui::TextEdit::multiline(&mut content)
                                .desired_width(f32::INFINITY)
                                .desired_rows(1);
                            let response =
                                if prefix.starts_with('#') {
                                    ui.add(edit.font(egui::FontId::proportional(
                                        if prefix == "# " { 28.0 } else { 21.0 },
                                    )))
                                } else {
                                    ui.add(edit)
                                };
                            if response.changed() {
                                *block = format!("{prefix}{content}{trailing}");
                                changed = true;
                            }
                        }
                    });
                });
            }
            ui.menu_button("+ Ajouter un bloc", |ui| {
                for (label, source) in [
                    ("Texte", "Texte\n\n"),
                    ("Titre", "# Titre\n\n"),
                    ("Sous-titre", "## Sous-titre\n\n"),
                    ("Tâche", "- [ ] Tâche\n\n"),
                    ("Citation", "> Citation\n\n"),
                    ("Code", "```\ncode\n```\n\n"),
                    ("Tableau", "| Nom | Valeur |\n| --- | --- |\n| | |\n\n"),
                ] {
                    if ui.button(label).clicked() {
                        if let Some(last) = blocks.last_mut() {
                            if !last.ends_with("\n\n") {
                                last.push_str("\n\n");
                            }
                        }
                        blocks.push(source.into());
                        changed = true;
                        ui.close();
                    }
                }
            });
        });
    if let Some(i) = remove {
        blocks.remove(i);
        changed = true;
    }
    if let Some((from, to)) = movement {
        if to < blocks.len() {
            blocks.swap(from, to);
            let n = blocks.len();
            for block in blocks.iter_mut().take(n.saturating_sub(1)) {
                if !block.ends_with("\n\n") {
                    block.push_str("\n\n");
                }
            }
            changed = true;
        }
    }
    if changed {
        *text = blocks.concat();
    }
    changed
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    fn frame(ctx: &egui::Context, text: &mut String, events: Vec<egui::Event>) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 800.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            super::editor(ui, 1, text);
        });
        output.textures_delta.clear();
        output
    }

    #[test]
    fn visual_editor_renders_without_rewriting_source() {
        let ctx = egui::Context::default();
        let original = "# Épreuve\n\n- [ ] Tâche\n\n| Nom | Valeur |\n| --- | --- |\n| été | 3 |\n\n```\na\n\nb\n```\n";
        let mut text = original.to_owned();
        let output = frame(&ctx, &mut text, vec![]);
        assert!(!output.shapes.is_empty());
        assert_eq!(text, original);
    }

    #[test]
    fn task_checkbox_changes_markdown_and_keeps_other_blocks() {
        let ctx = egui::Context::default();
        let mut text = "- [ ] Action\n\nTexte inchangé".to_owned();
        let _ = frame(&ctx, &mut text, vec![]);
        let output = frame(&ctx, &mut text, vec![]);
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(t) if t.galley.text() == "Terminée" => {
                    Some(t.pos + egui::vec2(5.0, 5.0))
                }
                _ => None,
            })
            .expect("Checkbox label must be rendered");
        frame(
            &ctx,
            &mut text,
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
            &ctx,
            &mut text,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert_eq!(text, "- [x] Action\n\nTexte inchangé");
    }

    #[test]
    fn preserves_fenced_code_blank_lines_and_unicode() {
        let text = "# Été\n\n```rust\nlet x = 1;\n\nlet y = 2;\n```\n\nTexte final";
        let chunks = super::split(text);
        assert_eq!(chunks.concat(), text);
        assert_eq!(chunks.len(), 3);
    }
}
