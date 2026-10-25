# Veröffentlichungsprüfung · 0.13.0

Stand: 25.10.2026. Technische Bestandsaufnahme, keine anwaltliche Freigabe.

## Umgesetzt

- Produktname auf Nutzerwunsch wieder **Diktator**. Die bekannte ähnliche
  Diktier-App „Diktator!“ bleibt ein nicht abschließend geklärtes Namensrisiko.
- Vom Nutzer bereitgestelltes Mikrofonlogo, unverändert als PNG übernommen und
  für Windows ins ICO-Format konvertiert. Keine unabhängige Rechtefreigabe.
- Eigene synthetisierte Signaltöne statt Formel-1-Aufnahmen.
- GPLv3-Text, Herkunftsbericht, Drittanbieterhinweise, Modell-/Schrift-/Runtime-
  Lizenzen, vendorte Rust-/Go-Quellen und primäre native Quellarchive und die festgelegten Unterabhängigkeiten samt Originalhinweisen.
- Begrenzte zusätzliche Erlaubnisse gelten ausschließlich für eigenen Code,
  nicht für fremden GPL-Code. Microsofts Software bleibt separat lizenziert.
- Microsoft-Runtime-Bedingungen im interaktiven Setup mit Zustimmungsschritt,
  Datenschutz-/Lizenzansicht in der App und Bedingungen-Hash im Updateprotokoll.
- Feste Updatequelle, lokale Herausgebersignatur, Hash-/Größenprüfung und getrennt
  gestarteter Helfer nach Prozessende. Private Schlüssel außerhalb der Quellen.

## Grenzen der Herkunftsprüfung

Der exakte Dateihashvergleich über größere eigene Anwendungsdateien fand keine
identischen Dateien in den gelieferten VoiceInk-/REA-Referenzbäumen. Das ist kein
Nachweis, dass keine Teilübernahme, Übersetzung oder gestalterische Ableitung
vorliegt. Siehe PROVENANCE.md. Eine vollständige manuelle Urheberrechts- oder
Designprüfung wurde nicht durchgeführt. Kein fremder Code darf über eine neu
hinzugefügte eigene Ausnahme umgelizenziert werden.

Der bekannte gleichartige Name „Diktator!“ wurde bereits dokumentiert. Es liegt keine
vollständige Ähnlichkeitsrecherche bei DPMA/EUIPO/WIPO und keine Markenfreigabe
vor. Eine Veröffentlichung auf GitHub ist ebenfalls eine Weitergabe, kein
urheberrechtlich geschützter Privatbereich und kein pauschaler Haftungsausschluss.

## Technische Abnahme

Die bisherigen Erkennungstests vergleichen ausgewählte öffentliche deutsche und
englische FLEURS-Aufnahmen mit einer Vorversion. Der deutsche Satz enthält nur
männlich markierte Stimmen und wurde zur Entwicklung verwendet. Keine blinde
Abnahme und kein Nachweis gleicher Qualität wie ChatGPT oder Gemini.

Für 0.13 werden Build, Kern-/Signaturtests, Quellpaket und Installer geprüft.
Der zugehörige Release-Prüfbericht nennt tatsächlich ausgeführte Tests. Die neue
Version ist als Beta gekennzeichnet: native Windows-WebView2-Abnahme auf realer
Hardware und ARM64-Ausführung bleiben offen. Wine-Ergebnisse ersetzen das nicht.
Setup/App haben keine Authenticode-Signatur; die eigene Update-Signatur ändert
Windows SmartScreen/Reputationswarnungen nicht.

Die primären nativen Quellarchive sind enthalten, aber ein unabhängiger,
bitidentischer Neubau der fremden DLLs einschließlich aller separat vom jeweiligen
Buildsystem bezogenen Unterabhängigkeiten ist nicht abgenommen. Die begrenzte
zusätzliche Erlaubnis für unmodifizierte native Komponenten ist dokumentiert.

## Rechtlicher Rahmen und verbleibende Pflege

Softwareideen und Grundsätze sind nicht als Programm-Ausdruck geschützt; konkrete
Ausdrucksformen können es sein (§69a UrhG). Je nach Fall kommen zusätzlich Bild-,
Design-, Kennzeichen- und Wettbewerbsrechte in Betracht. GPL ist eine Erlaubnis
unter Bedingungen, keine allgemeine Freigabe fremder Marken oder Fotos.

Verstöße können Unterlassung/Beseitigung und bei Verschulden Schadensersatz
sowie Abmahnkosten auslösen (§§97,97a UrhG). Es gibt keinen pauschal harmlosen
„GitHub-Fehler“ und keinen seriös vorab feststehenden Geldbetrag. Bei konkreten
Ansprüchen oder vor einer breiten kommerziellen Verteilung fachkundig prüfen lassen.

Microsoft erlaubt die Verteilung der WebView2-Runtime unter seinen Bedingungen.
Sie ist kein grundsätzliches Veröffentlichungsverbot; Hinweise, Endnutzer-
bedingungen und Datenschutzanforderungen müssen erfüllt bleiben. Fixed Version
bekommt keine eigenen automatischen Runtime-Updates: der Maintainer muss neue
Versionen prüfen und mit neuen App-Releases ausliefern.

Primärquellen:

- https://www.gesetze-im-internet.de/urhg/__69a.html
- https://www.gesetze-im-internet.de/urhg/__97.html
- https://www.gesetze-im-internet.de/urhg/__97a.html
- https://www.gnu.org/licenses/gpl-3.0.html
- https://www.gnu.org/licenses/gpl-faq.html
- https://www.dpma.de/marken/markenrecherche/
- https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution
- https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy

Die tatsächlich beigelegten Lizenztexte haben Vorrang vor dieser Zusammenfassung.
