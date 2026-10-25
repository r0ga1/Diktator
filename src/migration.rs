//! One-time product rename. Never delete personal data in the previous folder.
use crate::{
    bundle,
    store::{new_id, safe_id, Store},
};
use std::{fs, path::Path};
/// Prefer a newer Silbenlot store on the first 0.13 start, even if an older
/// Diktator folder exists. Back up that older store before replacing anything.
pub fn load_data(dir: &Path, previous: &Path) -> Result<Store, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let done = dir.join("migration-0.13.json");
    if done.exists() || dir == previous {
        return Store::load(dir, None);
    }
    let incoming = previous.join("data.json");
    let current = dir.join("data.json");
    let newer = incoming.is_file()
        && (!current.is_file()
            || incoming.metadata().and_then(|m| m.modified()).ok()
                > current.metadata().and_then(|m| m.modified()).ok());
    let data = if newer {
        let data: Store = serde_json::from_slice(&fs::read(&incoming).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Bisherige Daten konnten nicht übernommen werden: {e}"))?;
        let backup = dir.join(format!("vor-umbenennung-{}", new_id()));
        if current.exists() {
            fs::create_dir(&backup).map_err(|e| e.to_string())?;
            fs::copy(&current, backup.join("data.json")).map_err(|e| e.to_string())?;
        }
        for e in &data.history {
            if !e.audio || !safe_id(&e.id) {
                continue;
            }
            let name = format!("{}.wav", e.id);
            let from = previous.join(&name);
            let to = dir.join(&name);
            if from.is_file() {
                if to.exists() && backup.exists() {
                    fs::copy(&to, backup.join(&name)).map_err(|e| e.to_string())?;
                }
                fs::copy(from, to).map_err(|e| e.to_string())?;
            }
        }
        data.save(dir)?;
        data
    } else {
        Store::load(dir, Some(previous))?
    };
    fs::write(
        done,
        b"{\"version\":\"0.13.0\",\"previous\":\"Silbenlot\"}\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(data)
}
/// Remove only unchanged files listed by the previous installer. A tiny version
/// receipt remains so 0.12's already-running signed-update helper can confirm
/// installation after the application has moved to the Diktator folder.
pub fn retire_install(previous: &Path, destination: &Path) -> Result<(), String> {
    let old = previous.canonicalize().map_err(|e| e.to_string())?;
    let new = destination.canonicalize().map_err(|e| e.to_string())?;
    if old == new {
        return Err("Alte und neue Installation müssen getrennte Ordner verwenden.".into());
    }
    let marker = old.join(".diktator-installed.json");
    let bytes = fs::read(&marker).map_err(|e| e.to_string())?;
    let receipt: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if receipt["version"] == crate::VERSION
        && receipt["migratedTo"] == new.to_string_lossy().as_ref()
    {
        return Ok(());
    }
    let m: bundle::Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if !m.files.contains_key("Silbenlot.exe") || !crate::update::newer(crate::VERSION, &m.version) {
        return Err("Unbekannte Vorgängerinstallation.".into());
    }
    let mut directories = std::collections::BTreeSet::new();
    for (name, info) in m.files {
        let relative = std::path::Path::new(&name);
        if relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
            || name.contains(['\\', ':'])
        {
            return Err("Unsicherer Vorgängerpfad.".into());
        }
        let path = old.join(relative);
        if let Ok(canonical) = path.canonicalize() {
            if !canonical.starts_with(&old) {
                return Err("Vorgängerpfad verlässt den Installationsordner.".into());
            }
            if bundle::digest_file(&canonical).is_ok_and(|h| h == info.sha256) {
                fs::remove_file(path).map_err(|e| e.to_string())?;
            }
        }
        for p in relative.ancestors().skip(1) {
            if !p.as_os_str().is_empty() {
                directories.insert(p.to_path_buf());
            }
        }
    }
    for name in [
        "Uninstall.exe",
        ".verified-cache.json",
        "webview2/.diktator-appcontainer-rx",
    ] {
        let _ = fs::remove_file(old.join(name));
    }
    let mut dirs = directories.into_iter().collect::<Vec<_>>();
    dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for p in dirs {
        let _ = fs::remove_dir(old.join(p));
    }
    let receipt = serde_json::json!({"version":crate::VERSION,"migratedTo":new.to_string_lossy()});
    fs::write(marker, serde_json::to_vec(&receipt).unwrap()).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newer_data_migrates_once_and_keeps_both_originals() {
        let root = std::env::temp_dir().join(new_id());
        let old = root.join("Silbenlot");
        let new = root.join("Diktator");
        fs::create_dir_all(&old).unwrap();
        fs::create_dir(&new).unwrap();
        let mut previous = Store::default();
        previous.settings.shortcut = crate::store::Shortcut {
            key: 163,
            modifiers: 0,
        };
        previous.history.push(crate::store::Entry {
            id: "audio".into(),
            text: "neuer Verlauf".into(),
            audio: true,
            ..Default::default()
        });
        let mut existing = Store::default();
        existing.history.push(crate::store::Entry {
            text: "älter".into(),
            ..Default::default()
        });
        existing.save(&new).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        previous.save(&old).unwrap();
        fs::write(old.join("audio.wav"), b"saved audio").unwrap();
        let original = fs::read(old.join("data.json")).unwrap();
        let migrated = load_data(&new, &old).unwrap();
        assert_eq!(migrated.history[0].text, "neuer Verlauf");
        assert_eq!(migrated.settings.shortcut.key, 163);
        assert_eq!(fs::read(new.join("audio.wav")).unwrap(), b"saved audio");
        assert_eq!(fs::read(old.join("data.json")).unwrap(), original);
        assert!(fs::read_dir(&new).unwrap().filter_map(Result::ok).any(|e| e
            .file_name()
            .to_string_lossy()
            .starts_with("vor-umbenennung-")));
        let mut changed = migrated;
        changed.history.clear();
        changed.save(&new).unwrap();
        assert!(load_data(&new, &old).unwrap().history.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
