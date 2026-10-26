//! Mise en forme Markdown portant sur des lignes entieres.
//!
//! L'interface ne fournit que la selection courante, en index de caracteres ;
//! toute la logique de prefixe et de bascule vit ici. Elle est ainsi testable
//! sans fenetre, et l'editeur se contente d'appliquer le resultat.

/// Style de bloc demande par la barre de mise en forme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineFormat {
    Heading(u8),
    Bullet,
    Ordered,
    Quote,
    CodeBlock,
}

/// Remplacement a appliquer au document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineEdit {
    /// Premier caractere du bloc remplace.
    pub start: usize,
    /// Fin (exclue) du bloc remplace.
    pub end: usize,
    /// Nouveau texte du bloc.
    pub replacement: String,
    /// Selection a poser apres l'edition : le bloc formate.
    pub selection: (usize, usize),
}

/// Applique `format` aux lignes couvertes par `start..end`.
///
/// Renvoie `None` quand la selection est hors du document.
pub fn apply(text: &str, start: usize, end: usize, format: LineFormat) -> Option<LineEdit> {
    let total = text.chars().count();
    if start > end || end > total {
        return None;
    }
    let start_byte = char_to_byte(text, start);
    let end_byte = char_to_byte(text, end);
    let line_start = text[..start_byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = text[end_byte..]
        .find('\n')
        .map(|i| end_byte + i)
        .unwrap_or(text.len());
    // Une selection prise a l'interieur d'un bloc de code doit aussi pouvoir
    // le retirer : on etend alors le bloc a ses deux cloutures.
    let (line_start, line_end) = match format {
        LineFormat::CodeBlock => {
            fenced_block(text, line_start, line_end).unwrap_or((line_start, line_end))
        }
        _ => (line_start, line_end),
    };
    let block = &text[line_start..line_end];
    let mut lines: Vec<String> = block.split('\n').map(str::to_string).collect();
    let meaningful: Vec<&String> = lines.iter().filter(|line| !line.trim().is_empty()).collect();
    let has_prefix = |prefix: &str| {
        !meaningful.is_empty()
            && meaningful
                .iter()
                .all(|line| line.trim_start().starts_with(prefix))
    };
    let has_ordered = || {
        !meaningful.is_empty()
            && meaningful
                .iter()
                .all(|line| ordered_prefix_len(line).is_some())
    };

    match format {
        LineFormat::Heading(level) => {
            let prefix = format!("{} ", "#".repeat(level.clamp(1, 6) as usize));
            if has_prefix(&prefix) {
                lines = lines.iter().map(|line| remove_prefix(line, &prefix)).collect();
            } else {
                lines = lines
                    .iter()
                    .map(|line| add_prefix(&strip_heading(line), &prefix))
                    .collect();
            }
        }
        LineFormat::Bullet => {
            if has_prefix("- ") {
                lines = lines.iter().map(|line| remove_prefix(line, "- ")).collect();
            } else {
                lines = lines
                    .iter()
                    .map(|line| {
                        let clean = remove_prefix(line, "- ");
                        let clean = remove_prefix(&clean, "* ");
                        let clean = remove_prefix(&clean, "+ ");
                        add_prefix(&clean, "- ")
                    })
                    .collect();
            }
        }
        LineFormat::Ordered => {
            if has_ordered() {
                lines = lines
                    .iter()
                    .map(|line| {
                        let prefix = ordered_prefix(line).unwrap_or_default();
                        remove_prefix(line, &prefix)
                    })
                    .collect();
            } else {
                let mut number = 1;
                lines = lines
                    .iter()
                    .map(|line| {
                        let clean = match ordered_prefix(line) {
                            Some(prefix) => remove_prefix(line, &prefix),
                            None => line.clone(),
                        };
                        if clean.trim().is_empty() {
                            return clean;
                        }
                        let prefixed = add_prefix(&clean, &format!("{number}. "));
                        number += 1;
                        prefixed
                    })
                    .collect();
            }
        }
        LineFormat::Quote => {
            if has_prefix("> ") {
                lines = lines.iter().map(|line| remove_prefix(line, "> ")).collect();
            } else {
                lines = lines.iter().map(|line| add_prefix(line, "> ")).collect();
            }
        }
        LineFormat::CodeBlock => {
            let paired = lines.len() >= 2
                && lines.first().is_some_and(|line| is_opener(line))
                && lines.last().is_some_and(|line| is_closer(line));
            if paired {
                lines.remove(0);
                lines.pop();
            } else {
                lines.insert(0, "```".to_string());
                lines.push("```".to_string());
            }
        }
    }

    let new_block = lines.join("\n");
    let block_start = text[..line_start].chars().count();
    let block_end = block_start + block.chars().count();
    Some(LineEdit {
        start: block_start,
        end: block_end,
        selection: (block_start, block_start + new_block.chars().count()),
        replacement: new_block,
    })
}

/// Bornes du bloc de code (cloutures comprises) auquel appartient la selection.
///
/// La selection peut couvrir tout le bloc, une ou plusieurs lignes interieures,
/// ou une seule clouture : dans tous ces cas la bascule doit porter sur le bloc
/// entier, sinon elle laisserait une clouture orpheline.
fn fenced_block(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let block = &text[start..end];
    let first = block.split('\n').next().unwrap_or("");
    let last = block.rsplit('\n').next().unwrap_or("");
    // Selection couvrant deja l'ouverture et la cloture.
    if block.contains('\n') && is_opener(first) && is_closer(last) {
        return Some((start, end));
    }
    // Selection encadree par une ouverture et une cloture.
    if let (Some((open_start, open_end)), Some((close_start, close_end))) =
        (line_before(text, start), line_after(text, end))
    {
        if is_opener(&text[open_start..open_end]) && is_closer(&text[close_start..close_end]) {
            return Some((open_start, close_end));
        }
    }
    // Selection reduite a une clouture : retrouver l'autre extremite.
    if is_opener(first) {
        if let Some(close_end) = next_closer(text, end) {
            return Some((start, close_end));
        }
    }
    if is_closer(last) {
        if let Some(open_start) = previous_opener(text, start) {
            return Some((open_start, end));
        }
    }
    None
}

/// Debut et fin (octets, sans le saut de ligne) de la ligne precedant `line_start`.
fn line_before(text: &str, line_start: usize) -> Option<(usize, usize)> {
    if line_start == 0 || !text[..line_start].ends_with('\n') {
        return None;
    }
    let body = &text[..line_start - 1];
    let begin = body.rfind('\n').map(|i| i + 1).unwrap_or(0);
    Some((begin, line_start - 1))
}

/// Debut et fin de la ligne suivant `line_end`.
fn line_after(text: &str, line_end: usize) -> Option<(usize, usize)> {
    let rest = text[line_end..].strip_prefix('\n')?;
    let len = rest.find('\n').unwrap_or(rest.len());
    Some((line_end + 1, line_end + 1 + len))
}

/// Ouverture de bloc de code (` ``` `, eventuellement suivie d'un langage).
fn is_opener(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

/// Cloture de bloc de code : trois accents graves seuls.
fn is_closer(line: &str) -> bool {
    line.trim() == "```"
}

/// Fin de la premiere cloture situee apres `from`.
fn next_closer(text: &str, from: usize) -> Option<usize> {
    let mut cursor = from;
    while let Some((start, end)) = line_after(text, cursor) {
        if is_closer(&text[start..end]) {
            return Some(end);
        }
        cursor = end;
    }
    None
}

/// Debut de la derniere ouverture situee avant `from`.
fn previous_opener(text: &str, from: usize) -> Option<usize> {
    let mut cursor = from;
    while let Some((start, end)) = line_before(text, cursor) {
        if is_opener(&text[start..end]) {
            return Some(start);
        }
        cursor = start;
    }
    None
}

/// Indice d'octet du `char_index`-ieme caractere (fin du texte au dela).
pub fn char_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(offset, _)| offset)
        .unwrap_or(text.len())
}

