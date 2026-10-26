//! Portable workspaces and conservative three-way folder synchronization.
use crate::session::{Session, SessionTab};
use std::{
    collections::HashMap,
    io,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

pub fn uid() -> String {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    format!(
        "{:x}-{:x}-{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    )
}

pub fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ *b as u64).wrapping_mul(0x100000001b3)
    })
}

pub fn portable(mut session: Session) -> io::Result<Session> {
    session.sync_baseline.clear();
    session.sync_directory = None;
    for page in &mut session.tabs {
        for item in &mut page.mood_items {
            if let Some(path) = &item.image_path {
                let metadata = std::fs::metadata(path)?;
                if metadata.len() > 32 * 1024 * 1024 {
                    return Err(io::Error::other("Image trop volumineuse (32 Mo maximum)"));
                }
                item.embedded_image = Some(std::fs::read(path)?);
            }
            if item.embedded_image.is_some() {
                item.image_path = None;
            }
        }
    }
    Ok(session)
}

pub fn materialize(page: &mut SessionTab, dir: &Path) -> io::Result<()> {
    for item in &mut page.mood_items {
        if let Some(bytes) = &item.embedded_image {
            // Decode before writing: an imported attachment must really be an image.
            image::load_from_memory(bytes).map_err(io::Error::other)?;
            std::fs::create_dir_all(dir)?;
            let path = dir.join(format!("{:016x}.image", hash(bytes)));
            if !path.exists() {
                std::fs::write(&path, bytes)?;
            }
            item.image_path = Some(path.to_string_lossy().into_owned());
        }
    }
    Ok(())
}

fn content(page: &SessionTab) -> Vec<u8> {
    let mut page = page.clone();
    page.cursor = 0;
    page.dirty = false;
    page.path = None;
    for item in &mut page.mood_items {
        if item.embedded_image.is_some() {
            item.image_path = None;
        }
    }
    serde_json::to_vec(&page).unwrap_or_default()
}

/// Keep both sides when they differ from the same ancestor. Never infer deletion
/// from absence: deletion is represented by the persisted trash flag.
pub fn merge(local: &Session, remote: &Session, base: &[SessionTab]) -> (Session, usize) {
    let mut merged = local.clone();
    let mut conflicts = 0;
    let mut next = merged.tabs.iter().map(|p| p.page_id).max().unwrap_or(0) + 1;
    let mut remote_ids = HashMap::new();
    for remote_page in &remote.tabs {
        let existing = merged
            .tabs
            .iter()
            .find(|p| p.workspace.uid == remote_page.workspace.uid && !p.workspace.uid.is_empty());
        let id = existing.map(|p| p.page_id).unwrap_or_else(|| {
            let id = next;
            next += 1;
            id
        });
        remote_ids.insert(remote_page.page_id, id);
    }
    for page in &remote.tabs {
        let mut incoming = page.clone();
        incoming.page_id = remote_ids[&page.page_id];
        incoming.parent_id = page.parent_id.and_then(|id| remote_ids.get(&id).copied());
        if incoming.workspace.uid.is_empty() {
            incoming.workspace.uid = uid();
        }
        if let Some(i) = merged
            .tabs
            .iter()
            .position(|p| p.workspace.uid == incoming.workspace.uid)
        {
            // Compare using local IDs so independent session indices don't conflict.
            let baseline = base
                .iter()
                .find(|p| p.workspace.uid == incoming.workspace.uid);
            if content(&merged.tabs[i]) == content(&incoming) {
                continue;
            }
            let local_unchanged = baseline
                .map(|p| content(p) == content(&merged.tabs[i]))
                .unwrap_or(false);
            let remote_unchanged = baseline
                .map(|p| content(p) == content(&incoming))
                .unwrap_or(false);
            if local_unchanged {
                merged.tabs[i] = incoming;
            } else if !remote_unchanged {
                incoming.page_id = next;
                next += 1;
                incoming.workspace.uid = uid();
                incoming.page_title = format!("{} — conflit", incoming.page_title);
                incoming.dirty = true;
                merged.tabs.push(incoming);
                conflicts += 1;
            }
        } else {
            merged.tabs.push(incoming);
        }
    }
    (merged, conflicts)
}

pub fn read(path: &Path) -> io::Result<Session> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > 128 * 1024 * 1024 {
        return Err(io::Error::other(
            "Sauvegarde trop volumineuse (128 Mo maximum)",
        ));
    }
    serde_json::from_slice(&std::fs::read(path)?).map_err(io::Error::other)
}

