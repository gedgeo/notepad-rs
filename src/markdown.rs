//! Markdown : analyse et rendu dans egui.
//!
//! L'analyseur couvre le sous-ensemble utile a un bloc-notes : titres ATX,
//! paragraphes, listes a puces et numerotees, citations, blocs de code
//! clotures, regles horizontales et tableaux GFM simple.
//!
//! Le rendu produit des `LayoutJob` egui (une passe de style par bloc), et non
//! du HTML : l'apercu reste donc natif, immediat et sans dependance WebView.

use eframe::egui;

// ---------------------------------------------------------------------------
// Modele
// ---------------------------------------------------------------------------

/// Style cumulable d'un fragment de texte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub link: bool,
}

/// Fragment de texte avec son style et, pour un lien, sa cible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: Style,
    pub url: Option<String>,
}

/// Alignement d'une colonne de tableau.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Bloc Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading(u8, Vec<Span>),
    Paragraph(Vec<Span>),
    /// Lignes successives d'une citation (chacune peut etre vide).
    Quote(Vec<Vec<Span>>),
    Bullet(usize, Vec<Span>),
    Ordered(usize, u32, Vec<Span>),
    Code(String),
    Rule,
    Table {
        header: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
        align: Vec<Align>,
    },
}

// ---------------------------------------------------------------------------
// Analyse
// ---------------------------------------------------------------------------

/// Analyse un document Markdown en blocs.
pub fn parse(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut blocks = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();

        if trimmed.is_empty() {
            index += 1;
            continue;
        }

        // Bloc de code cloture (``` ou ~~~).
        if let Some(fence) = fence_marker(trimmed) {
            index += 1;
            let mut code = String::new();
            while index < lines.len() {
                if lines[index].trim_start().starts_with(fence) {
                    index += 1;
                    break;
                }
                code.push_str(lines[index]);
                code.push('\n');
                index += 1;
            }
            while code.ends_with('\n') {
                code.pop();
            }
            blocks.push(Block::Code(code));
            continue;
        }

        // Titre ATX.
        if let Some((level, rest)) = heading(line) {
            blocks.push(Block::Heading(level, parse_inline(rest)));
            index += 1;
            continue;
        }

        // Regle horizontale.
        if is_rule(trimmed) {
            blocks.push(Block::Rule);
            index += 1;
            continue;
        }

        // Tableau : ligne d'en-tete suivie d'une ligne de separateurs.
        if trimmed.contains('|') && index + 1 < lines.len() && is_table_separator(lines[index + 1]) {
            let header: Vec<Vec<Span>> = split_row(line)
                .into_iter()
                .map(|cell| parse_inline(&cell))
                .collect();
            let align = table_alignment(lines[index + 1], header.len());
            index += 2;
            let mut rows = Vec::new();
            while index < lines.len() {
                let next = lines[index];
                if next.trim().is_empty() || !next.contains('|') {
                    break;
                }
                rows.push(
                    split_row(next)
                        .into_iter()
                        .map(|cell| parse_inline(&cell))
                        .collect(),
                );
                index += 1;
            }
            blocks.push(Block::Table {
                header,
                rows,
                align,
            });
            continue;
        }

        // Citation : lignes consecutives prefixees par `>`.
        if trimmed.starts_with('>') {
            let mut lines_out = Vec::new();
            while index < lines.len() {
                let current = lines[index].trim_start();
                let Some(rest) = current.strip_prefix('>') else {
                    break;
                };
                let rest = rest.strip_prefix(' ').unwrap_or(rest);
                lines_out.push(parse_inline(rest));
                index += 1;
            }
            blocks.push(Block::Quote(lines_out));
            continue;
        }

        // Element de liste.
        if let Some((indent, number, rest)) = list_item(line) {
            match number {
                Some(number) => blocks.push(Block::Ordered(indent, number, parse_inline(rest))),
                None => blocks.push(Block::Bullet(indent, parse_inline(rest))),
            }
            index += 1;
            continue;
        }

        // Paragraphe : lignes ordinaires jointes jusqu'a la prochaine structure.
        let mut paragraph = String::new();
        let mut hard_break = false;
        while index < lines.len() && !lines[index].trim().is_empty() && !is_block_start(&lines, index)
        {
            let current = lines[index];
            if !paragraph.is_empty() {
                paragraph.push(if hard_break { '\n' } else { ' ' });
            }
            // Un antislash ou deux espaces en fin de ligne forcent un retour.
            hard_break = current.ends_with("  ") || current.ends_with('\\');
            let mut text = current.trim();
            if let Some(stripped) = text.strip_suffix('\\') {
                text = stripped.trim_end();
            }
            paragraph.push_str(text);
            index += 1;
        }
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(parse_inline(&paragraph)));
        } else {
            // Securite : ne jamais boucler sur une ligne non consommee.
            index += 1;
        }
    }

    blocks
}