/// Longueur de l'indentation (espaces et tabulations).
fn indent_len(line: &str) -> usize {
    line.char_indices()
        .find(|(_, c)| !matches!(c, ' ' | '\t'))
        .map(|(offset, _)| offset)
        .unwrap_or(line.len())
}

/// Retire `prefix` s'il suit l'indentation de la ligne.
fn remove_prefix(line: &str, prefix: &str) -> String {
    let indent = indent_len(line);
    let rest = &line[indent..];
    match rest.strip_prefix(prefix) {
        Some(tail) => format!("{}{}", &line[..indent], tail),
        None => line.to_string(),
    }
}

/// Insere `prefix` apres l'indentation de la ligne.
fn add_prefix(line: &str, prefix: &str) -> String {
    let indent = indent_len(line);
    format!("{}{}{}", &line[..indent], prefix, &line[indent..])
}

/// Retire les dieses de titre (`## `) en conservant l'indentation.
fn strip_heading(line: &str) -> String {
    let indent = indent_len(line);
    let rest = &line[indent..];
    let hashes = rest.chars().take_while(|c| *c == '#').count();
    if hashes == 0 {
        return line.to_string();
    }
    let tail = &rest[hashes..];
    let tail = tail.strip_prefix(' ').unwrap_or(tail);
    format!("{}{}", &line[..indent], tail)
}

