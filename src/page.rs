//! Mise en page et pagination pour l'apercu et l'impression.
//!
//! La geometrie est exprimee en points PostScript (1/72 pouce), comme le
//! pilote d'impression GDI ; la mesure du texte est injectee par l'appelant
//! (`egui` pour l'apercu, `GetTextExtentPoint32W` pour l'impression), afin
//! qu'une seule et meme algorithmie de decoupe serve aux deux.

use serde::{Deserialize, Serialize};

use crate::header;

/// Bandes reservees en haut et en bas de chaque page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bands {
    pub header: bool,
    pub footer: bool,
}

impl Bands {
    /// Bandes a reserver d'apres les modeles de « Mise en page ».
    pub fn from_setup(setup: &PageSetup) -> Bands {
        Bands {
            header: !setup.header.trim().is_empty(),
            footer: !setup.footer.trim().is_empty(),
        }
    }
}

/// Formats de papier proposés par « Mise en page ».
pub const PAPERS: [(&str, f32, f32); 4] = [
    ("A4", 210.0, 297.0),
    ("Lettre", 215.9, 279.4),
    ("Legal", 215.9, 355.6),
    ("A5", 148.0, 210.0),
];

/// Marges par defaut : 19 mm (0,75 pouce), comme le Bloc-notes.
pub const DEFAULT_MARGIN_MM: f32 = 19.05;

/// Mise en page du document, persistee dans la session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PageSetup {
    /// Indice dans `PAPERS`.
    pub paper: u8,
    pub landscape: bool,
    /// Marges en millimetres : haut, droite, bas, gauche.
    pub margins: [f32; 4],
    /// Modele d'en-tete (`&f`, `&p`, ...). Vide = pas d'en-tete.
    pub header: String,
    /// Modele de pied de page. Vide = pas de pied de page.
    pub footer: String,
}

impl Default for PageSetup {
    fn default() -> Self {
        PageSetup {
            paper: 0,
            landscape: false,
            margins: [DEFAULT_MARGIN_MM; 4],
            header: header::DEFAULT_HEADER.to_string(),
            footer: header::DEFAULT_FOOTER.to_string(),
        }
    }
}

impl PageSetup {
    /// Corrige les valeurs hors bornes (session modifiee a la main).
    pub fn sanitized(&self) -> PageSetup {
        PageSetup {
            paper: self.paper.min(PAPERS.len() as u8 - 1),
            landscape: self.landscape,
            margins: self.margins.map(|value| {
                if value.is_finite() {
                    value.clamp(0.0, 100.0)
                } else {
                    DEFAULT_MARGIN_MM
                }
            }),
            header: header::sanitize_template(&self.header),
            footer: header::sanitize_template(&self.footer),
        }
    }

    pub fn paper_label(&self) -> &'static str {
        PAPERS[self.paper.min(PAPERS.len() as u8 - 1) as usize].0
    }

    /// Dimensions du papier en millimetres, orientation comprise.
    pub fn size_mm(&self) -> (f32, f32) {
        let (_, width, height) = PAPERS[self.paper.min(PAPERS.len() as u8 - 1) as usize];
        if self.landscape {
            (height, width)
        } else {
            (width, height)
        }
    }

    /// Dimensions du papier en points.
    pub fn size_pt(&self) -> (f32, f32) {
        let (width, height) = self.size_mm();
        (mm_to_pt(width), mm_to_pt(height))
    }

    /// Marges en points : haut, droite, bas, gauche.
    pub fn margins_pt(&self) -> [f32; 4] {
        self.margins.map(mm_to_pt)
    }
}

pub fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

/// Document decoupe en pages de lignes deja repliees.
#[derive(Debug, Clone, PartialEq)]
pub struct PagedDoc {
    pub pages: Vec<Vec<String>>,
    /// Hauteur de ligne en points.
    pub line_height_pt: f32,
    pub font_pt: f32,
    /// Largeur et hauteur du papier en points.
    pub size_pt: (f32, f32),
    /// Marges en points : haut, droite, bas, gauche.
    pub margins_pt: [f32; 4],
    /// Bandes reservees en haut et en bas de la feuille.
    pub bands: Bands,
}

impl PagedDoc {
    /// Hauteur reservee a une bande (en-tete ou pied), en points.
    fn band_pt(&self) -> f32 {
        self.line_height_pt * 1.6
    }

