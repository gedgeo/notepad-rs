//! Recherche et remplacement, calques sur le comportement de la boite
//! « Rechercher / Remplacer » de Notepad : « Respecter la casse » et
//! « Boucler » (fWrapAround, nom de valeur lu dans le registre par
//! notepad.exe).

/// Resultat d'une recherche : bornes en octets dans la chaine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    pub start: usize,
    pub end: usize,
}

impl Match {
    // Utilise par les tests et par l'interface (longueur de la selection).
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.end == self.start
    }
}

fn chars_eq(a: char, b: char, match_case: bool) -> bool {
    if a == b {
        return true;
    }
    if match_case {
        return false;
    }
    if a.is_ascii() && b.is_ascii() {
        return a.eq_ignore_ascii_case(&b);
    }
    let mut left = a.to_lowercase();
    let mut right = b.to_lowercase();
    loop {
        match (left.next(), right.next()) {
            (Some(l), Some(r)) => {
                if l != r {
                    return false;
                }
            }
            (None, None) => return true,
            _ => return false,
        }
    }
}

fn match_end(hay: &str, start: usize, needle: &str, match_case: bool) -> Option<usize> {
    let mut end = start;
    let mut hay_chars = hay[start..].chars();
    for needle_char in needle.chars() {
        match hay_chars.next() {
            Some(hay_char) if chars_eq(hay_char, needle_char, match_case) => {
                end += hay_char.len_utf8();
            }
            _ => return None,
        }
    }
    Some(end)
}

/// Cherche `needle` a partir de l'octet `from`, sans bouclage.
pub fn find_from(hay: &str, needle: &str, from: usize, match_case: bool) -> Option<Match> {
    if needle.is_empty() || from > hay.len() {
        return None;
    }
    let mut start = from.min(hay.len());
    // `from` peut tomber dans un caractere : avancer jusqu'a la frontiere.
    while start < hay.len() && !hay.is_char_boundary(start) {
        start += 1;
    }
    for (offset, _) in hay[start..].char_indices() {
        let candidate = start + offset;
        if let Some(end) = match_end(hay, candidate, needle, match_case) {
            return Some(Match {
                start: candidate,
                end,
            });
        }
    }
    // Derniere position possible : la fin de chaine.
    if let Some(end) = match_end(hay, start, needle, match_case) {
        return Some(Match { start, end });
    }
    None
}

/// Recherche vers l'avant avec bouclage optionnel (case « Boucler »).
pub fn find_next(
    hay: &str,
    needle: &str,
    from: usize,
    match_case: bool,
    wrap: bool,
) -> Option<Match> {
    find_from(hay, needle, from, match_case).or_else(|| {
        if wrap && from > 0 {
            find_from(hay, needle, 0, match_case)
        } else {
            None
        }
    })
}

/// Recherche vers l'arriere avec bouclage optionnel.
pub fn find_previous(
    hay: &str,
    needle: &str,
    before: usize,
    match_case: bool,
    wrap: bool,
) -> Option<Match> {
    let mut last: Option<Match> = None;
    let limit = before.min(hay.len());
    for (offset, _) in hay.char_indices() {
        if offset >= limit {
            break;
        }
        if let Some(end) = match_end(hay, offset, needle, match_case) {
            if end <= limit {
                last = Some(Match { start: offset, end });
            }
        }
    }
    if last.is_some() || !wrap {
        return last;
    }
    // Bouclage : dernier resultat du document.
    let mut wrapped: Option<Match> = None;
    for (offset, _) in hay.char_indices() {
        if let Some(end) = match_end(hay, offset, needle, match_case) {
            wrapped = Some(Match { start: offset, end });
        }
    }
    wrapped
}

