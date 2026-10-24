# Signierte Beta veröffentlichen

1. Versionsnummer in Cargo.toml/Cargo.lock erhöhen. Änderungen und Drittanbieter-
   Versionen prüfen; `tools/collect_licenses.py` bei Abhängigkeitsänderungen nutzen.
2. `tools/fetch_native_sources.py` ausführen. Die großen, hashgebundenen Archive
   gehören ins Release-Quellpaket, nicht in Git.
3. Beide Architekturen und Installer wie im README bauen. Tests ausführen,
   Source.zip mit den gebauten Quellen abgleichen. Native Windows-Abnahme und
   verbleibende Grenzen ehrlich in den Release Notes angeben.
4. Überprüfte Quellen committen, numerischen Tag `vVERSION` setzen und pushen.
5. `Silbenlot-Release-Notes.md` im Ausgabeverzeichnis vorbereiten. Lokal ausführen:

   ```sh
   python3 tools/publish_release.py --assets /path/to/dist --publish-beta
   ```

Das Tool prüft Setup-Metadaten, Quellen, Hashes, Runtime-Bedingungen und Schlüssel-ID,
signiert update.json mit RSA-PSS/SHA-256 (32 Byte Salt) und veröffentlicht beide
Setups, das vollständige Quellpaket, update.json, update.sig und Prüfsummen als
GitHub-Vorabversion. Zunächst wird ein Entwurf mit allen Assets angelegt; nach
Prüfung von Uploadstatus, Größe und vorhandenen GitHub-Digests wird er freigegeben.
Ohne `--publish-beta` werden nur lokale Assets vorbereitet.
Ein normaler Commit/Push veröffentlicht kein binäres Update.

Die feste Quelle und der öffentliche Schlüssel stehen in release-config.json.
Der private Schlüssel liegt ausschließlich lokal unter
`~/.local/share/silbenlot-release/update-signing.pem` (0700-Verzeichnis, 0600-Datei).
Nie hochladen, ins Quellpaket kopieren oder per Chat ausgeben. Ein geschütztes
Offline-Backup ist Aufgabe des Herausgebers. Bei Schlüsselverlust können alte
Apps keinen neuen Schlüssel automatisch akzeptieren; bei Kompromittierung sind
Feed-Stopp und eine gesondert vertrauenswürdig verteilte neue App erforderlich.

Geänderte Microsoft-Runtime-Bedingungen ändern runtimeTermsSha256. Alte Clients
verlangen dann eine manuelle Setup-Prüfung statt stiller Zustimmung. Bei Forks
Repository und Schlüssel ersetzen; keine Updates im Namen dieses Herausgebers.

Die Signatur beweist die Freigabe durch den Schlüsselinhaber, keine Fehlerfreiheit
oder unabhängige rechtliche Prüfung. GitHub-2FA und Kontoschutz bleiben notwendig.
