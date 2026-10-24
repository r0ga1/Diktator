# Silbenlot 0.12.0 · Beta

Lokales Diktieren für Windows mit Parakeet TDT 0.6B v3. Rust übernimmt Aufnahme,
Erkennung und Texteingabe; Go und WebView2 zeigen die lokale HTML/CSS/JavaScript-
Oberfläche. Kein Konto und keine Cloud-Spracherkennung. NVIDIA/VoiceInk/Microsoft
sind keine Herausgeber oder Unterstützer dieser App.

**Beta:** Der Kern und Installer werden automatisiert und unter Wine geprüft.
Die neue Oberfläche auf echter Windows-Hardware sowie ARM64-Ausführung sind noch
nicht abschließend abgenommen. Keine Zusage fehlerfreier Erkennung oder eines
Gleichstands mit ChatGPT/Gemini. Grenzen und Herkunft: [RELEASE-REVIEW.md](RELEASE-REVIEW.md).

## Installieren

Aus den [GitHub-Releases](https://github.com/r0ga1/silbenlot/releases) genau **eine**
Datei laden:

- **Silbenlot-Setup-x64.exe** für Windows 10/11 mit Intel-/AMD-Prozessor (64 Bit).
- **Silbenlot-Setup-ARM64.exe** für Windows auf ARM.

Setup starten, die separat mitgelieferten Microsoft-Bedingungen lesen und den
Installationsassistenten abschließen. Standardordner:
`%LOCALAPPDATA%\Programs\Silbenlot`. Keine Administratorrechte, zusätzlichen
Downloads, Python-/Rust-/Go-Installation oder separate Modellinstallation nötig.
Parakeet, Silero-Sprachfilter, native Inferenzbibliotheken und die vollständige
WebView2 Fixed Runtime sind enthalten. Etwa 3 GB freien Speicher bereithalten;
für ein Update vorübergehend zusätzlich Platz für das Setup.

Danach **Silbenlot im Startmenü** öffnen, nicht jedes Mal das Setup ausführen.
Setup und App besitzen noch keine Windows-Authenticode-Signatur. Die separate
Silbenlot-Update-Signatur ersetzt kein Windows-Code-Signing-Zertifikat.
Bei einem Startproblem: `%LOCALAPPDATA%\Silbenlot\ui-start.log`.
Erneutes Ausführen des Setups repariert Programmdateien.

## Diktieren und Einstellungen

1. Zieltextfeld anklicken und **Strg+Alt+Leertaste** drücken.
2. Sprechen: standardmäßig erscheint Text bereits während der Aufnahme.
3. Dasselbe Kürzel stoppt die Aufnahme und setzt den abschließenden Text ein.

Unter **Einstellungen → Allgemein → Texteingabe** stehen Live-Eingabe, einmaliges
Einfügen nach dem Stoppen und ausschließlich Zwischenablage zur Wahl. Zusätzliches
Kopieren ist separat schaltbar. Eigene Tastatureingaben/Klicks pausieren die
automatische Eingabe für dieses Diktat; spezielle oder erhöhte Programme können
Eingaben blockieren. Der Text bleibt im Verlauf.

Das kleine Aufnahmefenster lässt sich direkt unter **Allgemein → Aufnahme**
abschalten. **Aufnahme & Audio** enthält Mikrofonwahl, einen Pegeltest ohne
Verlaufseintrag, das Tastenkürzel, getrennte Start-/Stopplautstärken und den
Windows-Berechtigungslink. Die Töne sind eigene Synthesen, keine F1-Aufnahmen.

**X** schließt standardmäßig in den Infobereich neben der Uhr. Rechtsklick auf
das Symbol bietet Öffnen und Beenden. Der Hintergrundmodus und Autostart sind
konfigurierbar. Dashboard, Datei-Transkription, Verlauf mit Kopieren-Schaltfläche
und Wörterbuch sind enthalten; keine Werbung, Modellverwaltung oder Enhancements.

Parakeet v3 erkennt 25 Sprachen insgesamt einschließlich Englisch automatisch.
Diese Schnittstelle hat keine feste Deutsch-Vorgabe. Das Modell arbeitet nicht
nativ im Streamingbetrieb: kurze Abschnitte werden wiederholt erkannt. Wortwahl,
Latenz und mehrdeutige einzelne Wörter hängen auch von Aufnahme und PC ab.
Ein INT8-Encoder mit FP32-Decoder/Joiner und ein konservativer Silero-Sprachfilter
werden verwendet. Keine automatische LLM-Umschreibung des Diktats.

## Leerlauf und persönliche Daten

Das Mikrofon ist im Leerlauf geschlossen, das Modell wird erst bei Bedarf geladen
und standardmäßig nach 15 Sekunden ohne Arbeit entladen. Im Infobereich endet
der zusätzliche Oberflächenprozess. „Modell bereithalten“ verbessert den nächsten
Start und braucht dafür mehr RAM.

Verlauf, Wörterbuch und Einstellungen liegen unverschlüsselt unter
`%LOCALAPPDATA%\Silbenlot`. Audio wird nur bei aktivierter Aufbewahrung gespeichert.
Bei der ersten Nutzung werden vorhandene Diktator-Daten, ersatzweise ältere
VoiceInkLocal-Daten, kopiert; der alte Ordner bleibt erhalten. Die alte App vor
dem Umstieg beenden, damit Kürzel nicht doppelt belegt sind. Updates und
Deinstallation erhalten persönliche Daten. Details: [PRIVACY.md](PRIVACY.md).

## Automatische Updates

Standardmäßig an, abschaltbar unter **Einstellungen → Updates**. Die Quelle ist
fest `r0ga1/silbenlot`, aktuell der **Beta-Kanal** einschließlich Vorabversionen.
Kein Repository-Feld und keine vom Nutzer einzufügende URL. Eine neuere numerische
Version wird nach Start und anschließend alle sechs Stunden gesucht; automatische
Prüfungen werden während aktiver Verarbeitung verschoben.

Das Update benötigt eine RSA-PSS-signierte `update.json`, passende Architektur,
Dateigröße und SHA-256 sowie ein zugehöriges Quellpaket im selben Release. Keine
Installation unsignierter Pakete. Ein geprüftes Setup wird beim **vollständigen
Beenden** nach Ende des App-Prozesses ausgeführt, nicht schon beim Schließen in
den Infobereich. Geänderte Microsoft-Runtime-Bedingungen erfordern ein manuell
geprüftes Setup. Ein Quellcode-Push allein ist kein App-Update.

Updates kontaktieren GitHub; auch Microsofts Runtime hat eigene Datenschutz-
bedingungen. „Lokale Erkennung“ bedeutet keine pauschale Garantie ohne sämtliche
Netzwerkverbindungen. [SECURITY.md](SECURITY.md) beschreibt das Vertrauensmodell.

## Lizenz und Herkunft

Originaler Anwendungscode: **GPL-3.0-only** mit den begrenzten zusätzlichen
Erlaubnissen in [ADDITIONAL_PERMISSION.md](ADDITIONAL_PERMISSION.md). Fremdcode,
Modelle, Schrift und Microsoft-Runtime behalten ihre eigenen Bedingungen.
[THIRD_PARTY.md](THIRD_PARTY.md), [NOTICE](NOTICE), [PROVENANCE.md](PROVENANCE.md)
und `licenses/` müssen erhalten bleiben. Das vollständige Quellpaket der
veröffentlichten Version liegt als **Silbenlot-Source.zip im selben Release**.
Es enthält vendorte Rust-/Go-Quellen sowie die primären nativen Quellarchive;
die großen Archive liegen nicht im Git-Verlauf.

Der Name und das abstrakte Symbol wurden von der früheren Diktator-Version
abgegrenzt. Eine erste Namenssuche ist keine vollständige Markenfreigabe. Dies
ist eine technische Bestandsaufnahme, keine anwaltliche Freigabe.

## Entwickeln und Releases bauen

Getesteter Cross-Build: macOS, Rust 1.99.1, Go 1.26.1, Python 3.12+, LLVM-MinGW
20261020. Abhängigkeiten liegen unter `vendor/` und `shell/vendor/`.

```sh
rustup target add x86_64-pc-windows-gnullvm aarch64-pc-windows-gnullvm
python3 tools/fetch_native_sources.py
python3 build.py --cache /path/to/cache --out /path/to/dist --toolchain /path/to/llvm-mingw --layout-only
python3 installer/build_installer.py --cache /path/to/cache --out /path/to/dist
```

NSIS wird auf dem Buildsystem benötigt; alternativ unterstützt der zweite Schritt
`--docker-container NAME` mit einem Linux-Container und `/workspace`-Einbindung.
Build-Downloads sind mit Version und SHA-256 fixiert. `bsdtar` entpackt die
Microsoft-CABs. Fixed Runtime 156.0.4314.42 wird als App-Dateien mitgeliefert und
muss durch neue, geprüfte App-Releases gepflegt werden.

Prüfungen: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test` unter Windows; für echte Modelltests zusätzlich `DIKTATOR_TEST_ROOT`
setzen. `cd shell && go test -mod=vendor -race -timeout 40s ./...` und
`node shell/startup_test.cjs` prüfen Pipe-Verhalten und Dashboard-Bootstrap.
Der Speech-Vergleich liegt unter `tools/evaluate_speech.py` und `tests/quality/`;
der ausgewählte kleine Testsatz ist kein unabhängiger Produktvergleich.

Releases signiert der Maintainer lokal mit `tools/publish_release.py`; Ablauf und
Schlüsselverwaltung stehen in [RELEASING.md](RELEASING.md). Der private Schlüssel
gehört niemals ins Repository, Quellpaket oder App-Paket. Forks benötigen eine
eigene Updatequelle und einen eigenen Schlüssel.
