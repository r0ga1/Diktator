# Signierte Beta veröffentlichen

1. Versionsnummer in Cargo.toml/Cargo.lock erhöhen. Änderungen und Drittanbieter-
   Versionen prüfen; `tools/collect_licenses.py` bei Abhängigkeitsänderungen nutzen.
2. `tools/fetch_native_sources.py` ausführen. Die großen, hashgebundenen Archive
   gehören ins Release-Quellpaket, nicht in Git.
3. Beide Architekturen und Installer wie in [DEVELOPMENT.md](DEVELOPMENT.md) bauen. Tests ausführen,
   Source.zip mit den gebauten Quellen abgleichen. Native Windows-Abnahme und
   verbleibende Grenzen ehrlich in den Release Notes angeben.
4. Überprüfte Quellen committen, numerischen Tag `vVERSION` setzen und pushen.
5. [RELEASE_TEMPLATE.md](RELEASE_TEMPLATE.md) als `Diktator-Release-Notes.md` ins
   Ausgabeverzeichnis kopieren. VERSION, Änderungsliste und Prüfstand ausfüllen;
   die beiden direkten Installer-Links müssen vor den technischen Details stehen.
   Lokal ausführen:

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

6. Nach erfolgreicher Veröffentlichung die Versionsnummer und Download-Links in
   README.md und INSTALLATION.md auf den neuen Tag aktualisieren, committen und
   pushen. Download-Dateinamen und Links gegen die öffentlichen Assets prüfen.
   Den About-Link des Repositories auf die neue Release-Seite setzen:

   ```sh
   gh repo edit r0ga1/Diktator --homepage https://github.com/r0ga1/Diktator/releases/tag/vVERSION
   ```

   Für Betas keinen `/releases/latest`-Link verwenden: GitHub berücksichtigt dort
   nur vollständige Releases. Den Beta-Status nicht für einen Download-Link ändern.
   Die App verwendet bereits die Release-Liste einschließlich Betas.
   Veröffentlichte Tags, Installer, Quellen und signierte Metadaten bleiben unverändert.

Installer gehören in **Releases**. **Packages** ist für Paketregistries gedacht.
Der Quellcode, `update.json`, `update.sig` und Prüfsummen bleiben verfügbar;
Anwender benötigen zur Installation ausschließlich das passende Setup.

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

## Übergang 0.12 → 0.13

0.12 vertraut weiter `r0ga1/silbenlot` und den bisherigen Asset-Namen. Das alte
Repository darf vor diesem Übergang nicht umbenannt oder entfernt werden.
`--legacy-bridge` erstellt zusätzlich ein eigenes signiertes Übergangsmanifest
und Dateialiasse im Unterordner `legacy-0.13.0`, mit unverändertem öffentlichen
Schlüssel und Microsoft-Bedingungen-Hash. Zuerst das neue Release, danach den
Übergang veröffentlichen. Numerischen Tag in beide Repositories pushen.

```sh
python3 tools/publish_release.py --assets /path/to/dist --legacy-bridge --publish-beta
```

Das neue Setup übernimmt die persönlichen Daten einmalig, sichert gegebenenfalls
einen älteren Diktator-Datenstand und entfernt nur unveränderte, vom alten
Installer verzeichnete Programmdateien. Ein kleiner Versionsbeleg im bisherigen
Programmordner bestätigt dem noch laufenden 0.12-Update-Helfer den Umstieg.
Der bisherige persönliche Silbenlot-Datenordner bleibt unangetastet. Die neue App
prüft ausschließlich `r0ga1/Diktator`; künftige Releases brauchen den Übergang nicht.

Nach beiden Veröffentlichungen den Workflow **Windows installer and update** im
neuen Repository starten. Er installiert die originale öffentliche 0.12-EXE auf
einem frischen Windows-Runner und prüft deren echten automatischen WinHTTP-Download,
CNG-Prüfung, Helfer/Installer, Ordner-/Datenübernahme, Dashboard, überwachte ASR,
rechte Strg-Taste und Verlaufslöschung. Nur für eine wegwerfbare CI-Umgebung.
