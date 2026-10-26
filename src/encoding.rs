//! Codages de texte reproduits d'apres le comportement observe de Notepad.
//!
//! Notepad expose cinq choix de codage : ANSI, UTF-8, UTF-8 avec BOM,
//! UTF-16 LE et UTF-16 BE. La detection repose sur les BOM puis sur la
//! validite UTF-8 (equivalent pratique du test `IsTextUnicode` importe par
//! `notepad.exe`).

use encoding_rs::WINDOWS_1252;

/// Codages proposes par Notepad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// Page de code systeme (Windows-1252 sur une installation francaise).
    Ansi,
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
}

/// Ordre d'affichage du menu « Codage ».
pub const ALL: [Encoding; 5] = [
    Encoding::Ansi,
    Encoding::Utf8,
    Encoding::Utf8Bom,
    Encoding::Utf16Le,
    Encoding::Utf16Be,
];

const BOM_UTF8: [u8; 3] = [0xEF, 0xBB, 0xBF];
const BOM_UTF16_LE: [u8; 2] = [0xFF, 0xFE];
const BOM_UTF16_BE: [u8; 2] = [0xFE, 0xFF];

impl Encoding {
    pub fn label(self) -> &'static str {
        match self {
            Encoding::Ansi => "ANSI",
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf8Bom => "UTF-8 avec BOM",
            Encoding::Utf16Le => "UTF-16 LE",
            Encoding::Utf16Be => "UTF-16 BE",
        }
    }

    /// Detection a l'ouverture : BOM d'abord, puis validite UTF-8, puis ANSI.
    pub fn detect(bytes: &[u8]) -> Encoding {
        if bytes.starts_with(&BOM_UTF8) {
            Encoding::Utf8Bom
        } else if bytes.starts_with(&BOM_UTF16_LE) {
            Encoding::Utf16Le
        } else if bytes.starts_with(&BOM_UTF16_BE) {
            Encoding::Utf16Be
        } else if std::str::from_utf8(bytes).is_ok() {
            Encoding::Utf8
        } else {
            Encoding::Ansi
        }
    }

    /// Decode les octets bruts selon le codage choisi.
    pub fn decode(self, bytes: &[u8]) -> String {
        match self {
            Encoding::Utf8 => String::from_utf8_lossy(strip_utf8_bom(bytes)).into_owned(),
            Encoding::Utf8Bom => String::from_utf8_lossy(strip_utf8_bom(bytes)).into_owned(),
            Encoding::Utf16Le => decode_utf16(strip_utf16_bom(bytes, &BOM_UTF16_LE), true),
            Encoding::Utf16Be => decode_utf16(strip_utf16_bom(bytes, &BOM_UTF16_BE), false),
            Encoding::Ansi => {
                let (text, _, _) = WINDOWS_1252.decode(bytes);
                text.into_owned()
            }
        }
    }

    /// Encode le texte pour l'enregistrement.
    pub fn encode(self, text: &str) -> Vec<u8> {
        match self {
            Encoding::Utf8 => text.as_bytes().to_vec(),
            Encoding::Utf8Bom => {
                let mut out = Vec::with_capacity(text.len() + BOM_UTF8.len());
                out.extend_from_slice(&BOM_UTF8);
                out.extend_from_slice(text.as_bytes());
                out
            }
            Encoding::Utf16Le => encode_utf16(text, true),
            Encoding::Utf16Be => encode_utf16(text, false),
            Encoding::Ansi => {
                let (bytes, _, _) = WINDOWS_1252.encode(text);
                bytes.into_owned()
            }
        }
    }

    /// Retourne vrai quand l'encodage ANSI remplace au moins un caractère.
    pub fn loses_information(self, text: &str) -> bool {
        match self {
            Encoding::Ansi => WINDOWS_1252.encode(text).2,
            _ => false,
        }
    }
}

fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&BOM_UTF8).unwrap_or(bytes)
}

fn strip_utf16_bom<'a>(bytes: &'a [u8], bom: &[u8]) -> &'a [u8] {
    bytes.strip_prefix(bom).unwrap_or(bytes)
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> String {
    let mut units = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        units.push(if little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        });
    }
    String::from_utf16_lossy(&units)
}

fn encode_utf16(text: &str, little_endian: bool) -> Vec<u8> {
    let bom: [u8; 2] = if little_endian {
        BOM_UTF16_LE
    } else {
        BOM_UTF16_BE
    };
    let mut out = Vec::with_capacity(text.len() * 2 + 2);
    out.extend_from_slice(&bom);
    for unit in text.encode_utf16() {
        let bytes = if little_endian {
            unit.to_le_bytes()
        } else {
            unit.to_be_bytes()
        };
        out.extend_from_slice(&bytes);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_boms() {
        assert_eq!(Encoding::detect(&[0xEF, 0xBB, 0xBF, b'a']), Encoding::Utf8Bom);
        assert_eq!(Encoding::detect(&[0xFF, 0xFE, b'a', 0]), Encoding::Utf16Le);
        assert_eq!(Encoding::detect(&[0xFE, 0xFF, 0, b'a']), Encoding::Utf16Be);
    }

    #[test]
    fn detects_utf8_without_bom() {
        assert_eq!(Encoding::detect("caf\u{e9}".as_bytes()), Encoding::Utf8);
    }

    #[test]
    fn detects_ansi_for_latin1_bytes() {
        // 0xE9 seul n'est pas de l'UTF-8 valide : Notepad affiche ANSI.
        assert_eq!(Encoding::detect(&[b'c', b'a', b'f', 0xE9]), Encoding::Ansi);
    }

    #[test]
    fn utf16_roundtrip_keeps_text() {
        for encoding in [Encoding::Utf16Le, Encoding::Utf16Be] {
            let bytes = encoding.encode("Bonjour\r\nmonde \u{1F600}");
            assert_eq!(encoding.decode(&bytes), "Bonjour\r\nmonde \u{1F600}");
        }
    }

    #[test]
    fn ansi_roundtrip_keeps_accents() {
        let bytes = Encoding::Ansi.encode("caf\u{e9} cr\u{e8}me");
        assert_eq!(bytes, vec![b'c', b'a', b'f', 0xE9, b' ', b'c', b'r', 0xE8, b'm', b'e']);
        assert_eq!(Encoding::Ansi.decode(&bytes), "caf\u{e9} cr\u{e8}me");
    }

    #[test]
    fn detects_ansi_loss() {
        assert!(!Encoding::Ansi.loses_information("café"));
        assert!(Encoding::Ansi.loses_information("東京"));
    }

    #[test]
    fn utf8_bom_roundtrip() {
        let bytes = Encoding::Utf8Bom.encode("texte");
        assert_eq!(&bytes[..3], &BOM_UTF8);
        assert_eq!(Encoding::Utf8Bom.decode(&bytes), "texte");
        assert_eq!(Encoding::Utf8.decode(&bytes), "texte");
    }
}
