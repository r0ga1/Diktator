# Herkunft und Abgrenzung

Silbenlot ist das umbenannte Windows-Projekt, dessen interne Vorversion 0.11
„Diktator“ hieß. Die öffentliche Produktbezeichnung wurde geändert, weil bereits
eine andere Diktier-App „Diktator!“ existiert. Eine erste Websuche zu „Silbenlot“
lieferte am 24.10.2026 keine offensichtliche gleichnamige Software. Das ist keine
vollständige Ähnlichkeits-/Markenrecherche und keine Zusicherung freier Kennzeichen.

## VoiceInk und bereitgestellte Dateien

VoiceInk 1.74 war eine Funktions- und Gestaltungsreferenz:
https://github.com/Beingpax/VoiceInk/tree/v1.74
Die untersuchte Referenzkopie enthält eine GPLv3-Lizenz und nennt diese Lizenz
in ihrem README. Die aktuelle Windows-Anwendung enthält keinen Swift-Quellbaum,
keine Mach-O-Binärdatei, keine originale VoiceInk-App, keinen kopierten Installer,
keine VoiceInk-Marke als Produktlogo und kein aus der gelieferten modifizierten
DMG entnommenes ausführbares Programm. Die Anwendung nutzt Rust, Go und neu für
dieses Windows-Projekt erstellte HTML/CSS/JavaScript-Dateien.

Ein Hashvergleich der nicht vendorten Anwendungscode-Dateien über 200 Bytes
gegen die bereitgestellten VoiceInk- und REA-Referenzbäume fand keine identischen
Dateien. Das erkennt keine Übersetzungen, Teilübernahmen, ähnliche Gestaltung
oder sonstige rechtlich relevante Ableitungen und ist keine juristische Freigabe.
Die GPL-Lizenzierung wird konservativ beibehalten. Zusätzliche Erlaubnisse gelten
nur für den eigenen Projektcode; sie erteilen keine Rechte an fremdem GPL-Code.
Sollten später bisher nicht dokumentierte Übernahmen erkannt werden, müssen
sie vor weiterer Verteilung zugeordnet und ihre Bedingungen erfüllt werden.

REA wurde als Analyse-/Entwicklungsreferenz bereitgestellt. Dessen beigefügte
Lizenz ist MIT (Copyright 2026 morluto). Es wurden keine identischen größeren
REA-Anwendungscode-Dateien im Silbenlot-Code gefunden; REA wird nicht als Tool
oder Laufzeit mitgeliefert. Künftige tatsächliche Übernahmen müssen dessen
MIT-Copyright- und Lizenzhinweis erhalten.

## Assets

Das neue App-Symbol besteht aus Audiobalken und einer abstrakten Sprechform. Es
wurde mit image_gen aus dem vorherigen generierten Piktogramm weiterentwickelt.
Es enthält kein Stockfoto, keine nachgezogene Foto-Umrandung, keine Frisur,
Brille, Uniform oder beabsichtigte Darstellung einer realen Person/Filmfigur.
Prompt und Herkunft stehen in assets/logo-prompt.txt. Keine Garantie auf
Exklusivrechte oder vollständige Marken-/Designfreiheit.

Die Start-/Stopptöne wurden im Projekt synthetisiert (tools/generate_sounds.py);
es sind keine Formel-1-Aufnahmen. Die Nunito-Schrift behält ihre OFL-Lizenz.
Die einfachen Navigationspfade stammen aus der früheren Implementierung dieses
Windows-Projekts; keine VoiceInk-SF-Symbol-Bibliothek wird extrahiert/mitgeliefert.

## Bibliotheken und Quellen

Rust- und Go-Abhängigkeiten samt ursprünglichen Lizenzen sind vendort.
Native Komponenten und Modell-Dateien sind in THIRD_PARTY.md beschrieben.
Die primären sherpa-onnx-/ONNX-Runtime-Quellarchive sind im Release-Quellpaket
unter third_party/source-archives mit Revision, Herkunft und SHA-256 enthalten.
Microsoft WebView2 bleibt eine separat lizenzierte unmodifizierte Runtime und
ist nicht als GPL-Quellcode ausgewiesen.

Diese Bestandsaufnahme ist eine technische Herkunftsdokumentation mit den
angegebenen Grenzen. Sie ist keine anwaltliche Prüfung und keine Zusicherung,
dass keine Rechte Dritter bestehen oder geltend gemacht werden können.

Die native Unterabhängigkeitsliste wurde zusätzlich anhand der festgelegten
CMake-Quellen verfolgt. Originalhinweise (unter anderem KISS FFT und fastcluster)
stehen in licenses/Native-Inference-Notices.txt; auch die passenden nativen
Unterquellarchive einschließlich Eigen/MPL sind im Quell-Release enthalten.
