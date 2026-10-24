use serde::{Deserialize, Serialize};
pub fn repository() -> &'static str {
    &crate::update_trust::config().repository
}
pub fn newer(candidate: &str, current: &str) -> bool {
    fn parts(s: &str) -> Option<[u32; 3]> {
        let p = s
            .strip_prefix('v')
            .unwrap_or(s)
            .split('.')
            .map(str::parse)
            .collect::<Result<Vec<u32>, _>>()
            .ok()?;
        p.try_into().ok()
    }
    matches!((parts(candidate),parts(current)),(Some(a),Some(b)) if a>b)
}
pub fn asset_url(url: &str, repo: &str) -> bool {
    url.starts_with(&format!("https://github.com/{repo}/releases/download/"))
        && !url.contains("..")
        && !url.contains(['\\', '\r', '\n', '?', '#'])
}
pub fn sha(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn package_name(installed: bool, arch: &str) -> &'static str {
    match (installed, arch == "aarch64") {
        (true, true) => "Silbenlot-Setup-ARM64.exe",
        (true, false) => "Silbenlot-Setup-x64.exe",
        (false, true) => "Silbenlot-ARM64.exe",
        (false, false) => "Silbenlot-x64.exe",
    }
}
#[derive(Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}
#[derive(Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub draft: bool,
    pub assets: Vec<Asset>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pending {
    pub path: std::path::PathBuf,
    pub destination: std::path::PathBuf,
    pub sha256: String,
    pub version: String,
    pub repo: String,
    pub manual: bool,
    pub manifest: String,
    pub signature: String,
}
#[cfg(windows)]
mod native {
    use super::*;
    use crate::{
        app::{App, Phase},
        platform::wide,
        update_trust,
    };
    use std::{
        ffi::c_void,
        fs,
        io::Write,
        path::Path,
        ptr::{null, null_mut},
        sync::{atomic::Ordering, Arc},
        thread,
        time::Duration,
    };
    use windows_sys::Win32::{Foundation::*, Networking::WinHttp::*, System::Threading::*};
    struct Internet(*mut c_void);
    impl Drop for Internet {
        fn drop(&mut self) {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }
    fn request(
        url: &str,
        limit: u64,
        mut sink: impl FnMut(&[u8]) -> Result<(), String>,
    ) -> Result<(), String> {
        unsafe {
            let rest = url
                .strip_prefix("https://")
                .ok_or("Nur HTTPS ist erlaubt.")?;
            let (host, path) = rest.split_once('/').ok_or("Ungültige Update-URL.")?;
            if ![
                "api.github.com",
                "github.com",
                "objects.githubusercontent.com",
                "release-assets.githubusercontent.com",
            ]
            .contains(&host)
            {
                return Err("Unbekannter Update-Host.".into());
            }
            let session = Internet(WinHttpOpen(
                wide(&format!("Silbenlot/{}", crate::VERSION)).as_ptr(),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                null(),
                null(),
                0,
            ));
            if session.0.is_null() {
                return Err("Windows-HTTP konnte nicht gestartet werden.".into());
            }
            WinHttpSetTimeouts(session.0, 15000, 15000, 30000, 30000);
            let connect = Internet(WinHttpConnect(session.0, wide(host).as_ptr(), 443, 0));
            if connect.0.is_null() {
                return Err("Update-Verbindung fehlgeschlagen.".into());
            }
            let req = Internet(WinHttpOpenRequest(
                connect.0,
                wide("GET").as_ptr(),
                wide(&format!("/{path}")).as_ptr(),
                null(),
                null(),
                null(),
                WINHTTP_FLAG_SECURE,
            ));
            if req.0.is_null() {
                return Err("Update-Anfrage fehlgeschlagen.".into());
            }
            let headers = wide("Accept: application/vnd.github+json\r\n");
            if WinHttpSendRequest(req.0, headers.as_ptr(), u32::MAX, null(), 0, 0, 0) == 0
                || WinHttpReceiveResponse(req.0, null_mut()) == 0
            {
                return Err(format!("Update-Netzwerkfehler: {}", GetLastError()));
            }
            let mut status = 0u32;
            let mut n = 4u32;
            if WinHttpQueryHeaders(
                req.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                null(),
                (&mut status as *mut u32).cast(),
                &mut n,
                null_mut(),
            ) == 0
                || status != 200
            {
                return Err(format!("Update-Server: HTTP {status}"));
            }
            let mut total = 0;
            let mut buf = [0u8; 65536];
            loop {
                let mut n = 0;
                if WinHttpReadData(req.0, buf.as_mut_ptr().cast(), buf.len() as u32, &mut n) == 0 {
                    return Err("Update-Download unterbrochen.".into());
                }
                if n == 0 {
                    break;
                }
                total += n as u64;
                if total > limit {
                    return Err("Update-Antwort ist zu groß.".into());
                }
                sink(&buf[..n as usize])?
            }
            Ok(())
        }
    }
    fn bytes(url: &str, limit: u64) -> Result<Vec<u8>, String> {
        let mut b = vec![];
        request(url, limit, |v| {
            b.extend_from_slice(v);
            Ok(())
        })?;
        Ok(b)
    }

