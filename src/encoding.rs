//! Codages de texte reproduits d'apres le comportement observe de Notepad.
//!
//! Notepad expose cinq choix de codage : ANSI, UTF-8, UTF-8 avec BOM,
//! UTF-16 LE et UTF-16 BE. La detection repose sur les BOM puis sur la
//! validite UTF-8 (equivalent pratique du test `IsTextUnicode` importe par
//! `notepad.exe`).

#[cfg(not(windows))]
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
            Encoding::Ansi => ansi_decode(bytes),
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
            Encoding::Ansi => ansi_encode(text).0,
        }
    }

    /// Retourne vrai quand l'encodage ANSI remplace au moins un caractère.
    pub fn loses_information(self, text: &str) -> bool {
        match self {
            Encoding::Ansi => ansi_encode(text).1,
            _ => false,
        }
    }
}

#[cfg(not(windows))]
fn ansi_decode(bytes: &[u8]) -> String {
    WINDOWS_1252.decode(bytes).0.into_owned()
}
#[cfg(not(windows))]
fn ansi_encode(text: &str) -> (Vec<u8>, bool) {
    let (bytes, _, loss) = WINDOWS_1252.encode(text);
    (bytes.into_owned(), loss)
}
#[cfg(windows)]
fn ansi_decode(bytes: &[u8]) -> String {
    ansi_decode_page(
        unsafe { windows_sys::Win32::Globalization::GetACP() },
        bytes,
    )
}
#[cfg(windows)]
fn ansi_decode_page(cp: u32, bytes: &[u8]) -> String {
    use windows_sys::Win32::Globalization::MultiByteToWideChar;
    if bytes.is_empty() {
        return String::new();
    }
    let len = i32::try_from(bytes.len()).expect("Fichier ANSI trop volumineux");
    unsafe {
        let count = MultiByteToWideChar(cp, 0, bytes.as_ptr(), len, std::ptr::null_mut(), 0);
        assert!(count > 0, "Conversion ANSI système impossible");
        let mut wide = vec![0u16; count as usize];
        assert_eq!(
            MultiByteToWideChar(cp, 0, bytes.as_ptr(), len, wide.as_mut_ptr(), count),
            count
        );
        String::from_utf16_lossy(&wide)
    }
}
#[cfg(windows)]
fn ansi_encode(text: &str) -> (Vec<u8>, bool) {
    ansi_encode_page(unsafe { windows_sys::Win32::Globalization::GetACP() }, text)
}
#[cfg(windows)]
fn ansi_encode_page(cp: u32, text: &str) -> (Vec<u8>, bool) {
    use windows_sys::Win32::Globalization::WideCharToMultiByte;
    if text.is_empty() {
        return (Vec::new(), false);
    }
    let wide: Vec<u16> = text.encode_utf16().collect();
    let len = i32::try_from(wide.len()).expect("Texte ANSI trop volumineux");
    unsafe {
        let flags = if cp == 65001 { 0 } else { 0x400 };
        let mut substituted = 0;
        let used = if cp == 65001 {
            std::ptr::null_mut()
        } else {
            &mut substituted
        };
        let count = WideCharToMultiByte(
            cp,
            flags,
            wide.as_ptr(),
            len,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            used,
        );
        assert!(count > 0, "Conversion ANSI système impossible");
        let mut bytes = vec![0u8; count as usize];
        assert_eq!(
            WideCharToMultiByte(
                cp,
                flags,
                wide.as_ptr(),
                len,
                bytes.as_mut_ptr(),
                count,
                std::ptr::null(),
                used
            ),
            count
        );
        let loss = substituted != 0 || ansi_decode_page(cp, &bytes) != text;
        (bytes, loss)
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

    #[cfg(windows)]
    #[test]
    fn system_code_pages_roundtrip_without_best_fit_substitution() {
        for (cp, text) in [
            (1252, "café"),
            (1251, "Привет"),
            (932, "東京"),
            (65001, "café 東京 😀"),
        ] {
            let (bytes, loss) = ansi_encode_page(cp, text);
            assert!(!loss, "page {cp}");
            assert_eq!(ansi_decode_page(cp, &bytes), text);
        }
        let (bytes, loss) = ansi_encode_page(1252, "東京");
        assert!(loss);
        assert_eq!(bytes, b"??");
        assert!(ansi_encode_page(1252, "Γ").1);
    }

    #[test]
    fn detects_boms() {
        assert_eq!(
            Encoding::detect(&[0xEF, 0xBB, 0xBF, b'a']),
            Encoding::Utf8Bom
        );
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
        assert_eq!(Encoding::Ansi.decode(&bytes), "caf\u{e9} cr\u{e8}me");
    }

    #[test]
    fn detects_ansi_loss() {
        for text in ["café", "東京", "Привет", "😀"] {
            assert_eq!(
                Encoding::Ansi.loses_information(text),
                Encoding::Ansi.decode(&Encoding::Ansi.encode(text)) != text
            );
        }
    }

    #[test]
    fn utf8_bom_roundtrip() {
        let bytes = Encoding::Utf8Bom.encode("texte");
        assert_eq!(&bytes[..3], &BOM_UTF8);
        assert_eq!(Encoding::Utf8Bom.decode(&bytes), "texte");
        assert_eq!(Encoding::Utf8.decode(&bytes), "texte");
    }
}