    /// Hauteur de l'en-tete, en points (0 s'il est absent).
    pub fn header_pt(&self) -> f32 {
        if self.bands.header { self.band_pt() } else { 0.0 }
    }

    /// Hauteur du pied de page, en points (0 s'il est absent).
    pub fn footer_pt(&self) -> f32 {
        if self.bands.footer { self.band_pt() } else { 0.0 }
    }

    /// Ordonnee (en points) de la premiere ligne de texte.
    pub fn text_top_pt(&self) -> f32 {
        self.margins_pt[0] + self.header_pt()
    }

    /// Hauteur disponible pour le texte, bandes deduites.
    pub fn text_height_pt(&self) -> f32 {
        (self.size_pt.1
            - self.margins_pt[0]
            - self.margins_pt[2]
            - self.header_pt()
            - self.footer_pt())
        .max(0.0)
    }
}

/// Replie et pagine `text`.
///
/// `measure` renvoie la largeur en points d'une ligne candidate ; il est
/// appele de facon gloutonne, mot par mot.
pub fn paginate(
    text: &str,
    setup: &PageSetup,
    font_pt: f32,
    bands: Bands,
    measure: &mut dyn FnMut(&str) -> f32,
) -> PagedDoc {
    let setup = setup.sanitized();
    paginate_sized(
        text,
        setup.size_pt(),
        setup.margins_pt(),
        font_pt,
        bands,
        measure,
    )
}

/// Pagine `text` sur une feuille de dimensions imposees (en points).
///
/// L'impression utilise cette variante : la taille reelle vient du pilote
/// (`PHYSICALWIDTH`/`PHYSICALHEIGHT`) et non du format choisi dans « Mise en
/// page », ce qui evite de dessiner hors de la feuille si le pilote refuse le
/// format demande.
pub fn paginate_sized(
    text: &str,
    size_pt: (f32, f32),
    margins_pt: [f32; 4],
    font_pt: f32,
    bands: Bands,
    measure: &mut dyn FnMut(&str) -> f32,
) -> PagedDoc {
    let font_pt = if font_pt.is_finite() {
        font_pt.clamp(4.0, 200.0)
    } else {
        11.0
    };
    let line_height_pt = font_pt * 1.45;
    let size_pt = (size_pt.0.max(1.0), size_pt.1.max(1.0));
    let margins_pt = margins_pt.map(|value| if value.is_finite() { value.max(0.0) } else { 0.0 });
    let usable_width = (size_pt.0 - margins_pt[1] - margins_pt[3]).max(1.0);
    let mut doc = PagedDoc {
        pages: Vec::new(),
        line_height_pt,
        font_pt,
        size_pt,
        margins_pt,
        bands,
    };
    let usable_height = doc.text_height_pt();
    let lines_per_page = ((usable_height / line_height_pt).floor() as usize).max(1);

    let mut lines: Vec<String> = Vec::new();
    for source in text.split('\n') {
        let source = source.replace('\t', "    ");
        lines.extend(wrap_line(&source, usable_width, measure));
    }
    if lines.is_empty() {
        lines.push(String::new());
    }

    for chunk in lines.chunks(lines_per_page) {
        doc.pages.push(chunk.to_vec());
    }
    if doc.pages.is_empty() {
        doc.pages.push(Vec::new());
    }
    doc
}

/// Indices (a partir de 0) des pages a imprimer.
///
/// `range` est une plage 1-based incluse, telle que saisie dans la boite
/// d'impression ; `None` imprime tout le document.
pub fn selected_pages(total: usize, range: Option<(usize, usize)>) -> Vec<usize> {
    if total == 0 {
        return Vec::new();
    }
    match range {
        None => (0..total).collect(),
        Some((from, to)) => {
            let from = from.max(1);
            if from > total {
                return Vec::new();
            }
            let to = to.max(from).min(total);
            (from - 1..to).collect()
        }
    }
}

