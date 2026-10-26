//! Persistance de session et liste des fichiers recents.
//!
//! Le Bloc-notes stocke son etat dans le registre (`Software\Microsoft\Notepad`)
//! et autosauvegarde les documents non enregistres sous
//! `%LOCALAPPDATA%\Microsoft\Notepad`. Ici tout vit dans un unique fichier
//! JSON : `%APPDATA%\notepad-rs\session.json`.
//!
//! Le fichier contient :
//! - les onglets ouverts (chemin, et texte autosauvegarde si le document est
//!   modifie), leur codage, fin de ligne, retour a la ligne, zoom et curseur ;
//! - les preferences d'affichage (theme, police, barre d'etat, taille de
//!   fenetre) ;
//! - la liste des fichiers recents (10 maximum, le plus recent en tete).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::doc::Eol;
use crate::encoding::Encoding;
use crate::page::PageSetup;

/// Nombre maximal de fichiers recents, comme le Bloc-notes.
pub const RECENT_LIMIT: usize = 10;

/// Etat complet d'une fenetre, sauvegarde et restaure au demarrage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub tabs: Vec<SessionTab>,
    pub active: usize,
    /// 0 = systeme, 1 = clair, 2 = sombre.
    pub theme: u8,
    pub font: Option<String>,
    pub font_size: f32,
    pub wrap: bool,
    pub status_bar: bool,
    /// Taille interieure de la fenetre en points logiques.
    pub window: Option<[f32; 2]>,
    /// Chemins complets, le plus recent en premier.
    pub recent: Vec<String>,
    /// Mise en page (papier, orientation, marges).
    pub page: PageSetup,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            tabs: Vec::new(),
            active: 0,
            theme: 0,
            font: None,
            font_size: 15.0,
            wrap: true,
            status_bar: true,
            window: None,
            recent: Vec::new(),
            page: PageSetup::default(),
        }
    }
}

/// Un onglet tel qu'il est persiste.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionTab {
    pub path: Option<String>,
    /// Texte autosauvegarde : present uniquement pour un document modifie
    /// (ou sans nom). `None` signifie « relire le fichier sur disque ».
    pub text: Option<String>,
    /// 0 = ANSI, 1 = UTF-8, 2 = UTF-8 BOM, 3 = UTF-16 LE, 4 = UTF-16 BE.
    pub encoding: u8,
    /// 0 = CRLF, 1 = LF, 2 = CR.
    pub eol: u8,
    pub wrap: bool,
    pub font_size: f32,
    pub cursor: usize,
    pub dirty: bool,
    /// Mode Markdown de l'onglet.
    pub markdown: bool,
    pub moodboard: bool,
    pub mood_items: Vec<MoodItem>,
    /// Identifiant stable de page et identifiant de la page parente.
    #[serde(default)]
    pub page_id: u64,
    #[serde(default)]
    pub parent_id: Option<u64>,
    #[serde(default)]
    pub page_title: String,
    pub workspace: PageOptions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PageOptions {
    pub favorite: bool,
    pub pinned: bool,
    pub trashed: bool,
    pub icon: String,
    pub color: [u8; 3],
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MoodItem {
    pub text: String,
    pub image_path: Option<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: u8,
    #[serde(default)]
    pub link: Option<String>,
}

impl Default for MoodItem {
    fn default() -> Self {
        Self {
            text: "Note".into(),
            image_path: None,
            x: 80.0,
            y: 80.0,
            width: 190.0,
            height: 130.0,
            color: 0,
            link: None,
        }
    }
}

pub fn encoding_code(encoding: Encoding) -> u8 {
    match encoding {
        Encoding::Ansi => 0,
        Encoding::Utf8 => 1,
        Encoding::Utf8Bom => 2,
        Encoding::Utf16Le => 3,
        Encoding::Utf16Be => 4,
    }
}

pub fn encoding_from_code(code: u8) -> Encoding {
    match code {
        0 => Encoding::Ansi,
        2 => Encoding::Utf8Bom,
        3 => Encoding::Utf16Le,
        4 => Encoding::Utf16Be,
        _ => Encoding::Utf8,
    }
}

pub fn eol_code(eol: Eol) -> u8 {
    match eol {
        Eol::Crlf => 0,
        Eol::Lf => 1,
        Eol::Cr => 2,
    }
}

pub fn eol_from_code(code: u8) -> Eol {
    match code {
        1 => Eol::Lf,
        2 => Eol::Cr,
        _ => Eol::Crlf,
    }
}

/// `%APPDATA%\notepad-rs`.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("AppData\\Roaming"))
        })
        .unwrap_or_else(std::env::temp_dir);
    base.join("notepad-rs")
}

pub fn session_path() -> PathBuf {
    data_dir().join("session.json")
}