/// Remplace toutes les occurrences et renvoie le texte obtenu + le nombre de
/// remplacements effectues.
pub fn replace_all(
    hay: &str,
    needle: &str,
    replacement: &str,
    match_case: bool,
) -> (String, usize) {
    if needle.is_empty() {
        return (hay.to_owned(), 0);
    }
    let mut out = String::with_capacity(hay.len());
    let mut count = 0;
    let mut cursor = 0;
    while let Some(found) = find_from(hay, needle, cursor, match_case) {
        out.push_str(&hay[cursor..found.start]);
        out.push_str(replacement);
        cursor = found.end;
        count += 1;
        if cursor >= hay.len() {
            break;
        }
    }
    out.push_str(&hay[cursor..]);
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_case_matches_keep_real_byte_boundaries() {
        let text = "K k K";
        assert_eq!(
            find_from(text, "k", 0, false),
            Some(Match { start: 0, end: 3 })
        );
        assert_eq!(
            find_from("k", "K", 0, false),
            Some(Match { start: 0, end: 1 })
        );
        assert_eq!(replace_all(text, "k", "X", false), ("X X X".into(), 3));
        assert_eq!(
            find_previous(text, "k", 3, false, false),
            Some(Match { start: 0, end: 3 })
        );
        assert_eq!(
            find_previous("k", "K", 0, false, true),
            Some(Match { start: 0, end: 1 })
        );
    }

    #[test]
    fn finds_case_sensitively() {
        let text = "Alpha alpha ALPHA";
        let first = find_from(text, "alpha", 0, true).unwrap();
        assert_eq!(first.start, 6);
        assert_eq!(first.len(), 5);
        // « ALPHA » n'est pas une correspondance sensible a la casse.
        assert!(find_from(text, "alpha", first.end, true).is_none());
        let upper = find_from(text, "ALPHA", first.end, true).unwrap();
        assert_eq!(upper.start, 12);
        assert!(!upper.is_empty());
    }

    #[test]
    fn finds_case_insensitively() {
        let text = "Alpha alpha ALPHA";
        let first = find_from(text, "ALPHA", 0, false).unwrap();
        assert_eq!(first.start, 0);
    }

    #[test]
    fn wraps_around() {
        let text = "un deux un";
        assert!(find_from(text, "un", 4, true).is_some());
        let wrapped = find_next(text, "un", 10, true, true).unwrap();
        assert_eq!(wrapped.start, 0);
        assert!(find_next(text, "un", 10, true, false).is_none());
    }

    #[test]
    fn finds_previous_and_wraps() {
        let text = "un deux un";
        // Depuis la fin du document, l'occurrence precedente commence a 8.
        let prev = find_previous(text, "un", 10, true, false).unwrap();
        assert_eq!(prev.start, 8);
        let wrapped = find_previous(text, "un", 1, true, true).unwrap();
        assert_eq!(wrapped.start, 8);
        assert!(find_previous(text, "un", 1, true, false).is_none());
    }

    #[test]
    fn replaces_every_occurrence() {
        let (out, n) = replace_all("a-b-a-b", "a", "X", true);
        assert_eq!(out, "X-b-X-b");
        assert_eq!(n, 2);
    }

    #[test]
    fn replace_is_case_insensitive_on_demand() {
        let (out, n) = replace_all("Ab aB ab", "aB", "-", false);
        assert_eq!(out, "- - -");
        assert_eq!(n, 3);
        // En mode sensible a la casse, seule la graphie exacte est remplacee.
        let (out, n) = replace_all("Ab aB ab", "aB", "-", true);
        assert_eq!(out, "Ab - ab");
        assert_eq!(n, 1);
    }

    #[test]
    fn empty_needle_is_a_noop() {
        let (out, n) = replace_all("abc", "", "x", true);
        assert_eq!(out, "abc");
        assert_eq!(n, 0);
    }

    #[test]
    fn handles_multibyte_haystack() {
        let text = "\u{e9}t\u{e9} \u{1F600} \u{e9}t\u{e9}";
        let found = find_from(text, "\u{e9}t\u{e9}", 0, true).unwrap();
        assert_eq!(found.start, 0);
        let second = find_from(text, "\u{e9}t\u{e9}", found.end, true).unwrap();
        assert!(second.start > found.end);
    }
}
