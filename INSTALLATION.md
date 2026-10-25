# Diktator 0.13.0 (Beta) — Installation unter Windows

**Direkt herunterladen:** [Windows x64 · Intel/AMD](https://github.com/r0ga1/Diktator/releases/download/v0.13.0/Diktator-Setup-x64.exe)
oder [Windows ARM64 · Snapdragon](https://github.com/r0ga1/Diktator/releases/download/v0.13.0/Diktator-Setup-ARM64.exe).
Du brauchst genau **eine** der beiden EXEs. GitHubs „Code → Download ZIP“ enthält
Entwicklerdateien und ist nicht der Installer.

## Was du brauchst

**Nur eine passende Setup-EXE.** Du brauchst weder Excel noch Python, Rust, Go, CUDA, ein zusätzliches KI-Modell oder einen separaten WebView2-Installer. Parakeet v3, die Oberfläche und alle benötigten Laufzeiten sind enthalten. Installation und Diktieren benötigen keine Cloud-Spracherkennung. Plane etwa 3 GB freien Speicher ein; ungefähr 8 GB RAM sind für lokale Erkennung sinnvoll, aber kein gemessener Mindestwert.

- **Intel-/AMD-PC:** `Diktator-Setup-x64.exe`.
- **Windows auf ARM, etwa Snapdragon:** `Diktator-Setup-ARM64.exe`.

Den Systemtyp findest du unter Windows **Einstellungen → System → Info**. Bei „x64-basierter Prozessor“ nimm x64; bei einem ARM-basierten Prozessor nimm ARM64. Die Dateien sind unsignierte Testbuilds; Windows kann einen Hinweis zum unbekannten Herausgeber anzeigen.

## Installation

1. Die oben verlinkte passende Datei mit **Setup** im Namen herunterladen und öffnen.
2. Im Assistenten **Weiter** wählen und die separat mitgelieferten Microsoft-Runtime-Bedingungen prüfen. Nur bei Zustimmung fortfahren. Eine noch laufende Diktator-Version wird regulär beendet; ein laufendes Diktat darf abschließen. Falls das nicht gelingt, die alte App über ihr Symbol neben der Windows-Uhr → **Programm beenden** schließen und erneut versuchen.
3. Den vorgeschlagenen Zielordner übernehmen: `%LOCALAPPDATA%\Programs\Diktator`. Das ist der App-Ordner für dein Benutzerkonto und benötigt keine Administratorrechte. Im Assistenten lässt sich ein anderer beschreibbarer Ordner wählen. Der systemweite Ordner `C:\Program Files` benötigt dagegen erhöhte Rechte und ist nicht das Ziel dieses Setups.
4. Bei Bedarf die **Desktop-Verknüpfung** auswählen und **Installieren** anklicken. Setup kopiert Modell, Oberfläche und Laufzeit und prüft die installierten Dateien. Bitte den Abschluss abwarten.
5. **Fertigstellen** wählen. „Diktator jetzt öffnen“ kann aktiviert bleiben. Später die App über **Start → Diktator** oder das Desktop-Symbol öffnen. **Die Setup-Datei musst du nicht bei jedem Start erneut öffnen.**

Die Setup-Datei kann nach erfolgreicher Installation gelöscht oder für eine spätere Reparatur aufbewahrt werden. Die installierten Programmdateien dürfen nicht einzeln verschoben werden. Alte portable EXEs danach nicht parallel starten.

## Erster Mikrofontest und Diktat

1. **Einstellungen → Mikrofon** öffnen, Mikrofon auswählen und **Mikrofon testen** anklicken. Der Test speichert keinen Verlauf.
2. Falls nötig über den dortigen Link die Windows-Mikrofonberechtigungen öffnen und den Zugriff für Desktop-Apps erlauben.
3. Das gewünschte Textfeld in einem anderen Programm anklicken und das eingestellte Diktierkürzel drücken. Bei neuen Einstellungen ist es **Strg+Alt+Leertaste**; ein bereits geändertes Kürzel bleibt erhalten.
4. Sprechen und mit demselben Kürzel stoppen. Text wird während des Sprechens eingefügt; nach Stop folgt die abschließende Erkennung. Start- und Stoppton sind getrennt regelbar. Beim ersten Diktat muss das lokale Modell zunächst laden.

**X** schließt standardmäßig in den Infobereich neben der Uhr, eventuell hinter dem Pfeil. Rechtsklick auf das Diktator-Symbol → **Programm beenden** beendet vollständig. Dieses Verhalten und Autostart lassen sich in den Einstellungen ändern.

## Reparatur und Deinstallation

- **Reparatur:** Dasselbe oder ein neueres passendes Setup nochmals ausführen und denselben Zielordner verwenden. Die Programmdateien werden neu bereitgestellt und geprüft. Verlauf und Einstellungen bleiben erhalten.
- **Deinstallation:** Windows **Einstellungen → Apps → Installierte Apps → Diktator → Deinstallieren**. Alternativ im Startmenü **Diktator deinstallieren** wählen. Die App wird regulär beendet und Programmdateien/Verknüpfungen werden entfernt.
- **Persönliche Daten:** Verlauf, Wörterbuch und Einstellungen bleiben unter `%LOCALAPPDATA%\Diktator` erhalten. Die Deinstallation löscht diese Daten nicht. Auch frühere portable Cache-Dateien werden nicht ungefragt entfernt.

## Wenn das Dashboard leer bleibt oder ein Startfehler erscheint

Ein Installer behebt die Installation und feste Dateipfade. Er ist kein automatischer Nachweis, dass ein bisheriger Rendererfehler behoben ist. Die native Dashboard-Abnahme auf echtem Windows ist weiterhin offen; der jeweilige Prüfbericht beschreibt die tatsächlich ausgeführten Tests.

Diktator vollständig über das Tray-Menü beenden, das Setup zur Reparatur erneut ausführen und anschließend **Start → Diktator** öffnen. Bleibt das Problem bestehen, die genaue Fehlermeldung und die Datei `%LOCALAPPDATA%\Diktator\ui-start.log` sichern. Du kannst den Pfad mit **Win+R** öffnen. Damit lässt sich die konkrete Ursache am Windows-PC untersuchen. Der Startfehler darf nicht als fehlendes Python-/CUDA-/Modellpaket fehlgedeutet werden: diese zusätzlichen Installationen werden nicht benötigt.

Automatische Updates sind standardmäßig eingeschaltet und fest an
`r0ga1/Diktator` gebunden (aktuell Betakanal). Es muss keine URL eingetragen
werden. Signierte, geprüfte Updates werden beim vollständigen Beenden installiert.
Unter **Einstellungen → App & Updates** lässt sich die Funktion ausschalten. Ein
Quellcode-Push allein ist noch kein signierter App-Release. Bei geänderten
Microsoft-Runtime-Bedingungen ist eine manuell geprüfte Installation erforderlich.