fn is_block_start(lines: &[&str], index: usize) -> bool {
    let line = lines[index];
    let trimmed = line.trim_start();
    if fence_marker(trimmed).is_some() || heading(line).is_some() || is_rule(trimmed) {
        return true;
    }
    if trimmed.starts_with('>') || list_item(line).is_some() {
        return true;
    }
    trimmed.contains('|') && index + 1 < lines.len() && is_table_separator(lines[index + 1])
}

fn fence_marker(trimmed: &str) -> Option<&'static str> {
    if trimmed.starts_with("```") {
        Some("```")
    } else if trimmed.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let text = rest.trim();
    // Un titre peut se terminer par des diezes decoratifs : `## Titre ##`.
    let text = text.trim_end_matches('#').trim_end();
    Some((hashes as u8, text))
}

fn is_rule(trimmed: &str) -> bool {
    let mut marker = None;
    let mut count = 0;
    for ch in trimmed.chars() {
        match ch {
            '-' | '*' | '_' => {
                if let Some(existing) = marker {
                    if existing != ch {
                        return false;
                    }
                } else {
                    marker = Some(ch);
                }
                count += 1;
            }
            ' ' | '\t' => {}
            _ => return false,
        }
    }
    count >= 3
}

fn list_item(line: &str) -> Option<(usize, Option<u32>, &str)> {
    let indent_chars = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
    let indent = (indent_chars / 2).min(6);
    let rest = &line[indent_chars..];
    for marker in ["- ", "* ", "+ "] {
        if let Some(text) = rest.strip_prefix(marker) {
            return Some((indent, None, text));
        }
    }
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && digits <= 9 {
        let after = &rest[digits..];
        for marker in [". ", ") "] {
            if let Some(text) = after.strip_prefix(marker) {
                let number = rest[..digits].parse::<u32>().ok()?;
                return Some((indent, Some(number), text));
            }
        }
    }
    None
}

fn split_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('|').unwrap_or(trimmed);
    trimmed.split('|').map(|cell| cell.trim().to_string()).collect()
}

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains('-') || !trimmed.contains('|') {
        return false;
    }
    trimmed
        .chars()
        .all(|ch| matches!(ch, '-' | '|' | ':' | ' ' | '\t'))
}

fn table_alignment(line: &str, columns: usize) -> Vec<Align> {
    let mut align = Vec::with_capacity(columns);
    for cell in split_row(line) {
        let cell = cell.trim();
        let left = cell.starts_with(':');
        let right = cell.ends_with(':');
        align.push(match (left, right) {
            (true, true) => Align::Center,
            (false, true) => Align::Right,
            _ => Align::Left,
        });
    }
    align.resize(columns, Align::Left);
    align
}

// ---------------------------------------------------------------------------
// Analyse en ligne
// ---------------------------------------------------------------------------

/// Analyse les styles en ligne d'un texte (gras, italique, code, lien...).
pub fn parse_inline(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    parse_styled(text, Style::default(), None, &mut spans);
    spans
}

fn push_text(spans: &mut Vec<Span>, text: &str, style: Style, url: Option<&str>) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut() {
        if last.style == style && last.url.as_deref() == url {
            last.text.push_str(text);
            return;
        }
    }
    spans.push(Span {
        text: text.to_string(),
        style,
        url: url.map(str::to_string),
    });
}

