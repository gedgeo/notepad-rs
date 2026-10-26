//! Direct paginated PDF export with embedded Windows fonts.
use genpdf::{elements, fonts, style, Element};
use std::path::Path;

pub fn export(path: &Path, title: &str, text: &str) -> Result<(), String> {
    let dir =
        std::path::PathBuf::from(std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into()))
            .join("Fonts");
    let load = |name: &str| fonts::FontData::load(dir.join(name), None).map_err(|e| e.to_string());
    let family = fonts::FontFamily {
        regular: load("arial.ttf")?,
        bold: load("arialbd.ttf")?,
        italic: load("ariali.ttf")?,
        bold_italic: load("arialbi.ttf")?,
    };
    let mut doc = genpdf::Document::new(family);
    doc.set_title(title);
    doc.set_font_size(11);
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);
    for block in crate::markdown::parse(text) {
        use crate::markdown::Block;
        let spans_text = |spans: Vec<crate::markdown::Span>| {
            spans.into_iter().map(|s| s.text).collect::<String>()
        };
        match block {
            Block::Heading(level, spans) => doc.push(
                elements::Paragraph::new(spans_text(spans)).styled(
                    style::Style::new()
                        .bold()
                        .with_font_size(22u8.saturating_sub(level * 2)),
                ),
            ),
            Block::Paragraph(spans) => doc.push(elements::Paragraph::new(spans_text(spans))),
            Block::Bullet(_, spans) => {
                doc.push(elements::Paragraph::new(format!("• {}", spans_text(spans))))
            }
            Block::Ordered(_, n, spans) => doc.push(elements::Paragraph::new(format!(
                "{n}. {}",
                spans_text(spans)
            ))),
            Block::Quote(lines) => {
                for line in lines {
                    doc.push(
                        elements::Paragraph::new(spans_text(line))
                            .styled(style::Style::new().italic()),
                    );
                }
            }
            Block::Code(code) => {
                for line in code.lines() {
                    doc.push(elements::Paragraph::new(line));
                }
            }
            Block::Rule => doc.push(elements::Paragraph::new("────────────────────")),
            Block::Table { header, rows, .. } => {
                doc.push(
                    elements::Paragraph::new(
                        header
                            .into_iter()
                            .map(spans_text)
                            .collect::<Vec<_>>()
                            .join(" | "),
                    )
                    .styled(style::Style::new().bold()),
                );
                for row in rows {
                    doc.push(elements::Paragraph::new(
                        row.into_iter()
                            .map(spans_text)
                            .collect::<Vec<_>>()
                            .join(" | "),
                    ));
                }
            }
        }
        doc.push(elements::Break::new(0.5));
    }
    // Render to memory first so an unsupported glyph or layout error cannot
    // destroy an existing export.
    let mut bytes = Vec::new();
    doc.render(&mut bytes).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(windows)]
    fn exports_accented_text_without_print_dialog() {
        let path =
            std::env::temp_dir().join(format!("notepad-rs-pdf-{}.pdf", crate::workspace_io::uid()));
        super::export(
            &path,
            "Épreuve",
            "# Réunion\n\nÉté, œuvre, français.\n\n- action",
        )
        .unwrap();
        let data = std::fs::read(&path).unwrap();
        assert!(data.starts_with(b"%PDF-"));
        assert!(data.len() > 1000);
        std::fs::remove_file(path).unwrap();
    }
}