pub struct FolderLock(std::path::PathBuf);
impl FolderLock {
    pub fn acquire(dir: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(".notepad-rs-sync.lock");
        // create_new is atomic on the shared filesystem. Never delete another
        // client's lock on a timer: it may still be writing a large archive.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        use std::io::Write;
        writeln!(file, "{} {}", std::process::id(), uid())?;
        Ok(Self(path))
    }
}
impl Drop for FolderLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session(text: &str) -> Session {
        let mut s = Session::default();
        let mut p = SessionTab::default();
        p.page_id = 1;
        p.workspace.uid = "same-page".into();
        p.text = Some(text.into());
        s.tabs.push(p);
        s
    }
    #[test]
    fn concurrent_edits_keep_both_versions() {
        let base = session("base");
        let (merged, conflicts) = merge(&session("local"), &session("remote"), &base.tabs);
        assert_eq!(conflicts, 1);
        assert_eq!(merged.tabs.len(), 2);
        assert_eq!(merged.tabs[0].text.as_deref(), Some("local"));
        assert_eq!(merged.tabs[1].text.as_deref(), Some("remote"));
    }
    #[test]
    fn remote_only_edit_updates_without_duplicate() {
        let base = session("base");
        let (merged, conflicts) = merge(&base, &session("remote"), &base.tabs);
        assert_eq!(conflicts, 0);
        assert_eq!(merged.tabs.len(), 1);
        assert_eq!(merged.tabs[0].text.as_deref(), Some("remote"));
    }
    #[test]
    fn repeated_sync_does_not_duplicate_pages() {
        let s = session("same");
        let (merged, conflicts) = merge(&s, &s, &s.tabs);
        assert_eq!(merged.tabs.len(), 1);
        assert_eq!(conflicts, 0);
    }
    #[test]
    fn local_only_edit_is_not_replaced_by_unchanged_remote() {
        let base = session("base");
        let (merged, conflicts) = merge(&session("local"), &base, &base.tabs);
        assert_eq!(conflicts, 0);
        assert_eq!(merged.tabs.len(), 1);
        assert_eq!(merged.tabs[0].text.as_deref(), Some("local"));
    }
    #[test]
    fn portable_image_restores_after_original_file_is_gone() {
        let dir = std::env::temp_dir().join(format!("notepad-rs-assets-{}", uid()));
        std::fs::create_dir_all(&dir).unwrap();
        let original = dir.join("original.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .save(&original)
            .unwrap();
        let mut s = session("image");
        let mut item = crate::session::MoodItem::default();
        item.image_path = Some(original.to_string_lossy().into_owned());
        s.tabs[0].mood_items.push(item);
        let archive = portable(s).unwrap();
        assert!(archive.tabs[0].mood_items[0].image_path.is_none());
        std::fs::remove_file(&original).unwrap();
        let bytes = serde_json::to_vec(&archive).unwrap();
        let mut imported: Session = serde_json::from_slice(&bytes).unwrap();
        materialize(&mut imported.tabs[0], &dir.join("restored")).unwrap();
        let restored =
            std::path::PathBuf::from(imported.tabs[0].mood_items[0].image_path.as_ref().unwrap());
        assert!(restored.exists());
        assert!(image::load_from_memory(&std::fs::read(&restored).unwrap()).is_ok());
        std::fs::remove_file(restored).unwrap();
        std::fs::remove_dir(dir.join("restored")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn shared_folder_excludes_concurrent_writer_and_releases_own_lock() {
        let dir = std::env::temp_dir().join(format!("notepad-rs-lock-{}", uid()));
        let first = FolderLock::acquire(&dir).unwrap();
        assert!(FolderLock::acquire(&dir).is_err());
        drop(first);
        let second = FolderLock::acquire(&dir).unwrap();
        drop(second);
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn invalid_attachment_is_rejected() {
        let mut p = SessionTab::default();
        let mut item = crate::session::MoodItem::default();
        item.embedded_image = Some(b"not an image".to_vec());
        p.mood_items.push(item);
        let dir = std::env::temp_dir().join(format!("notepad-rs-invalid-{}", uid()));
        assert!(materialize(&mut p, &dir).is_err());
        assert!(!dir.exists());
    }
}