    fn check(a: &App, manual: bool) -> Result<(), String> {
        if a.update_busy.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        struct Guard<'a>(&'a App);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.update_busy.store(false, Ordering::SeqCst);
            }
        }
        let _guard = Guard(a);
        let cfg = update_trust::config();
        let repo = repository();
        let current = std::env::current_exe().map_err(|e| e.to_string())?;
        if !current
            .parent()
            .is_some_and(|p| p.join(".diktator-installed.json").is_file())
        {
            return Err("Automatische Updates benötigen die installierte Setup-Version.".into());
        }
        if a.snapshot().phase != Phase::Idle && !manual {
            return Ok(());
        }
        a.state.lock().unwrap().update = "Suche nach signierten Updates …".into();
        let releases: Vec<Release> = serde_json::from_slice(&bytes(
            &format!("https://api.github.com/repos/{repo}/releases?per_page=20"),
            4 * 1024 * 1024,
        )?)
        .map_err(|e| e.to_string())?;
        let release = releases
            .into_iter()
            .filter(|r| {
                !r.draft
                    && (cfg.channel == "beta" || !r.prerelease)
                    && newer(&r.tag_name, crate::VERSION)
            })
            .max_by(|a, b| {
                if newer(&a.tag_name, &b.tag_name) {
                    std::cmp::Ordering::Greater
                } else if newer(&b.tag_name, &a.tag_name) {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        let Some(release) = release else {
            a.state.lock().unwrap().update = "Silbenlot ist aktuell.".into();
            return Ok(());
        };
        let find = |name: &str| {
            release
                .assets
                .iter()
                .find(|x| x.name == name && asset_url(&x.browser_download_url, repo))
                .ok_or_else(|| format!("Signierter Release unvollständig: {name}"))
        };
        let manifest = bytes(&find("update.json")?.browser_download_url, 64 * 1024)?;
        let signature = String::from_utf8(bytes(&find("update.sig")?.browser_download_url, 2048)?)
            .map_err(|e| e.to_string())?;
        let signed = update_trust::verified_manifest(&manifest, &signature)?;
        let artifact = update_trust::artifact(&signed, &release.tag_name, std::env::consts::ARCH)?;
        if !newer(&signed.version, crate::VERSION) {
            return Err("Update ist nicht neuer als die installierte Version.".into());
        }
        let asset = find(&artifact.file)?;
        if asset.size != artifact.bytes {
            return Err("Update-Größe stimmt nicht mit der signierten Freigabe überein.".into());
        }
        let source = find(&signed.source.file)?;
        if source.size != signed.source.bytes {
            return Err("Passendes Quellcode-Archiv fehlt.".into());
        }
        let folder = a.dir.join("updates").join(&signed.version);
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let path = folder.join(&artifact.file);
        let partial = folder.join(format!("{}.part", artifact.file));
        let mut output = fs::File::create(&partial).map_err(|e| e.to_string())?;
        let mut received = 0u64;
        let downloaded = request(&asset.browser_download_url, artifact.bytes, |b| {
            if a.quitting.load(Ordering::Relaxed) || (!manual && !a.settings().auto_update) {
                return Err("Update-Download abgebrochen.".into());
            }
            output.write_all(b).map_err(|e| e.to_string())?;
            received += b.len() as u64;
            a.state.lock().unwrap().update = format!(
                "Update {} wird geladen · {} %",
                signed.version,
                received * 100 / artifact.bytes
            );
            Ok(())
        });
        drop(output);
        if let Err(e) = downloaded {
            let _ = fs::remove_file(&partial);
            return Err(e);
        }
        if received != artifact.bytes || crate::bundle::digest_file(&partial)? != artifact.sha256 {
            let _ = fs::remove_file(&partial);
            return Err("Update-Datei ist beschädigt. Es wurde nichts installiert.".into());
        }
        fs::rename(&partial, &path).map_err(|e| e.to_string())?;
        let pending = Pending {
            path,
            destination: current,
            sha256: artifact.sha256,
            version: signed.version,
            repo: repo.into(),
            manual,
            manifest: String::from_utf8(manifest).map_err(|e| e.to_string())?,
            signature,
        };
        let temp = a.dir.join("updates/pending.tmp");
        fs::write(
            &temp,
            serde_json::to_vec(&pending).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(temp, a.dir.join("updates/pending.json")).map_err(|e| e.to_string())?;
        a.state.lock().unwrap().update = format!(
            "Update {} geprüft · wird beim vollständigen Beenden installiert",
            pending.version
        );
        Ok(())
    }
    pub fn check_now(a: Arc<App>) {
        thread::spawn(move || {
            if let Err(e) = check(&a, true) {
                a.state.lock().unwrap().update = e;
            }
        });
    }
    pub fn start(a: Arc<App>) {
        thread::spawn(move || {
            unsafe {
                SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
            }
            for _ in 0..100 {
                if a.quitting.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
            loop {
                if a.quitting.load(Ordering::Relaxed) {
                    return;
                }
                if a.settings().auto_update {
                    if a.snapshot().phase != Phase::Idle {
                        thread::park_timeout(Duration::from_secs(30));
                        continue;
                    }
                    if let Err(e) = check(&a, false) {
                        a.state.lock().unwrap().update = e;
                    }
                }
                thread::park_timeout(Duration::from_secs(21600));
            }
        });
    }
    fn pending(meta: &Path) -> Result<Pending, String> {
        let root = crate::platform::data_dir()
            .join("updates")
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if meta.canonicalize().map_err(|e| e.to_string())? != root.join("pending.json") {
            return Err("Ungültiger Update-Auftrag.".into());
        }
        let raw = fs::read(meta).map_err(|e| e.to_string())?;
        if raw.len() > 128 * 1024 {
            return Err("Update-Auftrag zu groß.".into());
        }
        let p: Pending = serde_json::from_slice(&raw)
            .map_err(|_| "Veralteter oder ungültiger Update-Auftrag.".to_string())?;
        let signed = update_trust::verified_manifest(p.manifest.as_bytes(), &p.signature)?;
        let artifact = update_trust::artifact(&signed, &p.version, std::env::consts::ARCH)?;
        if p.repo != repository()
            || p.sha256 != artifact.sha256
            || p.version != signed.version
            || !newer(&p.version, crate::VERSION)
            || p.path
                .file_name()
                .is_none_or(|n| n != artifact.file.as_str())
            || !p
                .path
                .canonicalize()
                .map_err(|e| e.to_string())?
                .starts_with(&root)
            || p.destination
                .file_name()
                .is_none_or(|n| n != "Silbenlot.exe")
            || !p
                .destination
                .parent()
                .is_some_and(|d| d.join(".diktator-installed.json").is_file())
            || fs::metadata(&p.path).map_err(|e| e.to_string())?.len() != artifact.bytes
            || crate::bundle::digest_file(&p.path)? != p.sha256
        {
            return Err("Bereitliegendes Update ist nicht vertrauenswürdig.".into());
        }
        Ok(p)
    }
    pub fn apply_on_exit(a: &App) {
        let meta = a.dir.join("updates/pending.json");
        if !meta.exists() {
            return;
        }
        let result = (|| {
            let p = pending(&meta)?;
            if !p.manual && !a.settings().auto_update {
                return Ok(());
            }
            let current = std::env::current_exe().map_err(|e| e.to_string())?;
            if p.destination.canonicalize().map_err(|e| e.to_string())?
                != current.canonicalize().map_err(|e| e.to_string())?
            {
                return Err("Update-Ziel wurde verändert.".into());
            }
            let helper = a
                .dir
                .join("updates")
                .join(format!("apply-{}.exe", crate::store::new_id()));
            fs::copy(&current, &helper).map_err(|e| e.to_string())?;
            if crate::bundle::digest_file(&helper)? != crate::bundle::digest_file(&current)? {
                return Err("Update-Helfer beschädigt.".into());
            }
            use std::os::windows::process::CommandExt;
            std::process::Command::new(helper)
                .arg("--apply-update")
                .arg(meta)
                .arg(std::process::id().to_string())
                .creation_flags(0x08000000)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok::<_, String>(())
        })();
        if let Err(e) = result {
            let _ = fs::write(a.dir.join("updates/last-update-error.txt"), e);
        }
    }
    pub fn helper(meta: &Path, pid: u32) -> Result<(), String> {
        let result = (|| {
            pending(meta)?;
            unsafe {
                let parent = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
                if !parent.is_null() {
                    let status = WaitForSingleObject(parent, 60000);
                    CloseHandle(parent);
                    if status != WAIT_OBJECT_0 {
                        return Err("Silbenlot wurde für das Update nicht beendet.".into());
                    }
                } else if GetLastError() != ERROR_INVALID_PARAMETER {
                    return Err("Update konnte das Beenden nicht prüfen.".into());
                }
            }
            // Recheck metadata and payload after the parent exits, before execution.
            let p = pending(meta)?;
            use std::os::windows::process::CommandExt;
            let status = std::process::Command::new(&p.path)
                .arg("/S")
                .arg(format!("/D={}", p.destination.parent().unwrap().display()))
                .creation_flags(0x08000000)
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!("Setup-Update fehlgeschlagen: {status}"));
            }
            let marker = p
                .destination
                .parent()
                .unwrap()
                .join(".diktator-installed.json");
            let installed: serde_json::Value =
                serde_json::from_slice(&fs::read(marker).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if installed["version"] != p.version {
                return Err("Die neue Version wurde nicht bestätigt.".into());
            }
            let _ = fs::remove_file(meta);
            let _ = fs::remove_file(&p.path);
            let _ =
                fs::remove_file(crate::platform::data_dir().join("updates/last-update-error.txt"));
            Ok(())
        })();
        if let Err(ref e) = result {
            let _ = fs::write(
                crate::platform::data_dir().join("updates/last-update-error.txt"),
                e,
            );
        }
        result
    }
}
#[cfg(windows)]
pub use native::{apply_on_exit, check_now, helper, start};
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_and_origins() {
        assert!(newer("v0.13.0", "0.12.0"));
        assert!(!newer("v0.12.0", "0.12.0"));
        assert!(!newer("v0.12.0/../../bad", "0.11.0"));
        assert!(asset_url(
            "https://github.com/r0ga1/silbenlot/releases/download/v0.13.0/Silbenlot-Setup-x64.exe",
            repository()
        ));
        assert!(!asset_url(
            "https://github.com/attacker/project/releases/download/v0.13.0/Silbenlot-Setup-x64.exe",
            repository()
        ));
        assert!(sha(&"a".repeat(64)));
        assert!(!sha("bad"));
    }
    #[test]
    fn setup_asset_names_are_fixed() {
        assert_eq!(package_name(true, "x86_64"), "Silbenlot-Setup-x64.exe");
        assert_eq!(package_name(true, "aarch64"), "Silbenlot-Setup-ARM64.exe");
    }
}