/// Chemin isolé d'une fenêtre lancée avec `--new-window`.
pub fn window_session_path(process_id: u32) -> PathBuf {
    data_dir().join(format!("session-window-{process_id}.json"))
}

impl Session {
    pub fn load_from(path: &Path) -> Session {
        let Ok(bytes) = std::fs::read(path) else {
            return Session::default();
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, json)?;
        if path.exists() {
            std::fs::copy(path, path.with_extension("json.bak"))?;
        }
        // `rename` remplace le fichier existant sous Windows depuis Rust 1.26.
        std::fs::rename(&temp, &path)
    }
}

/// Place `path` en tete des recents (dedoublonnage insensible a la casse,
/// 10 entrees maximum).
pub fn touch_recent_list(list: &mut Vec<String>, path: &Path) {
    let text = path.to_string_lossy().to_string();
    list.retain(|entry| !entry.eq_ignore_ascii_case(&text));
    list.insert(0, text);
    list.truncate(RECENT_LIMIT);
}

pub fn clear_recent_list(list: &mut Vec<String>) {
    list.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Session {
        let mut session = Session {
            theme: 2,
            font: Some("Consolas".into()),
            font_size: 18.0,
            wrap: false,
            ..Session::default()
        };
        session.tabs.push(SessionTab {
            path: Some("C:\\temp\\note.txt".into()),
            text: None,
            encoding: encoding_code(Encoding::Utf16Le),
            eol: eol_code(Eol::Lf),
            wrap: true,
            font_size: 16.0,
            cursor: 42,
            dirty: false,
            markdown: true,
            moodboard: false,
            mood_items: Vec::new(),
            page_id: 1,
            parent_id: None,
            page_title: String::new(),
            workspace: Default::default(),
        });
        session.tabs.push(SessionTab {
            path: None,
            text: Some("brouillon".into()),
            page_id: 2,
            parent_id: None,
            page_title: String::new(),
            workspace: Default::default(),
            encoding: encoding_code(Encoding::Utf8Bom),
            eol: eol_code(Eol::Cr),
            wrap: false,
            font_size: 15.0,
            cursor: 9,
            dirty: true,
            markdown: false,
            moodboard: false,
            mood_items: Vec::new(),
        });
        session
    }

    #[test]
    fn roundtrips_through_json() {
        let session = sample();
        let json = serde_json::to_vec(&session).unwrap();
        let back: Session = serde_json::from_slice(&json).unwrap();
        assert_eq!(back.tabs.len(), 2);
        assert_eq!(back.theme, 2);
        assert_eq!(back.font.as_deref(), Some("Consolas"));
        assert_eq!(back.tabs[0].encoding, 3);
        assert_eq!(back.tabs[0].cursor, 42);
        assert_eq!(back.tabs[1].text.as_deref(), Some("brouillon"));
        assert!(back.tabs[1].dirty);
    }

    #[test]
    fn tolerates_unknown_and_missing_fields() {
        let json = br#"{"tabs": [{"path": "C:\\a.txt"}], "theme": 1, "inconnu": true}"#;
        let session: Session = serde_json::from_slice(json).unwrap();
        assert_eq!(session.tabs.len(), 1);
        assert_eq!(session.tabs[0].encoding, 0);
        assert_eq!(session.tabs[0].font_size, 0.0);
        assert_eq!(session.theme, 1);
        assert_eq!(session.recent.len(), 0);
    }

    #[test]
    fn recent_list_is_deduplicated_and_capped() {
        let mut session = Session::default();
        touch_recent_list(&mut session.recent, Path::new("C:\\a.txt"));
        touch_recent_list(&mut session.recent, Path::new("C:\\b.txt"));
        touch_recent_list(&mut session.recent, Path::new("c:\\A.TXT"));
        assert_eq!(session.recent, vec!["c:\\A.TXT", "C:\\b.txt"]);
        for index in 0..20 {
            let path = format!("C:\\f{index}.txt");
            touch_recent_list(&mut session.recent, Path::new(&path));
        }
        assert_eq!(session.recent.len(), RECENT_LIMIT);
        assert_eq!(session.recent[0], "C:\\f19.txt");
        clear_recent_list(&mut session.recent);
        assert!(session.recent.is_empty());
    }

    #[test]
    fn encoding_and_eol_codes_roundtrip() {
        for encoding in crate::encoding::ALL {
            assert_eq!(encoding_from_code(encoding_code(encoding)), encoding);
        }
        for eol in crate::doc::EOL_ALL {
            assert_eq!(eol_from_code(eol_code(eol)), eol);
        }
        assert_eq!(encoding_from_code(200), Encoding::Utf8);
        assert_eq!(eol_from_code(200), Eol::Crlf);
    }

    #[test]
    fn data_dir_uses_appdata() {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            assert!(data_dir().starts_with(PathBuf::from(appdata)));
        }
        assert!(session_path().ends_with("session.json"));
    }
}