fn parse_styled(text: &str, style: Style, url: Option<&str>, spans: &mut Vec<Span>) {
    let chars: Vec<char> = text.chars().collect();
    let mut literal = String::new();
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];

        // Echappement.
        if ch == '\\' && index + 1 < chars.len() {
            literal.push(chars[index + 1]);
            index += 2;
            continue;
        }

        // Code en ligne.
        if ch == '`' {
            if let Some(end) = find_char(&chars, index + 1, '`') {
                push_text(spans, &literal, style, url);
                literal.clear();
                let inner: String = chars[index + 1..end].iter().collect();
                let mut code_style = style;
                code_style.code = true;
                push_text(spans, &inner, code_style, url);
                index = end + 1;
                continue;
            }
        }

        // Lien `[libelle](cible)`.
        if ch == '[' {
            if let Some((label, target, end)) = parse_link(&chars, index) {
                push_text(spans, &literal, style, url);
                literal.clear();
                let mut link_style = style;
                link_style.link = true;
                let mut inner = Vec::new();
                parse_styled(&label, link_style, Some(&target), &mut inner);
                spans.extend(inner);
                index = end;
                continue;
            }
        }

        // Emphase forte puis emphase simple.
        if (ch == '*' || ch == '_') && index + 1 < chars.len() && chars[index + 1] == ch {
            if let Some(rest) = text.get(byte_index_of(text, &chars, index + 2)..) {
                if let Some(content_end) = bold_close(rest, ch) {
                    push_text(spans, &literal, style, url);
                    literal.clear();
                    let mut bold = style;
                    bold.bold = true;
                    parse_styled(&rest[..content_end], bold, url, spans);
                    index += 2 + rest[..content_end].chars().count() + 2;
                    continue;
                }
            }
        }

        if ch == '*' || ch == '_' {
            // `_` n'ouvre pas d'emphase au milieu d'un mot (snake_case).
            let previous_alnum = index > 0 && chars[index - 1].is_alphanumeric();
            let next_alnum = index + 1 < chars.len() && chars[index + 1].is_alphanumeric();
            let skip = ch == '_' && previous_alnum && next_alnum;
            if !skip {
                if let Some(rest) = text.get(byte_index_of(text, &chars, index + 1)..) {
                    if let Some(end) = find_delimiter(rest, &ch.to_string()) {
                        if !rest[..end].is_empty() {
                            push_text(spans, &literal, style, url);
                            literal.clear();
                            let mut italic = style;
                            italic.italic = true;
                            parse_styled(&rest[..end], italic, url, spans);
                            index += 1 + rest[..end].chars().count() + 1;
                            continue;
                        }
                    }
                }
            }
        }

        // Barre.
        if ch == '~' && index + 1 < chars.len() && chars[index + 1] == '~' {
            if let Some(rest) = text.get(byte_index_of(text, &chars, index + 2)..) {
                if let Some(end) = find_delimiter(rest, "~~") {
                    push_text(spans, &literal, style, url);
                    literal.clear();
                    let mut strike = style;
                    strike.strike = true;
                    parse_styled(&rest[..end], strike, url, spans);
                    index += 2 + rest[..end].chars().count() + 2;
                    continue;
                }
            }
        }

        literal.push(ch);
        index += 1;
    }

    push_text(spans, &literal, style, url);
}

fn byte_index_of(text: &str, chars: &[char], char_index: usize) -> usize {
    if char_index >= chars.len() {
        return text.len();
    }
    chars
        .iter()
        .take(char_index)
        .map(|c| c.len_utf8())
        .sum::<usize>()
}

fn find_char(chars: &[char], from: usize, needle: char) -> Option<usize> {
    (from..chars.len()).find(|index| chars[*index] == needle)
}

