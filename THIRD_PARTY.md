# Diktator – Fremdkomponenten

Diktator 0.13.0 combines a Rust core with a Go window host and HTML/CSS/JavaScript UI under GPL-3.0-only with the additional permissions in ADDITIONAL_PERMISSION.md. Its earlier functional reference was VoiceInk 1.74 (GPL-3.0, https://github.com/Beingpax/VoiceInk/tree/v1.74). The supplied modified Mac executable is neither executed nor redistributed. The new distribution contains a local Go/WebView2 frontend, no llama.cpp, Qwen, Ollama integration, AI-model downloader, affiliate links or enhancement prompts.

**NVIDIA Parakeet TDT 0.6B v3**: NVIDIA Corporation, Creative Commons Attribution 4.0 International. https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3. Shipped representation: sherpa-onnx ONNX conversion, INT8 encoder with FP32 decoder/joiner and tokens. Decoder/joiner from https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3/tree/1a468a35cbba69418f126de829e75261dea4a4e4, each pinned by SHA-256 in build.py. This is a modified/quantized representation of the original model. No NVIDIA endorsement is implied. License and model card included.

**sherpa-onnx 1.13.8**: k2-fsa contributors / Xiaomi Corporation, Apache-2.0. https://github.com/k2-fsa/sherpa-onnx/releases/tag/v1.13.8. The C-API shared library is embedded in the EXE and loaded locally after verified extraction. It is native third-party code, not a Rust rewrite of the inference engine. The ABI structs in src/asr_ffi.rs are derived from this exact release's header. Unused model-family fields must remain present for binary compatibility; the app configures only Parakeet v3.

**ONNX Runtime 1.28.2**: Microsoft Corporation, MIT, as included in the pinned sherpa-onnx runtime. License and third-party notices included. CPU execution on x64/ARM64; no GPU runtime installed.

Rust packages are locked in Cargo.lock, including **CPAL** (Apache-2.0, WASAPI), **windows-sys/windows** (MIT/Apache-2.0), **Symphonia** (MPL-2.0), **serde/serde_json**, **regex**, **sha2**, **chrono**, **unicode-segmentation**, **libloading**, **hound**, **zip/flate2** and their dependencies. Complete locked package sources are included in `vendor/` in Diktator-Source.zip with their original license files. `licenses/Rust-dependencies.txt` includes attribution and the supplied top-level license/notice texts for all locked packages, including packages used only for builds or other operating systems. This file is also embedded in the portable EXE's extracted bundle. Run `tools/collect_licenses.py` when changing dependencies.

The short radio-style audio cues were synthesized specifically for Diktator; they contain no Formula 1 recordings. The monochrome app icon uses three audio bars and an abstract speech shape. It was regenerated from the earlier generated icon, removing hair, eyewear and any intended portrait/film-character likeness; no supplied stock photograph was used. Prompt/provenance is in assets/logo-prompt.txt. No copyright or trademark clearance warranty is made.

**Go 1.26.1**, **golang.org/x/sys**: BSD-3-Clause; **go-webview2** and **go-winloader**: MIT; **Microsoft WebView2Loader**: BSD notices supplied by the wrapper. Go module versions are pinned in shell/go.mod and fully vendored in shell/vendor/. Diktator patches go-webview2 to release native objects when its host window closes, suppress background erasure, explicitly synchronize visibility/bounds, use valid COM callback identity, report navigation errors and dispatch pipe calls away from the UI thread. The page carries its native bridge inline. The official SDK loader is shipped alongside the host; the complete Microsoft Fixed Runtime is also redistributed under its separate Microsoft terms as described below.

**Nunito variable font**: Vernon Adams, Cyreal and Jacques Le Bailly, SIL Open Font License 1.1. Embedded locally from the earlier UI; OFL included. Other UI text uses system fonts, with no remote font requests. Navigation/action icon paths and base visual patterns originate from the earlier implementation of this Windows project; see PROVENANCE.md for the scope and limits of the source comparison.

Build-time components: Rust 1.99.1 (MIT/Apache-2.0), LLVM-MinGW 20261020 (LLVM/Clang Apache-2.0 with LLVM exceptions, mingw-w64 and runtime terms), Python standard library. These tools are not installed on the user's PC by Diktator.

## Microsoft WebView2 Fixed Runtime

The portable bundle includes the complete official Microsoft WebView2 Fixed Version Runtime 156.0.4314.42 for its architecture, independently of the GPL Rust core. This Microsoft redistributable is **not** relicensed under GPL. Its terms are in `licenses/WebView2-FixedVersion-Microsoft.html` and `.txt`; using it is subject to those terms. Runtime third-party notices and licenses are preserved in the unmodified runtime tree. Source/download hashes are pinned in `webview-runtime.json`. Official distribution documentation: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution

Microsoft WebView2 may collect diagnostic data under its license/privacy terms: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy . Diktator itself performs speech recognition locally and never sends recordings to the renderer or a cloud transcription service. The UI loads only its embedded page and uses private pipes; background networking is disabled through the browser launch arguments.

The two 0.6.0 notification chimes are newly synthesized original PCM assets with independent in-app attenuation. No F1 recordings are used, and no system-wide output volume is changed.

The vendored Go WebView2 binding callback dispatches native calls off the UI thread and marshals replies back through Dispatch. This prevents private-pipe waits and cross-process owned file dialogs from blocking the owning native window message pump.


### WebView2 SDK Loader 1.0.4314.35

Der architekturpassende native `WebView2Loader.dll` liegt im eingebetteten UI-Paket. Er wird unverändert aus dem offiziellen Microsoft.Web.WebView2-NuGet-Paket entnommen und über den Windows-DLL-Loader geladen. Version, Downloadquelle und SHA-256 stehen in `webview-sdk.json`; die SDK-Lizenz liegt unter `licenses/WebView2SDK-BSD.txt`. Das ersetzt den bisherigen manuell in den Speicher geladenen DLL-Blob. Die vollständige Fixed Runtime bleibt separat lizenziert.


## NSIS installer

The offline installer is compiled with NSIS 3.08 from Debian's official packages. The compiler is a build-time tool; users do not install NSIS. NSIS stub/plugin copyright and license notices are included in `licenses/NSIS-copyright.txt`. The installed application retains all model, runtime and source notices.


**Silero VAD**: Silero Team, MIT. Small 16 kHz speech-activity model exported by k2-fsa; bundled as `model/silero_vad.onnx` (643854 bytes, SHA-256 `9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6`). Source: https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx. Upstream: https://github.com/snakers4/silero-vad. License in `licenses/Silero-MIT.txt`. This gate only identifies speech; Parakeet v3 remains the sole transcription model. It loads/unloads with the ASR engine, with no idle audio processing.

**FLEURS QA references (not app audio assets)**: Google / FLEURS contributors, CC BY 4.0, https://huggingface.co/datasets/google/fleurs, revision 70bb2e84b976b7e960aa89f1c648e09c59f894dd. tests/quality/fleurs-manifest.json contains reference text and audio hashes; tools/evaluate_speech.py downloads only the selected public test recordings for development evaluation. The installer does not contain these recordings.

## Primary native source snapshots
The release source archive also includes the exact primary sherpa-onnx 1.13.8 and ONNX Runtime 1.28.2 source snapshots under third_party/source-archives/. Revisions, URLs and hashes are recorded in third_party/native-sources.json. These snapshots are not the Microsoft WebView2 runtime and are not installed with the application. Their build systems document separately fetched dependencies; a bit-for-bit independent rebuild of the upstream DLLs has not been accepted in this project. See ADDITIONAL_PERMISSION.md for the limited permission concerning unmodified native components.

## Native inference subdependencies

The sherpa-onnx CPU library also compiles native dependency code; its Apache
license alone does not replace their notices. `licenses/Native-Inference-Notices.txt`
preserves the original root license/copyright notices for kaldi-native-fbank
1.22.3 (Apache-2.0), KISS FFT commit febd4cae (BSD-3-Clause / Unlicense files),
kaldi-decoder 0.3.0 and kaldifst 1.8.0 (Apache-2.0), OpenFST
1.8.5-2026-07-09 (Apache-2.0), simple-sentencepiece 0.7 (Apache-2.0) including
its Jakob Progsch / Václav Zeman thread pool (zlib notice), nlohmann/json 3.12.0
(MIT), hclust-cpp 2026-02-25 (BSD-2-Clause notices, including fastcluster authors),
and Eigen 5.0.1 (primarily MPL-2.0, plus its supplied compatible license texts).
Their exact hash-pinned source archives, including the full unmodified Eigen
source, accompany the same release's Diktator-Source.zip. MPL rights and source
availability are not waived by the application's additional permission.

This inventory follows the pinned sherpa-onnx CMake dependency graph. Its module
path selects the top-level Eigen/OpenFST definitions. Optional source tests/tools
retain their original notices inside the source archives; they are not all
compiled into the DLL. The no-TTS runtime variant is used. PortAudio demo
executables and TTS/espeak components are not shipped by Diktator. This is not a
bit-identical independent reconstruction or a legal clearance warranty.

## Public regression audio

`tests/fixtures/de.wav` is an unchanged public Google FLEURS test sample under CC BY 4.0, with exact source/revision, attribution and hash in tests/fixtures/README.md. It is included in sources for testing, not in the installed app.
