//! Fixed publisher identity and signed release metadata. No user-editable feed.
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseConfig {
    pub repository: String,
    pub channel: String,
    pub key_id: String,
    pub runtime_terms_sha256: String,
    rsa_modulus_hex: String,
    rsa_exponent_hex: String,
}
pub fn config() -> &'static ReleaseConfig {
    static CONFIG: OnceLock<ReleaseConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        serde_json::from_str(include_str!("../release-config.json")).expect("release configuration")
    })
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Artifact {
    pub arch: String,
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateManifest {
    pub schema: u32,
    pub repository: String,
    pub channel: String,
    pub key_id: String,
    pub version: String,
    pub runtime_terms_sha256: String,
    pub artifacts: Vec<Artifact>,
    pub source: Artifact,
}
pub const MAX_PACKAGE_BYTES: u64 = 1_800_000_000;
pub fn artifact(manifest: &UpdateManifest, tag: &str, arch: &str) -> Result<Artifact, String> {
    let cfg = config();
    if manifest.runtime_terms_sha256 != cfg.runtime_terms_sha256 {
        return Err(
            "Neue Microsoft-Runtime-Bedingungen: Bitte das Setup manuell prüfen und installieren."
                .into(),
        );
    }
    if manifest.schema != 1
        || manifest.repository != cfg.repository
        || manifest.channel != cfg.channel
        || manifest.key_id != cfg.key_id
        || manifest.version != tag.strip_prefix('v').unwrap_or(tag)
        || !crate::update::newer(&manifest.version, "0.0.0")
        || manifest.artifacts.len() != 2
        || manifest.source.file != "Silbenlot-Source.zip"
        || manifest.source.arch != "source"
        || !crate::update::sha(&manifest.source.sha256)
        || manifest.source.bytes == 0
        || manifest.source.bytes > 500_000_000
    {
        return Err("Signierte Update-Metadaten passen nicht zu Silbenlot.".into());
    }
    for expected_arch in ["x86_64", "aarch64"] {
        let matches = manifest
            .artifacts
            .iter()
            .filter(|a| a.arch == expected_arch)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err("Update-Architekturen fehlen oder sind doppelt.".into());
        }
        let a = matches[0];
        if a.file != crate::update::package_name(true, expected_arch)
            || !crate::update::sha(&a.sha256)
            || a.bytes == 0
            || a.bytes > MAX_PACKAGE_BYTES
        {
            return Err("Ungültiges signiertes Update-Paket.".into());
        }
    }
    manifest
        .artifacts
        .iter()
        .find(|a| a.arch == arch)
        .cloned()
        .ok_or_else(|| "Unbekannte Update-Architektur.".into())
}
fn unhex(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Ungültige Signaturkodierung.".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&value[i..i + 2], 16)
                .map_err(|_| "Ungültige Signaturkodierung.".into())
        })
        .collect()
}
#[cfg(windows)]
pub fn verify_signature(body: &[u8], signature_hex: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::ptr::null_mut;
    use windows_sys::Win32::Security::Cryptography::*;
    if body.is_empty() || body.len() > 64 * 1024 || signature_hex.trim().len() != 768 {
        return Err("Update-Signatur fehlt oder hat eine ungültige Größe.".into());
    }
    let signature = unhex(signature_hex.trim())?;
    let cfg = config();
    let modulus = unhex(&cfg.rsa_modulus_hex)?;
    let exponent = unhex(&cfg.rsa_exponent_hex)?;
    if modulus.len() != 384 || exponent != [1, 0, 1] {
        return Err("Ungültiger Herausgeberschlüssel.".into());
    }
    // BCRYPT_RSAKEY_BLOB header is six little-endian ULONGs, then big-endian e,n.
    let mut blob = Vec::new();
    for value in [
        BCRYPT_RSAPUBLIC_MAGIC,
        3072,
        exponent.len() as u32,
        modulus.len() as u32,
        0,
        0,
    ] {
        blob.extend_from_slice(&value.to_le_bytes());
    }
    blob.extend_from_slice(&exponent);
    blob.extend_from_slice(&modulus);
    struct Algorithm(BCRYPT_ALG_HANDLE);
    impl Drop for Algorithm {
        fn drop(&mut self) {
            unsafe {
                BCryptCloseAlgorithmProvider(self.0, 0);
            }
        }
    }
    struct Key(BCRYPT_KEY_HANDLE);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                BCryptDestroyKey(self.0);
            }
        }
    }
    unsafe {
        let mut provider = null_mut();
        if BCryptOpenAlgorithmProvider(&mut provider, BCRYPT_RSA_ALGORITHM, std::ptr::null(), 0) < 0
        {
            return Err("Windows-Signaturprüfung nicht verfügbar.".into());
        }
        let provider = Algorithm(provider);
        let mut key = null_mut();
        if BCryptImportKeyPair(
            provider.0,
            null_mut(),
            BCRYPT_RSAPUBLIC_BLOB,
            &mut key,
            blob.as_ptr(),
            blob.len() as u32,
            0,
        ) < 0
        {
            return Err("Herausgeberschlüssel konnte nicht geladen werden.".into());
        }
        let key = Key(key);
        let hash = Sha256::digest(body);
        let padding = BCRYPT_PSS_PADDING_INFO {
            pszAlgId: BCRYPT_SHA256_ALGORITHM,
            cbSalt: 32,
        };
        if BCryptVerifySignature(
            key.0,
            (&padding as *const BCRYPT_PSS_PADDING_INFO).cast(),
            hash.as_ptr(),
            hash.len() as u32,
            signature.as_ptr(),
            signature.len() as u32,
            BCRYPT_PAD_PSS,
        ) < 0
        {
            return Err("Update-Signatur ungültig. Es wurde nichts installiert.".into());
        }
    }
    Ok(())
}
#[cfg(windows)]
pub fn verified_manifest(body: &[u8], signature: &str) -> Result<UpdateManifest, String> {
    verify_signature(body, signature)?;
    serde_json::from_slice(body).map_err(|_| "Signierte Update-Datei ist ungültig.".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_is_bound_to_publisher_version_arch_and_source() {
        let cfg = config();
        let a = |arch: &str| Artifact {
            arch: arch.into(),
            file: crate::update::package_name(true, arch).into(),
            bytes: 123,
            sha256: "a".repeat(64),
        };
        let mut m = UpdateManifest {
            schema: 1,
            repository: cfg.repository.clone(),
            channel: cfg.channel.clone(),
            key_id: cfg.key_id.clone(),
            version: "0.13.0".into(),
            runtime_terms_sha256: cfg.runtime_terms_sha256.clone(),
            artifacts: vec![a("x86_64"), a("aarch64")],
            source: Artifact {
                arch: "source".into(),
                file: "Silbenlot-Source.zip".into(),
                bytes: 1,
                sha256: "b".repeat(64),
            },
        };
        assert!(artifact(&m, "v0.13.0", "x86_64").is_ok());
        assert!(artifact(&m, "v0.12.0", "x86_64").is_err());
        m.repository = "attacker/project".into();
        assert!(artifact(&m, "v0.13.0", "x86_64").is_err());
        m.repository = cfg.repository.clone();
        m.artifacts[0].file = "../../bad.exe".into();
        assert!(artifact(&m, "v0.13.0", "x86_64").is_err());
        m.artifacts[0] = a("x86_64");
        m.artifacts[0].bytes = MAX_PACKAGE_BYTES + 1;
        assert!(artifact(&m, "v0.13.0", "x86_64").is_err());
    }
    #[test]
    fn strict_signature_encoding() {
        assert!(unhex("zz").is_err());
        assert!(unhex("a").is_err());
        assert_eq!(unhex("00ff").unwrap(), [0, 255]);
    }
    #[cfg(windows)]
    #[test]
    fn real_cng_signature_rejects_tampering_and_missing_signature() {
        let body = include_bytes!("../tests/quality/update-signature-message.txt");
        let signature = include_str!("../tests/quality/update-signature.hex");
        verify_signature(body, signature).unwrap();
        let mut changed = body.to_vec();
        changed[0] ^= 1;
        assert!(verify_signature(&changed, signature).is_err());
        assert!(verify_signature(body, "").is_err());
        let mut changed = signature.trim().as_bytes().to_vec();
        changed[10] = if changed[10] == b'0' { b'1' } else { b'0' };
        assert!(verify_signature(body, std::str::from_utf8(&changed).unwrap()).is_err());
    }
}
