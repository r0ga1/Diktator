# Security and release trust

Please report vulnerabilities privately through GitHub Private Vulnerability
Reporting when available. Do not include private audio/transcripts in public
issues. This beta has not received an independent security audit or native
Windows/ARM64 hardware acceptance.

Updates are bound to the repository, channel and RSA-3072 public key in
release-config.json. The application verifies RSA-PSS/SHA-256 (32-byte salt)
using Windows CNG, then validates metadata, architecture, size and package SHA-256.
The signed metadata and payload are checked again by a helper after the old
process exits. There is no unsigned/checksum-only fallback. A repository or CDN
compromise alone should not authorize a package without the private signing key.
This does not protect a PC already compromised with the user's own privileges.

The signing key is not in Git or the source release and is not uploaded to
GitHub. Maintainers must back it up securely. Replacing the publisher key requires
a separate, reviewed migration; silently generating a new key breaks update trust.
GPL users may modify/rebuild/install the app manually or establish their own
publisher identity. No publisher private key is required to exercise GPL rights.

A change to the Microsoft Runtime terms' digest blocks automatic installation
by an older client. Users must then review the new setup terms manually.
The fixed WebView2 runtime is version-pinned and must be maintained with app
releases. A default-enabled update switch does not by itself guarantee fixes
will be published or that every client stays current.
