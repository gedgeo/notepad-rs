//! En-tetes et pieds de page imprimes, comme le Bloc-notes.
//!
//! Un modele est une seule ligne de texte contenant des jetons :
//!
//! | Jeton | Valeur |
//! |-------|--------|
//! | `&f` | nom du fichier |
//! | `&p` | numero de page |
//! | `&P` | nombre total de pages (extension) |
//! | `&d` | date du jour |
//! | `&t` | heure |
//! | `&&` | esperluette |
//! | `&l`, `&c`, `&r` | repartit la suite a gauche, au centre, a droite |
//!
//! Le Bloc-notes imprime `&f` en en-tete et `Page &p` en pied de page par
//! defaut, sans alignement explicite : les deux sections sont donc a gauche.

/// En-tete par defaut du Bloc-notes.
pub const DEFAULT_HEADER: &str = "&f";

/// Pied de page par defaut du Bloc-notes.
pub const DEFAULT_FOOTER: &str = "Page &p";

/// Longueur maximale d'un modele (session modifiee a la main).
pub const MAX_LEN: usize = 200;

/// Valeurs variables injectees dans un modele, pour une page donnee.
#[derive(Clone, Copy)]
pub struct Fields<'a> {
    pub file: &'a str,
    /// Numero de page, a partir de 1.
    pub page: usize,
    /// Nombre total de pages.
    pub total: usize,
    pub date: &'a str,
    pub time: &'a str,
}

/// Contenu d'une bande, reparti en trois colonnes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sections {
    pub left: String,
    pub center: String,
    pub right: String,
}

impl Sections {
    /// Vrai quand les trois colonnes sont vides (rien a imprimer).
    pub fn is_empty(&self) -> bool {
        self.left.is_empty() && self.center.is_empty() && self.right.is_empty()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Column {
    Left,
    Center,
    Right,
}

/// Develope `template` avec les valeurs de `fields`.
///
/// Les jetons inconnus (`&x`) sont conserves tels quels, comme le fait le
/// Bloc-notes ; un `&` final est imprime litteralement.
pub fn resolve(template: &str, fields: &Fields) -> Sections {
    let mut sections = Sections::default();
    let mut column = Column::Left;
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '&' {
            push(&mut sections, column, ch);
            continue;
        }
        match chars.next() {
            Some('l') => column = Column::Left,
            Some('c') => column = Column::Center,
            Some('r') => column = Column::Right,
            Some('f') => push_str(&mut sections, column, fields.file),
            Some('p') => push_str(&mut sections, column, &fields.page.to_string()),
            Some('P') => push_str(&mut sections, column, &fields.total.to_string()),
            Some('d') => push_str(&mut sections, column, fields.date),
            Some('t') => push_str(&mut sections, column, fields.time),
            Some('&') => push(&mut sections, column, '&'),
            Some(other) => {
                push(&mut sections, column, '&');
                push(&mut sections, column, other);
            }
            None => push(&mut sections, column, '&'),
        }
    }
    // Un modele « &lA &cB &rC » laisse des espaces de separation : ils
    // decaleraient le centrage et l'alignement a droite.
    sections.left = sections.left.trim_end().to_string();
    sections.center = sections.center.trim().to_string();
    sections.right = sections.right.trim_start().to_string();
    sections
}

fn push(sections: &mut Sections, column: Column, ch: char) {
    match column {
        Column::Left => sections.left.push(ch),
        Column::Center => sections.center.push(ch),
        Column::Right => sections.right.push(ch),
    }
}

fn push_str(sections: &mut Sections, column: Column, text: &str) {
    match column {
        Column::Left => sections.left.push_str(text),
        Column::Center => sections.center.push_str(text),
        Column::Right => sections.right.push_str(text),
    }
}

/// Ramene un modele a une seule ligne imprimable.
pub fn sanitize_template(template: &str) -> String {
    let mut clean = String::new();
    for ch in template.chars() {
        let ch = if ch.is_control() { ' ' } else { ch };
        clean.push(ch);
        if clean.chars().count() >= MAX_LEN {
            break;
        }
    }
    clean.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> Fields<'static> {
        Fields {
            file: "notes.md",
            page: 2,
            total: 5,
            date: "26/10/2026",
            time: "14:05",
        }
    }

    #[test]
    fn default_header_is_the_file_name_on_the_left() {
        let sections = resolve(DEFAULT_HEADER, &fields());
        assert_eq!(sections.left, "notes.md");
        assert!(sections.center.is_empty() && sections.right.is_empty());
    }

    #[test]
    fn default_footer_is_the_page_number_on_the_left() {
        let sections = resolve(DEFAULT_FOOTER, &fields());
        assert_eq!(sections.left, "Page 2");
    }

    #[test]
    fn expands_every_token() {
        let sections = resolve("&f &d &t &p/&P", &fields());
        assert_eq!(sections.left, "notes.md 26/10/2026 14:05 2/5");
    }

    #[test]
    fn splits_into_three_columns() {
        let sections = resolve("&lA&cB&rC", &fields());
        assert_eq!(sections.left, "A");
        assert_eq!(sections.center, "B");
        assert_eq!(sections.right, "C");
    }

    #[test]
    fn column_switches_accumulate() {
        let sections = resolve("un&cdeux&rtrois&lquatre", &fields());
        assert_eq!(sections.left, "unquatre");
        assert_eq!(sections.center, "deux");
        assert_eq!(sections.right, "trois");
    }

    #[test]
    fn ampersand_and_unknown_tokens() {
        let sections = resolve("a && b &x &", &fields());
        assert_eq!(sections.left, "a & b &x &");
    }

    #[test]
    fn empty_template_prints_nothing() {
        assert!(resolve("", &fields()).is_empty());
        assert!(resolve("   ", &fields()).is_empty());
    }

    #[test]
    fn sanitize_collapses_control_characters_and_truncates() {
        assert_eq!(sanitize_template("  Page\r\n&p\t"), "Page  &p");
        assert_eq!(sanitize_template(&"x".repeat(MAX_LEN + 50)).chars().count(), MAX_LEN);
    }
}
