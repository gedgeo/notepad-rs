//! Modele de document : texte normalise en `\n` en memoire, fins de ligne
//! d'origine conservees, codage conserve. Notepad lit `fWrap` dans
//! `Software\Microsoft\Notepad` ; ici ces preferences sont portees par le
//! document et par l'application.

use std::path::PathBuf;

use crate::encoding::Encoding;

/// Fin de ligne utilisee a l'enregistrement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eol {
    Crlf,
    Lf,
    Cr,
}

pub const EOL_ALL: [Eol; 3] = [Eol::Crlf, Eol::Lf, Eol::Cr];

impl Eol {
    pub fn label(self) -> &'static str {
        match self {
            Eol::Crlf => "Windows (CRLF)",
            Eol::Lf => "Unix (LF)",
            Eol::Cr => "Macintosh (CR)",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Eol::Crlf => "CRLF",
            Eol::Lf => "LF",
            Eol::Cr => "CR",
        }
    }

    /// Fin de ligne dominante du fichier charge (CRLF prioritaire en cas
    /// d'egalite, comme Notepad sur Windows).
    pub fn detect(text: &str) -> Eol {
        let mut crlf = 0usize;
        let mut lf = 0usize;
        let mut cr = 0usize;
        let bytes = text.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\r' => {
                    if bytes.get(index + 1) == Some(&b'\n') {
                        crlf += 1;
                        index += 1;
                    } else {
                        cr += 1;
                    }
                }
                b'\n' => lf += 1,
                _ => {}
            }
            index += 1;
        }
        if crlf >= lf && crlf >= cr && crlf > 0 {
            Eol::Crlf
        } else if lf >= cr && lf > 0 {
            Eol::Lf
        } else {
            Eol::Cr
        }
    }

    /// Applique cette fin de ligne au texte en memoire (qui utilise `\n`).
    pub fn apply(self, text: &str) -> String {
        match self {
            Eol::Lf => text.to_owned(),
            Eol::Crlf => text.replace('\n', "\r\n"),
            Eol::Cr => text.replace('\n', "\r"),
        }
    }
}

/// Une piece jointe ouverte dans un onglet.
pub struct Doc {
    pub id: u64,
    pub text: String,
    pub path: Option<PathBuf>,
    pub encoding: Encoding,
    pub eol: Eol,
    pub dirty: bool,
    pub wrap: bool,
    pub font_size: f32,
    /// Mode Markdown : barre de mise en forme et aperçu disponibles.
    pub markdown: bool,
    /// Onglet moodboard : la toile remplace l'éditeur texte.
    pub moodboard: bool,
    /// Octets d'origine, conserves pour reinterpreter le fichier quand on
    /// change de codage sans l'avoir modifie (comportement Notepad).
    pub raw: Option<Vec<u8>>,
    /// Selection a appliquer au prochain affichage (navigation de recherche).
    pub pending_selection: Option<(usize, usize)>,
    /// Position du curseur en index de caracteres, mise a jour a l'affichage.
    pub cursor_char: usize,
    pub selection_chars: usize,
}

impl Doc {
    pub fn new(id: u64) -> Doc {
        Doc {
            id,
            text: String::new(),
            path: None,
            encoding: Encoding::Utf8,
            eol: Eol::Crlf,
            dirty: false,
            wrap: true,
            font_size: 15.0,
            markdown: false,
            moodboard: false,
            raw: None,
            pending_selection: None,
            cursor_char: 0,
            selection_chars: 0,
        }
    }

    pub fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Sans titre".to_string());
        if self.dirty {
            format!("*{name}")
        } else {
            name
        }
    }

    /// Charge un fichier et deduit codage + fin de ligne.
    pub fn from_bytes(id: u64, path: PathBuf, bytes: Vec<u8>) -> Doc {
        let encoding = Encoding::detect(&bytes);
        let decoded = encoding.decode(&bytes);
        let eol = Eol::detect(&decoded);
        let text = normalize_newlines(&decoded);
        Doc {
            id,
            text,
            path: Some(path),
            encoding,
            eol,
            dirty: false,
            wrap: true,
            font_size: 15.0,
            markdown: false,
            moodboard: false,
            raw: Some(bytes),
            pending_selection: None,
            cursor_char: 0,
            selection_chars: 0,
        }
    }

    /// Bytes a ecrire sur disque pour la fin de ligne et le codage courants.
    pub fn bytes_for_save(&self) -> Vec<u8> {
        self.encoding.encode(&self.eol.apply(&self.text))
    }

    /// Reinterprete les octets d'origine avec un nouveau codage.
    pub fn reinterpret(&mut self, encoding: Encoding) {
        self.encoding = encoding;
        if let Some(raw) = &self.raw {
            let decoded = encoding.decode(raw);
            self.eol = Eol::detect(&decoded);
            self.text = normalize_newlines(&decoded);
            self.dirty = false;
        }
    }
}

pub fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Ligne (1-based) et colonne (1-based) pour un index de caracteres.
pub fn line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (count, ch) in text.chars().enumerate() {
        if count >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Decalage (en caracteres) du debut d'une ligne 1-based.
pub fn char_index_of_line(text: &str, line: usize) -> Option<usize> {
    if line == 0 {
        return None;
    }
    if line == 1 {
        return Some(0);
    }
    let mut current = 1usize;
    for (index, ch) in text.chars().enumerate() {
        if ch == '\n' {
            current += 1;
            if current == line {
                return Some(index + 1);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_crlf() {
        assert_eq!(Eol::detect("a\r\nb\r\nc"), Eol::Crlf);
    }

    #[test]
    fn detects_lf() {
        assert_eq!(Eol::detect("a\nb\nc"), Eol::Lf);
    }

    #[test]
    fn detects_cr() {
        assert_eq!(Eol::detect("a\rb\rc"), Eol::Cr);
    }

    #[test]
    fn normalizes_mixed_endings() {
        assert_eq!(normalize_newlines("a\r\nb\rc\nd"), "a\nb\nc\nd");
    }

    #[test]
    fn applies_endings() {
        assert_eq!(Eol::Crlf.apply("a\nb"), "a\r\nb");
        assert_eq!(Eol::Lf.apply("a\nb"), "a\nb");
        assert_eq!(Eol::Cr.apply("a\nb"), "a\rb");
    }

    #[test]
    fn line_col_is_one_based() {
        let text = "abc\ndef";
        assert_eq!(line_col(text, 0), (1, 1));
        assert_eq!(line_col(text, 2), (1, 3));
        assert_eq!(line_col(text, 4), (2, 1));
        assert_eq!(line_col(text, 6), (2, 3));
    }

    #[test]
    fn line_index_lookup() {
        let text = "abc\ndef\nghi";
        assert_eq!(char_index_of_line(text, 1), Some(0));
        assert_eq!(char_index_of_line(text, 2), Some(4));
        assert_eq!(char_index_of_line(text, 3), Some(8));
        assert_eq!(char_index_of_line(text, 4), None);
    }
}