/// Longueur (en octets) du marqueur de liste numerotee (`12. `) s'il existe.
fn ordered_prefix_len(line: &str) -> Option<usize> {
    let indent = indent_len(line);
    let rest = &line[indent..];
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let after = &rest[digits..];
    if after.starts_with(". ") || after.starts_with(") ") {
        Some(indent + digits + 2)
    } else {
        None
    }
}

/// Marqueur de liste numerotee de la ligne, indentation comprise.
fn ordered_prefix(line: &str) -> Option<String> {
    let len = ordered_prefix_len(line)?;
    Some(line[..len].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(text: &str, format: LineFormat) -> String {
        let end = text.chars().count();
        apply(text, 0, end, format).unwrap().replacement
    }

    #[test]
    fn heading_adds_then_removes() {
        assert_eq!(edit("Titre", LineFormat::Heading(2)), "## Titre");
        assert_eq!(edit("## Titre", LineFormat::Heading(2)), "Titre");
    }

    #[test]
    fn heading_replaces_the_existing_level() {
        assert_eq!(edit("# Titre", LineFormat::Heading(3)), "### Titre");
        assert_eq!(edit("### Titre\nsuite", LineFormat::Heading(1)), "# Titre\n# suite");
    }

    #[test]
    fn bullet_toggles_and_converts_markers() {
        assert_eq!(edit("point", LineFormat::Bullet), "- point");
        assert_eq!(edit("* point", LineFormat::Bullet), "- point");
        assert_eq!(edit("- point", LineFormat::Bullet), "point");
    }

    #[test]
    fn ordered_list_numbers_each_line() {
        assert_eq!(edit("un\ndeux\ntrois", LineFormat::Ordered), "1. un\n2. deux\n3. trois");
    }

    #[test]
    fn ordered_list_removes_any_numbering() {
        assert_eq!(edit("1. un\n2. deux", LineFormat::Ordered), "un\ndeux");
    }

    #[test]
    fn quote_toggles() {
        assert_eq!(edit("texte", LineFormat::Quote), "> texte");
        assert_eq!(edit("> texte", LineFormat::Quote), "texte");
    }

    #[test]
    fn code_block_wraps_then_unwraps() {
        let once = edit("let x = 1;", LineFormat::CodeBlock);
        assert_eq!(once, "```\nlet x = 1;\n```");
        assert_eq!(edit(&once, LineFormat::CodeBlock), "let x = 1;");
    }

    #[test]
    fn code_block_unwraps_from_inner_selection() {
        let text = "```\nlet x = 1;\n```";
        let start = 4;
        let end = start + "let x = 1;".chars().count();
        let applied = apply(text, start, end, LineFormat::CodeBlock).unwrap();
        assert_eq!(applied.start, 0);
        assert_eq!(applied.end, text.chars().count());
        assert_eq!(applied.replacement, "let x = 1;");
    }

    #[test]
    fn code_block_unwraps_from_fence_line() {
        let text = "```\nlet x = 1;\n```";
        let total = text.chars().count();
        let opener = apply(text, 0, 3, LineFormat::CodeBlock).unwrap();
        assert_eq!(opener.replacement, "let x = 1;");
        assert_eq!(opener.start, 0);
        assert_eq!(opener.end, total);
        let closer = apply(text, total - 3, total, LineFormat::CodeBlock).unwrap();
        assert_eq!(closer.replacement, "let x = 1;");
        assert_eq!(closer.start, 0);
        assert_eq!(closer.end, total);
    }

    #[test]
    fn code_block_ignores_unpaired_fence() {
        let text = "```\nlet x = 1;\nfin";
        let start = 4;
        let end = start + "let x = 1;".chars().count();
        let applied = apply(text, start, end, LineFormat::CodeBlock).unwrap();
        assert_eq!(applied.replacement, "```\nlet x = 1;\n```");
    }

    #[test]
    fn indentation_is_preserved() {
        assert_eq!(edit("    point", LineFormat::Bullet), "    - point");
        assert_eq!(edit("    - point", LineFormat::Bullet), "    point");
    }

    #[test]
    fn format_only_touches_selected_lines() {
        let text = "avant\napres\nfin";
        let start = text.chars().count() - 3;
        let end = text.chars().count();
        let applied = apply(text, start, end, LineFormat::Heading(1)).unwrap();
        assert_eq!(applied.start, 12);
        assert_eq!(applied.replacement, "# fin");
        assert_eq!(applied.selection, (12, 17));
    }

    #[test]
    fn out_of_range_selection_is_ignored() {
        assert!(apply("abc", 0, 9, LineFormat::Quote).is_none());
        assert!(apply("abc", 2, 1, LineFormat::Quote).is_none());
    }
}