/// Fin du contenu d'une emphase forte (`**` ou `__`).
///
/// Un delimiteur fermant peut appartenir a une serie plus longue
/// (`***mot***`) : seuls les deux derniers caracteres ferment le gras, les
/// precedents restent dans le contenu et servent a l'italique imbrique.
fn bold_close(text: &str, marker: char) -> Option<usize> {
    let bytes = text.as_bytes();
    let needle: String = [marker, marker].iter().collect();
    let mut search = 0;
    while let Some(found) = text[search..].find(&needle) {
        let at = search + found;
        if at > 0 && bytes[at - 1] == b'\\' {
            search = at + needle.len();
            continue;
        }
        let mut run_end = at;
        while run_end < bytes.len() && bytes[run_end] == marker as u8 {
            run_end += 1;
        }
        let content_end = run_end - 2;
        if content_end > 0 {
            return Some(content_end);
        }
        search = run_end;
    }
    None
}

/// Cherche le prochain delimiteur, en ignorant les echappements.
fn find_delimiter(text: &str, delimiter: &str) -> Option<usize> {    let mut search = 0;
    while let Some(found) = text[search..].find(delimiter) {
        let absolute = search + found;
        let escaped = text[..absolute].ends_with('\\');
        if !escaped {
            return Some(absolute);
        }
        search = absolute + delimiter.len();
    }
    None
}

fn parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    let close = find_char(chars, start + 1, ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = find_char(chars, close + 2, ')')?;
    let label: String = chars[start + 1..close].iter().collect();
    let target: String = chars[close + 2..end].iter().collect();
    let target = target.trim();
    let target = target.split_whitespace().next().unwrap_or(target).to_string();
    Some((label, target, end + 1))
}

/// Texte brut d'un fragment (aide aux tests de l'analyseur).
#[cfg(test)]
pub fn spans_text(spans: &[Span]) -> String {
    let mut text = String::new();
    for span in spans {
        text.push_str(&span.text);
    }
    text
}

// ---------------------------------------------------------------------------
// Rendu egui
// ---------------------------------------------------------------------------

/// Options de rendu de l'apercu.
#[derive(Clone)]
pub struct RenderOptions {
    /// Famille de police du document.
    pub regular: egui::FontFamily,
    /// Famille grasse correspondante (repli : `regular`).
    pub bold: egui::FontFamily,
    pub font_size: f32,
}

