# Entwickeln und Releases bauen

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