fn wrap_line(
    line: &str,
    max_width: f32,
    measure: &mut dyn FnMut(&str) -> f32,
) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let indent: String = line.chars().take_while(|c| *c == ' ').collect();
    let mut lines = Vec::new();
    let mut current = indent.clone();

    for word in line.split(' ').filter(|word| !word.is_empty()) {
        let candidate = if current.trim().is_empty() {
            format!("{current}{word}")
        } else {
            format!("{current} {word}")
        };
        if current.trim().is_empty() || measure(&candidate) <= max_width {
            current = candidate;
        } else {
            lines.push(current.trim_end().to_string());
            current = word.to_string();
        }
    }
    if !current.trim_end().is_empty() || lines.is_empty() {
        lines.push(current.trim_end().to_string());
    }

    // Un mot plus large que la page est coupe en morceaux mesurables.
    let mut result = Vec::with_capacity(lines.len());
    for line in lines {
        if measure(&line) <= max_width {
            result.push(line);
            continue;
        }
        let mut piece = String::new();
        for ch in line.chars() {
            let mut candidate = piece.clone();
            candidate.push(ch);
            if !piece.is_empty() && measure(&candidate) > max_width {
                result.push(piece);
                piece = String::new();
            }
            piece.push(ch);
        }
        if !piece.is_empty() {
            result.push(piece);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bandes de reference pour les tests.
    const NO_BANDS: Bands = Bands {
        header: false,
        footer: false,
    };
    const FOOTER_ONLY: Bands = Bands {
        header: false,
        footer: true,
    };

    /// Mesure synthetique : 5 points par caractere.
    fn fake_measure() -> impl FnMut(&str) -> f32 {
        |line: &str| line.chars().count() as f32 * 5.0
    }

    #[test]
    fn wraps_words_to_page_width() {
        let setup = PageSetup {
            margins: [10.0, 10.0, 10.0, 10.0],
            ..PageSetup::default()
        };
        let text = "mot ".repeat(2000);
        let mut measure = fake_measure();
        let doc = paginate(&text, &setup, 11.0, NO_BANDS, &mut measure);
        let width_pt = doc.size_pt.0 - 2.0 * mm_to_pt(10.0);
        for page in &doc.pages {
            for line in page {
                assert!(measure(line) <= width_pt + 0.01, "ligne trop large : {line}");
            }
        }
        assert!(doc.pages.len() > 1);
    }

    #[test]
    fn keeps_explicit_blank_lines() {
        let mut measure = fake_measure();
        let doc = paginate("a\n\nb", &PageSetup::default(), 11.0, NO_BANDS, &mut measure);
        let lines: Vec<&String> = doc.pages.iter().flatten().collect();
        assert_eq!(lines, vec!["a", "", "b"]);
    }

    #[test]
    fn empty_document_still_has_one_page() {
        let mut measure = fake_measure();
        let doc = paginate("", &PageSetup::default(), 11.0, FOOTER_ONLY, &mut measure);
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.pages[0], vec![String::new()]);
    }

    #[test]
    fn long_word_is_split() {
        let setup = PageSetup {
            margins: [10.0, 10.0, 10.0, 10.0],
            ..PageSetup::default()
        };
        let mut measure = fake_measure();
        let word = "x".repeat(500);
        let doc = paginate(&word, &setup, 11.0, NO_BANDS, &mut measure);
        let width_pt = doc.size_pt.0 - 2.0 * mm_to_pt(10.0);
        for page in &doc.pages {
            for line in page {
                assert!(measure(line) <= width_pt + 0.01);
            }
        }
        assert!(doc.pages.iter().flatten().count() > 1);
    }

    #[test]
    fn landscape_swaps_dimensions() {
        let portrait = PageSetup::default();
        let landscape = PageSetup {
            landscape: true,
            ..PageSetup::default()
        };
        assert_eq!(portrait.size_mm(), (210.0, 297.0));
        assert_eq!(landscape.size_mm(), (297.0, 210.0));
        assert!(landscape.size_pt().0 > landscape.size_pt().1);
    }

    #[test]
    fn sanitizes_margins_and_paper() {
        let setup = PageSetup {
            paper: 99,
            margins: [-5.0, f32::NAN, 500.0, 12.0],
            ..PageSetup::default()
        }
        .sanitized();
        assert_eq!(setup.paper, (PAPERS.len() - 1) as u8);
        assert_eq!(setup.margins[0], 0.0);
        assert_eq!(setup.margins[1], DEFAULT_MARGIN_MM);
        assert_eq!(setup.margins[2], 100.0);
        assert_eq!(setup.margins[3], 12.0);
    }

    #[test]
    fn pagination_respects_available_height() {
        let setup = PageSetup {
            margins: [20.0, 20.0, 20.0, 20.0],
            ..PageSetup::default()
        };
        let mut measure = fake_measure();
        let text = (0..200)
            .map(|index| format!("ligne {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let doc = paginate(&text, &setup, 12.0, FOOTER_ONLY, &mut measure);
        let per_page = (doc.text_height_pt() / doc.line_height_pt).floor() as usize;
        for page in &doc.pages[..doc.pages.len() - 1] {
            assert_eq!(page.len(), per_page.max(1));
        }
        assert!(doc.pages.len() > 1);
    }

    #[test]
    fn sized_pagination_follows_the_device_sheet() {
        let mut measure = fake_measure();
        let doc = paginate_sized(
            "abc",
            (100.0, 50.0),
            [5.0, 5.0, 5.0, 5.0],
            10.0,
            NO_BANDS,
            &mut measure,
        );
        assert_eq!(doc.size_pt, (100.0, 50.0));
        assert_eq!(doc.margins_pt, [5.0, 5.0, 5.0, 5.0]);
        assert_eq!(doc.pages.len(), 1);
    }

    #[test]
    fn sized_pagination_clamps_negative_margins() {
        let mut measure = fake_measure();
        let doc = paginate_sized(
            "abc",
            (100.0, 100.0),
            [-5.0, -1.0, 2.0, 3.0],
            10.0,
            FOOTER_ONLY,
            &mut measure,
        );
        assert_eq!(doc.margins_pt[0], 0.0);
        assert_eq!(doc.margins_pt[1], 0.0);
        assert_eq!(doc.margins_pt[2], 2.0);
        assert_eq!(doc.margins_pt[3], 3.0);
        assert!(doc.text_height_pt() > 0.0);
    }

    #[test]
    fn header_reserves_space_above_the_text() {
        let setup = PageSetup {
            margins: [20.0, 20.0, 20.0, 20.0],
            ..PageSetup::default()
        };
        let mut measure = fake_measure();
        let plain = paginate("a", &setup, 10.0, NO_BANDS, &mut measure);
        let both = paginate(
            "a",
            &setup,
            10.0,
            Bands {
                header: true,
                footer: true,
            },
            &mut measure,
        );
        assert!(both.text_top_pt() > plain.text_top_pt());
        assert!(both.text_height_pt() < plain.text_height_pt());
        assert_eq!(both.text_top_pt(), both.margins_pt[0] + both.header_pt());
    }

    #[test]
    fn bands_follow_the_templates() {
        let mut setup = PageSetup::default();
        assert_eq!(
            Bands::from_setup(&setup),
            Bands {
                header: true,
                footer: true
            }
        );
        setup.footer.clear();
        assert_eq!(
            Bands::from_setup(&setup),
            Bands {
                header: true,
                footer: false
            }
        );
        setup.header = "   ".to_string();
        assert_eq!(Bands::from_setup(&setup), NO_BANDS);
    }

    #[test]
    fn selected_pages_clamps_the_requested_range() {
        assert_eq!(selected_pages(3, None), vec![0, 1, 2]);
        assert_eq!(selected_pages(5, Some((2, 3))), vec![1, 2]);
        assert_eq!(selected_pages(5, Some((4, 99))), vec![3, 4]);
        assert!(selected_pages(5, Some((9, 12))).is_empty());
        assert!(selected_pages(0, None).is_empty());
    }

    #[test]
    fn setup_roundtrips_through_json() {
        let setup = PageSetup {
            paper: 2,
            landscape: true,
            margins: [10.0, 20.0, 30.0, 40.0],
            header: "&cTitre".to_string(),
            footer: "Page &p/&P".to_string(),
        };
        let json = serde_json::to_string(&setup).unwrap();
        let back: PageSetup = serde_json::from_str(&json).unwrap();
        assert_eq!(back, setup);
        let partial: PageSetup = serde_json::from_str("{\"paper\": 1}").unwrap();
        assert_eq!(partial.paper, 1);
        assert_eq!(partial.margins, [DEFAULT_MARGIN_MM; 4]);
        assert_eq!(partial.header, header::DEFAULT_HEADER);
        assert_eq!(partial.footer, header::DEFAULT_FOOTER);
    }
}
