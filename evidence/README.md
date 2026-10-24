# 0.12.0 build-time evidence

24.10.2026. The application core and tests are cross-compiled for Windows x64;
executed in a dedicated Linux/Wine container on macOS/Rosetta, not native Windows.

- 26 Rust tests passed: 13 unit/signature, 7 model/settings/migration, 6 bundle.
  The CNG test uses a real RSA-PSS signature and rejects changed/missing signatures.
- Real model checks cover German/English/French, quiet speech, sentence edges,
  mixed-language context, silence, noise/hum/click rejection and live edit patches.
- Go private-pipe tests passed with the race detector (1.655 s).
- Node dashboard bootstrap, hidden idle polling, fixed feed/license links and
  core/render failure cases passed. This is not a native WebView2 visual test.
- Installation/repair verified 296 deployed x64 files, preserved six test entries,
  restored a damaged UI file and did not create a second portable model cache.
- Actual installed Parakeet recognized the German reference sentence correctly.
- Legacy data migration preserved old data bytes and all test history, removed
  the editable repository setting and exposed the five allowed legal texts.
- The optimized 0.11.99 fixture verified the signed production manifest/setup,
  exited and used a copied helper outside the install folder. The real setup
  restored 0.12.0 and its exact core hash, preserving test data (54.783 s).
- Both setup bootstraps, application/native payload PE architectures, user-level
  manifests, every payload hash and model precision were statically checked.
- The interactive German wizard and unchecked Microsoft license acceptance step
  were visually inspected, then cancelled. Automated tests use disposable QA.

The initial local update test supplies the payload locally; it does not prove
GitHub/WinHTTP delivery. The final notice-bearing installer also passed uninstall: program directory,
shortcuts and owned registry entries removed; personal data retained.
Post-publication network-delivery results are reported separately with the release. Native Windows dashboard,
real microphone/device coverage and ARM64 execution remain hardware acceptance
items. No independent security or legal clearance is claimed.

Before public release, native dependency notices and 12 hash-pinned source
archives were added, including both Eigen versions (sherpa and ONNX Runtime).
The core, Go host and models did not change in this packaging revision. Both
final packages were hash/architecture checked again; x64 install/repair/uninstall
were rerun. The earlier installed-model and local post-exit tests use the same
application/model code and are identified separately in their JSON evidence.
