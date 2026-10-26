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

fn style_block(block: &mut String, marker: &str) {
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
    let suffix = &block[block.trim_end_matches('\n').len()..];
    let body = block[prefix.len()..].trim_end_matches('\n');
    let styled =
        if body.starts_with(marker) && body.ends_with(marker) && body.len() >= marker.len() * 2 {
            body[marker.len()..body.len() - marker.len()].to_owned()
        } else {
            format!("{marker}{body}{marker}")
        };
    *block = format!("{prefix}{styled}{suffix}");
}

#[cfg(test)]
pub fn editor(ui: &mut egui::Ui, id: u64, text: &mut String) -> bool {
    editor_selected(ui, id, text, &mut None)
}

pub fn editor_selected(
    ui: &mut egui::Ui,
    id: u64,
    text: &mut String,
    selected: &mut Option<usize>,
) -> bool {
    ui.spacing_mut().interact_size.y = 24.0;
    ui.spacing_mut().button_padding = egui::vec2(4.0, 2.0);
    let mut blocks = split(text);
    let mut changed = false;
    let mut remove = None;
    let mut movement = None;
    egui::ScrollArea::vertical()
        .id_salt(("block-scroll", id))
        .show(ui, |ui| {
            for (i, block) in blocks.iter_mut().enumerate() {
                ui.push_id((id, i), |ui| {
                    if *selected == Some(i) && !block.starts_with("```") && !block.starts_with('|')
                    {
                        ui.horizontal(|ui| {
                            ui.add_space(30.0);
                            if ui
                                .button(egui::RichText::new("B").strong())
                                .on_hover_text("Gras sur le bloc")
                                .clicked()
                            {
                                style_block(block, "**");
                                changed = true;
                            }
                            if ui
                                .button(egui::RichText::new("I").italics())
                                .on_hover_text("Italique sur le bloc")
                                .clicked()
                            {
                                style_block(block, "*");
                                changed = true;
                            }
                            ui.menu_button(egui_phosphor::regular::LINK, |ui| {
                                let key = egui::Id::unique(("block-url", id, i));
                                let mut url = ui.ctx().data_mut(|data| {
                                    data.get_temp::<String>(key)
                                        .unwrap_or_else(|| "https://".into())
                                });
                                ui.label("Adresse du lien");
                                ui.text_edit_singleline(&mut url);
                                ui.ctx().data_mut(|data| data.insert_temp(key, url.clone()));
                                if ui.button("Insérer dans le bloc").clicked()
                                    && (url.starts_with("https://") || url.starts_with("http://"))
                                {
                                    let suffix =
                                        block[block.trim_end_matches('\n').len()..].to_owned();
                                    let body = block.trim_end_matches('\n').to_owned();
                                    *block = format!("[{body}]({url}){suffix}");
                                    changed = true;
                                    ui.close();
                                }
                            });
                        });
                    }
                    let frame = egui::Frame::NONE.inner_margin(egui::Margin::symmetric(4, 2));
                    let group = frame.show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.menu_button(egui_phosphor::regular::DOTS_SIX_VERTICAL, |ui| {
                                *selected = Some(i);
                                if ui.button("Monter").clicked() && i > 0 {
                                    movement = Some((i, i - 1));
                                    ui.close();
                                }
                                if ui.button("Descendre").clicked() {
                                    movement = Some((i, i + 1));
                                    ui.close();
                                }
                                if ui.button("Supprimer").clicked() {
                                    remove = Some(i);
                                    *selected = None;
                                    ui.close();
                                }
                            })
                            .response
                            .on_hover_text("Actions du bloc");
                            ui.vertical(|ui| {
                                ui.set_width(ui.available_width().max(80.0));
                                let suffix = block[block.trim_end_matches('\n').len()..].to_owned();
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
                                                .map(|s| s.trim().to_owned())
                                                .collect()
                                        })
                                        .collect();
                                    let mut table_changed = false;
                                    let cell_width = ((ui.available_width() - 20.0)
                                        / rows[0].len().max(1) as f32)
                                        .max(100.0);
                                    egui::Grid::new(("table", id, i))
                                        .min_col_width(cell_width)
                                        .striped(true)
                                        .show(ui, |ui| {
                                            for (r, row) in rows.iter_mut().enumerate() {
                                                if r == 1 {
                                                    continue;
                                                }
                                                for cell in row {
                                                    table_changed |= ui
                                                        .add_sized(
                                                            [cell_width, 32.0],
                                                            egui::TextEdit::singleline(cell)
                                                                .desired_width(cell_width),
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
                                        *selected = Some(i);
                                    }
                                } else if block.starts_with("- [ ] ") || block.starts_with("- [x] ")
                                {
                                    let mut done = block.starts_with("- [x]");
                                    let mut content = block[6..].trim_end_matches('\n').to_owned();
                                    ui.horizontal(|ui| {
                                        let check = ui
                                            .scope(|ui| {
                                                ui.visuals_mut().widgets.inactive.bg_stroke =
                                                    egui::Stroke::new(
                                                        1.0,
                                                        egui::Color32::from_rgb(100, 116, 139),
                                                    );
                                                ui.checkbox(&mut done, "")
                                            })
                                            .inner;
                                        let edit = ui.add(
                                            egui::TextEdit::singleline(&mut content)
                                                .desired_width(ui.available_width())
                                                .frame(egui::Frame::NONE)
                                                .font(egui::FontId::proportional(17.0)),
                                        );
                                        let edited = check.changed() || edit.changed();
                                        check.labelled_by(edit.id);
                                        if edited {
                                            *block = format!(
                                                "- [{}] {content}{suffix}",
                                                if done { "x" } else { " " }
                                            );
                                            changed = true;
                                        }
                                        if edit.has_focus() {
                                            *selected = Some(i);
                                        }
                                    });
                                } else {
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
                                    let mut content =
                                        block[prefix.len()..].trim_end_matches('\n').to_owned();
                                    let font = if prefix.starts_with('#') {
                                        egui::FontId::new(
                                            if prefix == "# " { 28.0 } else { 22.0 },
                                            ui.style().text_styles[&egui::TextStyle::Heading]
                                                .family
                                                .clone(),
                                        )
                                    } else {
                                        egui::FontId::proportional(17.0)
                                    };
                                    let response = ui.add(
                                        egui::TextEdit::multiline(&mut content)
                                            .font(font)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(1)
                                            .margin(egui::Vec2::ZERO)
                                            .frame(egui::Frame::NONE),
                                    );
                                    if response.changed() {
                                        *block = format!("{prefix}{content}{suffix}");
                                        changed = true;
                                    }
                                    if response.has_focus() {
                                        *selected = Some(i);
                                    }
                                }
                            });
                        });
                    });
                    if *selected == Some(i) {
                        ui.painter().rect_stroke(
                            group.response.rect,
                            4.0,
                            egui::Stroke::new(1.0, crate::ui_design::BLUE),
                            egui::StrokeKind::Outside,
                        );
                    }
                });
            }
            ui.add_space(12.0);
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
                        *selected = Some(blocks.len() - 1);
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
            *selected = Some(to);
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
    #[test]
    fn formatting_preserves_heading_and_blank_lines_and_toggles() {
        let mut block = "## Un titre\n\n".to_owned();
        super::style_block(&mut block, "**");
        assert_eq!(block, "## **Un titre**\n\n");
        super::style_block(&mut block, "**");
        assert_eq!(block, "## Un titre\n\n");
    }

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
                egui::Shape::Text(t) if t.galley.text() == "Action" => {
                    Some(t.pos + egui::vec2(-18.0, 8.0))
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
