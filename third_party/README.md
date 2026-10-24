# Native inference source snapshots

`native-sources.json` pins the primary source revisions corresponding to the
unmodified sherpa-onnx 1.13.8 / ONNX Runtime 1.28.2 binaries consumed by build.py.
The release source ZIP includes both `.tar.gz` files under `source-archives/`.
These large snapshots are release assets, not tracked as ordinary Git blobs.
All Rust/Go dependency sources and their licenses remain in vendor/ and
shell/vendor/. The model is separately licensed CC BY 4.0 data, not relicensed GPL.

To inspect/rebuild the native components, extract the snapshots in a development
folder and follow their included CMake/Windows build documentation. The sherpa
binary variant is shared, CPU, MT, MinSizeRel, without TTS; target x64 or ARM64.
Its CMake dependency files pin the separately downloaded source prerequisites.
ONNX Runtime's own build system documents its native prerequisites and options.
An independent bit-identical rebuild of these upstream DLLs was not performed.
The shipped binary archive URLs/hashes in build.py remain the exact build inputs.
See ADDITIONAL_PERMISSION.md for the limited permission for separately licensed,
unmodified inference components; modified/incorporated GPL code still requires
its corresponding source. Microsoft WebView2 is a separate redistributable with
its own terms; no purported GPL source is provided for its proprietary parts.

The archive inventory now includes the pinned native subdependencies, including
KISS FFT, Eigen, OpenFST, kaldifst, kaldi-decoder, kaldi-native-fbank,
simple-sentencepiece, json and hclust-cpp. Their original notices are also in
licenses/Native-Inference-Notices.txt in the app. Eigen's full unmodified source
is available in the same release source ZIP to preserve MPL source access.

ONNX Runtime additionally pins its own Eigen 3.4-branch commit 1d8b82b.
That exact full source ZIP is also included (upstream deps.txt SHA-1 verified,
additionally pinned here with SHA-256); sherpa's Eigen 5.0.1 is not substituted
for it. The runtime's original ThirdPartyNotices retains the MPL terms.
