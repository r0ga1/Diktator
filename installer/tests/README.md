# Disposable Wine integration checks

These scripts are evidence/reproduction helpers for this task’s dedicated Linux/Wine container, mounted at `/io` with `WINEPREFIX=/io/prefix`. They must not run against production data. The installer test expects the actual x64 setup at `/io/Silbenlot-Setup-x64.exe`; the model check opens a native file picker for the public German reference WAV and requires manual selection through the UI. Run install/repair, model check, then uninstall. Test only the dedicated container. They preserve the existing test history and assert its byte hash after repair/model cleanup/uninstall. Windows/ARM64 hardware acceptance remains separate.

Current scripts: install_smoke12.py, installed_model12.py, uninstall_smoke12.py.

update_apply12.py exercises a local signed production setup with a disposable
0.11.99 core fixture (same source/public key, only Cargo package version changed).
Build that fixture in release mode with static CRT, copy it to
/io/Silbenlot-update-test.exe, and provide the released update.json/update.sig as
/io/release12-update.json and /io/release12-update.sig. It changes only the QA
installation's core/marker, then checks the copied post-exit helper and repair.
Never run against real user data. Network delivery is a separate release check.
