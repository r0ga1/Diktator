# 0.13.0 · Native Windows acceptance

25 October 2026. Release binaries remain the immutable build from tag v0.13.0.

- [Native Rust and Go tests](https://github.com/r0ga1/Diktator/actions/runs/40187612254): publisher signatures, sided shortcut matching, migration, stalled-worker watchdog and Go race tests.
- [Published installer/dashboard](https://github.com/r0ga1/Diktator/actions/runs/40189288026): fresh per-user installation, all installed hashes, ordinary WebView2 dashboard ready and regular shutdown.
- [Full original 0.12 update](https://github.com/r0ga1/Diktator/actions/runs/40190099910): native automatic WinHTTP download, CNG verification, post-exit installation, folder/registry transition, preserved old data, migrated history/dictionary, installed dashboard, real supervised German Parakeet inference, right Ctrl and right Ctrl+F9 capture through the Windows hook, history clearing and dictionary preservation.

`windows-0.13.json` is the exported result from the last run. With the model unloaded and dashboard closed, the core used 12.5 MiB working set and 0.0% of one logical CPU during a two-second CI sample. This is a short VM measurement, not a guarantee for every PC or the memory footprint of an open dashboard.

The public German audio is attributed in tests/fixtures/README.md. The transcript retains errors: these checks do not establish ChatGPT/Gemini parity or error-free dictation. Physical microphone coverage, all target applications and ARM64 execution remain untested on real user hardware. The installer has no Authenticode certificate. Legal limitations remain in PROVENANCE.md.

The release source archive records the build-time tree. Post-publication acceptance results and improvements to the test harness are recorded on main and in the release report.
