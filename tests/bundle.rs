use diktator::bundle::{Bundle, FileInfo, Manifest, MAGIC};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
    path::PathBuf,
};
fn fixture(file_name: &str, content: &[u8]) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("diktator-bundle-{}", diktator::store::new_id()));
    std::fs::create_dir(&dir).unwrap();
    let exe = dir.join("Silbenlot.exe");
    let mut files = BTreeMap::new();
    files.insert(
        file_name.into(),
        FileInfo {
            sha256: format!("{:x}", Sha256::digest(content)),
            bytes: content.len() as u64,
        },
    );
    let manifest = Manifest {
        version: diktator::VERSION.into(),
        arch: std::env::consts::ARCH.into(),
        files,
    };
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    z.start_file("manifest.json", options).unwrap();
    z.write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    z.start_file(file_name, options).unwrap();
    z.write_all(content).unwrap();
    let payload = z.finish().unwrap().into_inner();
    let mut bytes = b"MZ-test-executable-prefix".to_vec();
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&Sha256::digest(&payload));
    std::fs::write(&exe, bytes).unwrap();
    (dir, exe)
}
#[test]
fn single_exe_extracts_checks_and_repairs_cache() {
    let (dir, exe) = fixture("model/example.onnx", b"verified model");
    let cache = dir.join("cache");
    let b = Bundle::open(&exe, &cache, |_, _| true).unwrap();
    assert_eq!(
        std::fs::read(b.root.join("model/example.onnx")).unwrap(),
        b"verified model"
    );
    std::fs::write(b.root.join("model/example.onnx"), b"corruption").unwrap();
    let b = Bundle::open(&exe, &cache, |_, _| true).unwrap();
    assert_eq!(
        std::fs::read(b.root.join("model/example.onnx")).unwrap(),
        b"verified model"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn corrupted_bundle_and_traversal_rejected() {
    for malicious in [false, true] {
        let (dir, exe) = fixture(
            if malicious {
                "../outside"
            } else {
                "model/file"
            },
            b"data",
        );
        if !malicious {
            let mut b = std::fs::read(&exe).unwrap();
            b[40] ^= 1;
            std::fs::write(&exe, b).unwrap();
        }
        assert!(Bundle::open(&exe, &dir.join("cache"), |_, _| true).is_err());
        assert!(!dir.join("outside").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
#[test]
fn closing_first_start_cancels_cleanly() {
    let (dir, exe) = fixture("model/file", &vec![5u8; 20000]);
    assert!(Bundle::open(&exe, &dir.join("cache"), |_, _| false).is_err());
    assert_eq!(std::fs::read_dir(dir.join("cache")).unwrap().count(), 0);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn verified_warm_start_has_no_preparation_and_detects_changed_files() {
    let (dir, exe) = fixture("model/file", b"first contents");
    let cache = dir.join("cache");
    let b = Bundle::open(&exe, &cache, |_, _| true).unwrap();
    // No preparation callback means no progress window on ordinary subsequent starts.
    Bundle::open(&exe, &cache, |_, _| {
        panic!("warm start must reuse verified cache")
    })
    .unwrap();
    let path = b.root.join("model/file");
    std::fs::write(&path, b"wrong contents").unwrap(); // same length
    let file = std::fs::File::options().write(true).open(&path).unwrap();
    file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(100))
        .unwrap();
    drop(file);
    let mut repairs = 0;
    Bundle::open(&exe, &cache, |_, _| {
        repairs += 1;
        true
    })
    .unwrap();
    assert!(repairs > 0);
    assert_eq!(std::fs::read(path).unwrap(), b"first contents");
    // Removing the receipt triggers a full verification of both EXE and cache.
    std::fs::remove_file(b.root.join(".verified-cache.json")).unwrap();
    let mut bytes = std::fs::read(&exe).unwrap();
    bytes[40] ^= 1;
    std::fs::write(&exe, bytes).unwrap();
    assert!(Bundle::open(&exe, &cache, |_, _| true).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

fn installed_fixture() -> (PathBuf, PathBuf, Manifest) {
    let dir =
        std::env::temp_dir().join(format!("diktator-installed-{}", diktator::store::new_id()));
    std::fs::create_dir(&dir).unwrap();
    let mut files = BTreeMap::new();
    for name in [
        "Silbenlot.exe",
        "model/encoder.int8.onnx",
        "model/decoder.onnx",
        "model/joiner.onnx",
        "model/tokens.txt",
        "model/silero_vad.onnx",
        "runtime/sherpa-onnx-c-api.dll",
        "runtime/onnxruntime.dll",
        "runtime/onnxruntime_providers_shared.dll",
        "ui/Silbenlot-UI.exe",
        "ui/WebView2Loader.dll",
        "ui/index.html",
        "webview2/msedgewebview2.exe",
        "webview2/msedge.dll",
    ] {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"verified").unwrap();
        files.insert(
            name.into(),
            FileInfo {
                bytes: 8,
                sha256: format!("{:x}", Sha256::digest(b"verified")),
            },
        );
    }
    let m = Manifest {
        version: diktator::VERSION.into(),
        arch: std::env::consts::ARCH.into(),
        files,
    };
    let exe = dir.join("Silbenlot.exe");
    (dir, exe, m)
}
#[test]
fn installed_layout_uses_deployed_files_and_detects_corruption() {
    let (dir, exe, m) = installed_fixture();
    assert!(Bundle::installed(&exe).unwrap().is_none());
    std::fs::write(
        dir.join(".diktator-installed.json"),
        serde_json::to_vec(&m).unwrap(),
    )
    .unwrap();
    let b = Bundle::installed(&exe).unwrap().unwrap();
    assert_eq!(b.root, dir);
    assert!(dir.join(".verified-cache.json").is_file());
    assert!(!dir.join("cache").exists());
    assert_eq!(Bundle::installed(&exe).unwrap().unwrap().root, dir);
    let model = dir.join("model/encoder.int8.onnx");
    std::fs::write(&model, b"corrupt!").unwrap();
    let f = std::fs::File::options().write(true).open(&model).unwrap();
    f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(100))
        .unwrap();
    drop(f);
    assert!(Bundle::installed(&exe)
        .err()
        .unwrap()
        .contains("Setup erneut"));
    std::fs::write(&model, b"verified").unwrap();
    assert!(Bundle::installed(&exe).unwrap().is_some());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn installed_layout_rejects_incomplete_wrong_version_and_unsafe_manifest() {
    let (dir, exe, mut m) = installed_fixture();
    let marker = dir.join(".diktator-installed.json");
    m.version = "0.0.0".into();
    std::fs::write(&marker, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(Bundle::installed(&exe).is_err());
    m.version = diktator::VERSION.into();
    m.files.insert(
        "../outside".into(),
        FileInfo {
            bytes: 0,
            sha256: String::new(),
        },
    );
    std::fs::write(&marker, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(Bundle::installed(&exe).is_err());
    m.files.remove("../outside");
    m.files.remove("model/tokens.txt");
    std::fs::write(&marker, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(Bundle::installed(&exe)
        .err()
        .unwrap()
        .contains("unvollständig"));
    assert!(!dir.join("outside").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