/// Rend les blocs dans `ui` (l'appelant fournit la zone defilante).
pub fn render(ui: &mut egui::Ui, blocks: &[Block], options: &RenderOptions) {
    let size = options.font_size;
    let dark = ui.visuals().dark_mode;
    let text_color = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();

    for (block_index, block) in blocks.iter().enumerate() {
        match block {
            Block::Heading(level, spans) => {
                let scale = match level {
                    1 => 1.7,
                    2 => 1.45,
                    3 => 1.25,
                    4 => 1.12,
                    5 => 1.05,
                    _ => 1.0,
                };
                ui.add_space(size * if *level <= 2 { 0.9 } else { 0.5 });
                let job = layout_job(ui, spans, size * scale, options, text_color, true);
                ui.add(egui::Label::new(job).wrap());
                if *level <= 2 {
                    ui.add_space(size * 0.2);
                }
            }
            Block::Paragraph(spans) => {
                let job = layout_job(ui, spans, size, options, text_color, false);
                ui.add(egui::Label::new(job).wrap());
                ui.add_space(size * 0.55);
            }
            Block::Quote(lines) => {
                egui::Frame::new()
                    .fill(ui.visuals().faint_bg_color)
                    .corner_radius(4)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        for spans in lines {
                            let job = layout_job(ui, spans, size, options, weak, false);
                            let mut job = job;
                            for section in &mut job.sections {
                                section.format.italics = true;
                            }
                            ui.add(egui::Label::new(job).wrap());
                        }
                    });
                ui.add_space(size * 0.4);
            }
            Block::Bullet(indent, spans) => {
                ui.horizontal_top(|ui| {
                    ui.add_space(size * 0.5 + *indent as f32 * size * 1.1);
                    ui.label(
                        egui::RichText::new("•")
                            .size(size)
                            .color(ui.visuals().strong_text_color()),
                    );
                    let job = layout_job(ui, spans, size, options, text_color, false);
                    ui.add(egui::Label::new(job).wrap());
                });
                ui.add_space(size * 0.15);
            }
            Block::Ordered(indent, number, spans) => {
                ui.horizontal_top(|ui| {
                    ui.add_space(size * 0.5 + *indent as f32 * size * 1.1);
                    ui.label(
                        egui::RichText::new(format!("{number}."))
                            .size(size)
                            .color(ui.visuals().strong_text_color()),
                    );
                    let job = layout_job(ui, spans, size, options, text_color, false);
                    ui.add(egui::Label::new(job).wrap());
                });
                ui.add_space(size * 0.15);
            }
            Block::Code(code) => {
                egui::Frame::new()
                    .fill(if dark {
                        egui::Color32::from_rgb(30, 30, 34)
                    } else {
                        egui::Color32::from_rgb(245, 245, 248)
                    })
                    .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                    .corner_radius(4)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        let job = egui::text::LayoutJob::simple(
                            code.clone(),
                            egui::FontId::monospace(size * 0.92),
                            text_color,
                            ui.available_width(),
                        );
                        ui.add(egui::Label::new(job).wrap());
                    });
                ui.add_space(size * 0.6);
            }
            Block::Rule => {
                ui.add_space(size * 0.2);
                ui.separator();
                ui.add_space(size * 0.2);
            }
            Block::Table {
                header,
                rows,
                align,
            } => {
                egui::Grid::new(("md-table", block_index))
                    .striped(true)
                    .num_columns(header.len().max(1))
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for (column, cell) in header.iter().enumerate() {
                            table_cell(ui, cell, size, options, text_color, true, align, column);
                        }
                        ui.end_row();
                        for row in rows {
                            for (column, cell) in row.iter().enumerate() {
                                table_cell(ui, cell, size, options, text_color, false, align, column);
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(size * 0.6);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn table_cell(
    ui: &mut egui::Ui,
    spans: &[Span],
    size: f32,
    options: &RenderOptions,
    color: egui::Color32,
    header: bool,
    align: &[Align],
    column: usize,
) {
    let mut job = layout_job(ui, spans, size, options, color, header);
    job.halign = match align.get(column) {
        Some(Align::Center) => egui::Align::Center,
        Some(Align::Right) => egui::Align::Max,
        _ => egui::Align::Min,
    };
    ui.add(egui::Label::new(job).wrap());
}

fn layout_job(
    ui: &egui::Ui,
    spans: &[Span],
    size: f32,
    options: &RenderOptions,
    color: egui::Color32,
    force_bold: bool,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    for span in spans {
        let bold = span.style.bold || (force_bold && !span.style.code);
        let font_id = font_for(options, span.style.code, bold, size);
        let mut format = egui::TextFormat {
            font_id,
            color: if bold && !span.style.code {
                ui.visuals().strong_text_color()
            } else {
                color
            },
            ..Default::default()
        };
        format.line_height = Some((size * 1.45).max(format.font_id.size * 1.45));
        if span.style.italic {
            format.italics = true;
        }
        if span.style.strike {
            format.strikethrough = egui::Stroke::new(1.0, color);
        }
        if span.style.code {
            format.background = ui.visuals().code_bg_color;
            format.color = ui.visuals().text_color();
        }
        if span.style.link {
            format.color = ui.visuals().hyperlink_color;
            format.underline = egui::Stroke::new(1.0, ui.visuals().hyperlink_color);
        }
        job.append(&span.text, 0.0, format);
    }
    job
}

fn font_for(options: &RenderOptions, code: bool, bold: bool, size: f32) -> egui::FontId {
    if code {
        return egui::FontId::monospace(size * 0.95);
    }
    let family = if bold {
        options.bold.clone()
    } else {
        options.regular.clone()
    };
    egui::FontId::new(size, family)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn text(spans: &[Span]) -> String {
        spans_text(spans)
    }

    #[test]
    fn parses_headings_and_paragraphs() {
        let blocks = parse("# Titre\n\nUn **gras** et un *italique*.");
        assert_eq!(blocks.len(), 2);
        match &blocks[0] {
            Block::Heading(level, spans) => {
                assert_eq!(*level, 1);
                assert_eq!(text(spans), "Titre");
            }
            other => panic!("bloc inattendu : {other:?}"),
        }
        match &blocks[1] {
            Block::Paragraph(spans) => {
                assert_eq!(text(spans), "Un gras et un italique.");
                assert!(spans.iter().any(|span| span.style.bold && span.text == "gras"));
                assert!(spans.iter().any(|span| span.style.italic && span.text == "italique"));
            }
            other => panic!("bloc inattendu : {other:?}"),
        }
    }

    #[test]
    fn parses_lists_quotes_and_rules() {
        let blocks = parse("- un\n- deux\n\n1. premier\n2. second\n\n> cite\n\n---");
        assert!(matches!(blocks[0], Block::Bullet(0, _)));
        assert!(matches!(blocks[1], Block::Bullet(0, _)));
        assert!(matches!(blocks[2], Block::Ordered(0, 1, _)));
        assert!(matches!(blocks[3], Block::Ordered(0, 2, _)));
        assert!(matches!(&blocks[4], Block::Quote(lines) if lines.len() == 1));
        assert!(matches!(blocks[5], Block::Rule));
    }

    #[test]
    fn parses_fenced_code_verbatim() {
        let blocks = parse("```rust\nlet **x** = 1;\n```\nfin");
        match &blocks[0] {
            Block::Code(code) => assert_eq!(code, "let **x** = 1;"),
            other => panic!("bloc inattendu : {other:?}"),
        }
        assert!(matches!(&blocks[1], Block::Paragraph(spans) if text(spans) == "fin"));
    }

    #[test]
    fn parses_tables_with_alignment() {
        let blocks = parse("| Nom | Age |\n|:----|----:|\n| Ada | 36 |\n| Lin | 44 |");
        match &blocks[0] {
            Block::Table { header, rows, align } => {
                assert_eq!(header.len(), 2);
                assert_eq!(text(&header[0]), "Nom");
                assert_eq!(rows.len(), 2);
                assert_eq!(text(&rows[1][1]), "44");
                assert_eq!(align, &[Align::Left, Align::Right]);
            }
            other => panic!("bloc inattendu : {other:?}"),
        }
    }

    #[test]
    fn parses_inline_links_and_code() {
        let spans = parse_inline("Voir [le site](https://exemple.fr) et `code`.");
        assert_eq!(spans_text(&spans), "Voir le site et code.");
        assert!(spans
            .iter()
            .any(|span| span.style.link && span.url.as_deref() == Some("https://exemple.fr")));
        assert!(spans.iter().any(|span| span.style.code && span.text == "code"));
    }

    #[test]
    fn handles_escapes_and_snake_case() {
        let spans = parse_inline("\\*pas gras\\* et snake_case_name");
        assert_eq!(spans_text(&spans), "*pas gras* et snake_case_name");
        assert!(spans.iter().all(|span| !span.style.italic));
    }

    #[test]
    fn parses_nested_emphasis() {
        let spans = parse_inline("**gras et *italique***");
        assert!(spans
            .iter()
            .any(|span| span.style.bold && span.style.italic && span.text == "italique"));
        assert!(spans.iter().any(|span| span.style.bold && span.text == "gras et "));
    }

    #[test]
    fn joins_wrapped_lines_and_keeps_hard_breaks() {
        let blocks = parse("premiere\nseconde\n\nligne  \nsuivante");
        match &blocks[0] {
            Block::Paragraph(spans) => assert_eq!(text(spans), "premiere seconde"),
            other => panic!("bloc inattendu : {other:?}"),
        }
        match &blocks[1] {
            Block::Paragraph(spans) => assert_eq!(text(spans), "ligne\nsuivante"),
            other => panic!("bloc inattendu : {other:?}"),
        }
    }

    #[test]
    fn bullet_indentation_is_kept() {
        let blocks = parse("- a\n  - b");
        assert!(matches!(&blocks[0], Block::Bullet(0, _)));
        assert!(matches!(&blocks[1], Block::Bullet(1, _)));
    }

    #[test]
    fn empty_document_has_no_blocks() {
        assert!(parse("").is_empty());
        assert!(parse("\n\n  \n").is_empty());
    }
}
