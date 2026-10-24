# Silbenlot · Handbuch

Installation: [INSTALLATION.md](INSTALLATION.md).

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
