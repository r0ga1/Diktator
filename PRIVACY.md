# Datenschutz und Netzwerkverbindungen

Diktator führt Spracherkennung mit Parakeet v3 lokal auf dem Windows-PC aus.
Diktate werden nicht an einen Cloud-Spracherkennungsdienst oder an die
WebView2-Oberfläche übergeben. Keine Anmeldung, Werbung oder eigene Telemetrie.

## Lokale Daten

Einstellungen, Wörterbuch und Verlauf liegen unverschlüsselt unter
`%LOCALAPPDATA%\Diktator\data.json`. Auf Wunsch werden dort auch WAV-Aufnahmen
aufbewahrt; standardmäßig ist das aus. Mikrofontests erzeugen keine Historie.
Andere Prozesse mit denselben Benutzerrechten können auf diese Dateien zugreifen.
Die Deinstallation entfernt Programmdateien; persönliche Daten bleiben erhalten.
Bei der ersten Nutzung kann ein vorhandener Diktator-/VoiceInkLocal-Datenbestand
kopiert werden. Das Original wird nicht gelöscht.

## Automatische Updates

Bei frischen Einstellungen sind Updates eingeschaltet. Die App fragt frühestens
zehn Sekunden nach dem Start und dann ungefähr alle sechs Stunden den festen
GitHub-Kanal `r0ga1/Diktator` ab. Während eines Diktats wird eine automatische
Prüfung zurückgestellt. Dabei erfährt GitHub übliche Verbindungsdaten wie IP-Adresse
und den App-User-Agent, jedoch keine Diktate, Audiodateien oder Wörterbücher.
Update-Metadaten und Pakete werden von GitHub/GitHub-CDNs heruntergeladen;
Herausgebersignatur, Architektur, Version, Größe und SHA-256 werden überprüft.
Die Option kann in den Einstellungen abgeschaltet werden. Ein manueller Klick
auf „Jetzt prüfen“ löst unabhängig davon eine Prüfung aus.
GitHub-Datenschutz: https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement

## Microsoft WebView2

Die Oberfläche benötigt die separat lizenzierte Microsoft Fixed Runtime.
Microsoft beschreibt mögliche Diagnosedaten, SmartScreen und weitere Funktionen
in seiner Datenschutzerklärung und dem Edge Privacy Whitepaper. Die App setzt
Flags gegen Hintergrundverbindungen; das ist kein Nachweis, dass die Runtime
unter allen Umständen keinerlei Verbindungen aufbaut.

Microsoft Defender SmartScreen ist nach den Microsoft-Bedingungen standardmäßig
aktiviert und kann Informationen des Endnutzers an Microsoft senden. Details:
https://aka.ms/privacy
https://learn.microsoft.com/en-us/microsoft-edge/privacy-whitepaper#smartscreen
https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy

Die Behauptung „Spracherkennung lokal“ gilt für die ASR-Verarbeitung. Sie ist
keine Zusicherung vollständiger Netzwerkfreiheit des Gesamtprogramms.

## Umstieg von Silbenlot

Beim ersten Umstieg übernimmt Diktator einen neueren Silbenlot-Datenstand einmalig.
Ein vorhandener älterer Diktator-Datenstand wird zuvor im Diktator-Datenordner unter
`vor-umbenennung-*` gesichert. Der alte Silbenlot-Datenordner bleibt erhalten.
„Verlauf löschen“ entfernt nur den aktuellen Verlauf samt zugeordnetem Audio;
frühere Datenordner und Migrationssicherungen werden nicht automatisch gelöscht.
