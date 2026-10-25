use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
pub const MAGIC: &[u8; 16] = b"DIKTATOR-BUNDLE1";
pub const FOOTER: u64 = 56;
#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub arch: String,
    pub files: BTreeMap<String, FileInfo>,
}
#[derive(Serialize, Deserialize)]
pub struct FileInfo {
    pub sha256: String,
    pub bytes: u64,
}
pub struct Bundle {
    pub root: PathBuf,
}
pub fn digest_file(path: &Path) -> Result<String, String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n])
    }
    Ok(format!("{:x}", h.finalize()))
}
fn verified(root: &Path, m: &Manifest) -> bool {
    m.files.iter().all(|(name, info)| {
        let p = root.join(name);
        p.metadata()
            .is_ok_and(|v| v.is_file() && v.len() == info.bytes)
            && digest_file(&p).is_ok_and(|v| v == info.sha256)
    })
}
// Reuse files already verified on first extraction when the EXE and every cached
// file still have exactly the recorded metadata. A normal modification falls back
// to full SHA-256 validation/repair; this is a startup cache, not an anti-tamper seal.
#[derive(Serialize, Deserialize, PartialEq)]
struct Stamp {
    bytes: u64,
    seconds: u64,
    nanos: u32,
}
#[derive(Serialize, Deserialize)]
struct Receipt {
    package: String,
    manifest: String,
    executable: Stamp,
    files: BTreeMap<String, Stamp>,
}
fn stamp(path: &Path) -> Result<Stamp, String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err("Ungültige Cache-Datei.".into());
    }
    let t = m
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    Ok(Stamp {
        bytes: m.len(),
        seconds: t.as_secs(),
        nanos: t.subsec_nanos(),
    })
}
fn receipt(
    root: &Path,
    exe: &Path,
    package: &str,
    json: &str,
    m: &Manifest,
) -> Result<Receipt, String> {
    let files = m
        .files
        .keys()
        .map(|name| stamp(&root.join(name)).map(|v| (name.clone(), v)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(Receipt {
        package: package.into(),
        manifest: format!("{:x}", Sha256::digest(json.as_bytes())),
        executable: stamp(exe)?,
        files,
    })
}
fn unchanged(root: &Path, exe: &Path, package: &str, json: &str, m: &Manifest) -> bool {
    let Ok(bytes) = fs::read(root.join(".verified-cache.json")) else {
        return false;
    };
    let Ok(previous) = serde_json::from_slice::<Receipt>(&bytes) else {
        return false;
    };
    receipt(root, exe, package, json, m).is_ok_and(|now| {
        previous.package == now.package
            && previous.manifest == now.manifest
            && previous.executable == now.executable
            && previous.files == now.files
            && now
                .files
                .iter()
                .all(|(name, v)| v.bytes == m.files[name].bytes)
    })
}
fn remember(root: &Path, exe: &Path, package: &str, json: &str, m: &Manifest) {
    if let Ok(r) = receipt(root, exe, package, json, m) {
        if let Ok(bytes) = serde_json::to_vec(&r) {
            let _ = fs::write(root.join(".verified-cache.json"), bytes);
        }
    }
}
fn validate_manifest(manifest: &Manifest) -> Result<usize, String> {
    if manifest.arch != std::env::consts::ARCH || manifest.version != crate::VERSION {
        return Err("Paketarchitektur oder Version passt nicht zur EXE.".into());
    }
    let overall = manifest
        .files
        .values()
        .try_fold(0u64, |sum, v| sum.checked_add(v.bytes))
        .filter(|v| *v <= 2_500_000_000)
        .ok_or("Ungültige Paketgröße.")? as usize;
    if manifest.files.keys().any(|name| {
        name.is_empty()
            || name.contains([':', '\\'])
            || name
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
    }) {
        return Err("Unsicherer Paketpfad.".into());
    }
    Ok(overall)
}
impl Bundle {
    /// A real installation has its complete payload next to the core. It never
    /// extracts another model/runtime copy into the portable cache.
    pub fn installed(exe: &Path) -> Result<Option<Self>, String> {
        let root = exe.parent().ok_or("Programmordner fehlt.")?;
        let marker = root.join(".diktator-installed.json");
        if !marker.try_exists().map_err(|e| e.to_string())? {
            return Ok(None);
        }
        let mut json = String::new();
        fs::File::open(&marker)
            .map_err(|e| e.to_string())?
            .take(2 * 1024 * 1024 + 1)
            .read_to_string(&mut json)
            .map_err(|e| e.to_string())?;
        if json.len() > 2 * 1024 * 1024 {
            return Err("Installationsliste ist zu groß.".into());
        }
        let m: Manifest = serde_json::from_str(&json)
            .map_err(|e| format!("Installationsliste beschädigt: {e}"))?;
        validate_manifest(&m)?;
        for required in [
            "Diktator.exe",
            "model/encoder.int8.onnx",
            "model/decoder.onnx",
            "model/joiner.onnx",
            "model/tokens.txt",
            "model/silero_vad.onnx",
            "runtime/sherpa-onnx-c-api.dll",
            "runtime/onnxruntime.dll",
            "runtime/onnxruntime_providers_shared.dll",
            "ui/Diktator-UI.exe",
            "ui/WebView2Loader.dll",
            "ui/index.html",
            "webview2/msedgewebview2.exe",
            "webview2/msedge.dll",
        ] {
            if !m.files.contains_key(required) {
                return Err(format!(
                    "Installationsliste unvollständig: {required}. Setup erneut ausführen."
                ));
            }
        }
        let hash = format!("{:x}", Sha256::digest(json.as_bytes()));
        if !unchanged(root, exe, &hash, &json, &m) {
            if !verified(root, &m) {
                return Err("Installationsdateien fehlen oder sind beschädigt. Bitte das Diktator-Setup erneut ausführen.".into());
            }
            remember(root, exe, &hash, &json, &m);
        }
        Ok(Some(Self {
            root: root.to_path_buf(),
        }))
    }

    pub fn open(
        exe: &Path,
        cache: &Path,
        mut progress: impl FnMut(usize, usize) -> bool,
    ) -> Result<Self, String> {
        let mut f = fs::File::open(exe).map_err(|e| e.to_string())?;
        let size = f.metadata().map_err(|e| e.to_string())?.len();
        if size < FOOTER {
            return Err("Eingebettete Programmdateien fehlen.".into());
        }
        f.seek(SeekFrom::End(-(FOOTER as i64)))
            .map_err(|e| e.to_string())?;
        let mut tail = [0u8; 56];
        f.read_exact(&mut tail).map_err(|e| e.to_string())?;
        if &tail[..16] != MAGIC {
            return Err("Diese EXE enthält kein Diktator-Modellpaket. Die vollständige portable EXE verwenden.".into());
        }
        let len = u64::from_le_bytes(tail[16..24].try_into().unwrap());
        if len > size - FOOTER || len > 1_500_000_000 {
            return Err("Ungültiges eingebettetes Paket.".into());
        }
        let start = size - FOOTER - len;
        let package_hash = tail[24..56]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let reader = Section {
            file: f,
            start,
            len,
            pos: 0,
        };
        let mut zip = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;
        let mut manifest_json = String::new();
        zip.by_name("manifest.json")
            .map_err(|e| e.to_string())?
            .take(2 * 1024 * 1024)
            .read_to_string(&mut manifest_json)
            .map_err(|e| e.to_string())?;
        let manifest: Manifest = serde_json::from_str(&manifest_json).map_err(|e| e.to_string())?;
        if manifest.arch != std::env::consts::ARCH || manifest.version != crate::VERSION {
            return Err("Paketarchitektur oder Version passt nicht zur EXE.".into());
        }
        let overall = validate_manifest(&manifest)?;
        let root = cache.join(format!(
            "{}-{}-{}",
            manifest.version,
            manifest.arch,
            &package_hash[..12]
        ));
        if unchanged(&root, exe, &package_hash, &manifest_json, &manifest) {
            return Ok(Self { root });
        }
        fs::create_dir_all(cache).map_err(|e| e.to_string())?;
        if !progress(0, overall) {
            return Err("Start abgebrochen.".into());
        }
        // A changed/missing receipt always verifies the whole compressed bundle.
        let mut f = fs::File::open(exe).map_err(|e| e.to_string())?;
        f.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;
        let mut h = Sha256::new();
        let mut left = len;
        let mut buf = vec![0u8; 1024 * 1024];
        while left > 0 {
            let n = (left as usize).min(buf.len());
            f.read_exact(&mut buf[..n]).map_err(|e| e.to_string())?;
            h.update(&buf[..n]);
            left -= n as u64;
            if !progress(0, overall) {
                return Err("Start abgebrochen.".into());
            }
        }
        if format!("{:x}", h.finalize()) != package_hash {
            return Err("Programmdatei beschädigt: Paket-Prüfsumme stimmt nicht.".into());
        }
        if root.exists() && verified(&root, &manifest) {
            remember(&root, exe, &package_hash, &manifest_json, &manifest);
            return Ok(Self { root });
        }
        let stage = cache.join(format!(".extract-{}", crate::store::new_id()));
        fs::create_dir(&stage).map_err(|e| e.to_string())?;
        let mut received = 0usize;
        let outcome = (|| {
            let mut total = 0u64;
            for i in 0..zip.len() {
                let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
                let name = file.name().to_string();
                if name == "manifest.json" {
                    continue;
                }
                let path = file.enclosed_name().ok_or("Unsicherer Paketpfad.")?;
                if file.is_dir() {
                    continue;
                }
                let info = manifest
                    .files
                    .get(&name)
                    .ok_or("Unbekannte Datei im Paket.")?;
                total += info.bytes;
                if total > 2_500_000_000 || file.size() != info.bytes {
                    return Err("Ungültige Paketgröße.".into());
                }
                let to = stage.join(path);
                if let Some(parent) = to.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?
                }
                let mut dst = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&to)
                    .map_err(|e| e.to_string())?;
                use std::io::Write;
                let mut block = vec![0u8; 1024 * 1024];
                loop {
                    let n = file.read(&mut block).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    dst.write_all(&block[..n]).map_err(|e| e.to_string())?;
                    received += n;
                    if !progress(received, overall) {
                        return Err("Start abgebrochen.".into());
                    }
                }
                drop(dst);
                if digest_file(&to)? != info.sha256 {
                    return Err(format!("Prüfsumme fehlerhaft: {name}"));
                }
                drop(file);
            }
            if !verified(&stage, &manifest) {
                return Err("Paket unvollständig.".into());
            }
            fs::write(stage.join("manifest.json"), &manifest_json).map_err(|e| e.to_string())?;
            if root.exists() {
                fs::remove_dir_all(&root).map_err(|e| e.to_string())?
            }
            fs::rename(&stage, &root).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if outcome.is_err() {
            let _ = fs::remove_dir_all(&stage);
        }
        outcome?;
        remember(&root, exe, &package_hash, &manifest_json, &manifest);
        Ok(Self { root })
    }
}
struct Section {
    file: fs::File,
    start: u64,
    len: u64,
    pos: u64,
}
impl Read for Section {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        let n = b.len().min(self.len.saturating_sub(self.pos) as usize);
        self.file.seek(SeekFrom::Start(self.start + self.pos))?;
        let n = self.file.read(&mut b[..n])?;
        self.pos += n as u64;
        Ok(n)
    }
}
impl Seek for Section {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        let next = match p {
            SeekFrom::Start(n) => n as i128,
            SeekFrom::Current(n) => self.pos as i128 + n as i128,
            SeekFrom::End(n) => self.len as i128 + n as i128,
        };
        if next < 0 || next > self.len as i128 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Paketoffset",
            ));
        }
        self.pos = next as u64;
        Ok(self.pos)
    }
}
