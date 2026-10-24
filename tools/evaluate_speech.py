#!/usr/bin/env python3
"""Reproduce Silbenlot's bounded local FLEURS comparison. Python standard library only.

Build examples/transcribe_file first. Pass --fetch to download the public test
recordings (CC BY 4.0); no audio is uploaded and nothing is installed in the app.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tarfile
import time
import unicodedata
import urllib.request


def words(text):
    return re.findall(r"[^\W_]+", unicodedata.normalize("NFKC", text).casefold())


def distance(reference, hypothesis):
    previous = list(range(len(hypothesis) + 1))
    for i, word in enumerate(reference, 1):
        row = [i]
        for j, candidate in enumerate(hypothesis, 1):
            row.append(min(row[-1] + 1, previous[j] + 1,
                           previous[j - 1] + (word != candidate)))
        previous = row
    return previous[-1]


def verify(path, expected):
    return path.is_file() and hashlib.sha256(path.read_bytes()).hexdigest() == expected


def fetch(samples, directory):
    groups = collections.defaultdict(dict)
    for sample in samples:
        if not verify(directory / sample["file"], sample["sha256"]):
            groups[sample["source"]][Path(sample["file"]).name] = sample
    for url, missing in groups.items():
        with urllib.request.urlopen(url, timeout=60) as response:
            # Stream only until all selected files are found. Never extract paths.
            with tarfile.open(fileobj=response, mode="r|gz") as archive:
                for member in archive:
                    sample = missing.get(Path(member.name).name)
                    if not member.isfile() or sample is None:
                        continue
                    if member.size > 10 * 1024 * 1024:
                        raise ValueError("Unexpected test recording size")
                    content = archive.extractfile(member).read()
                    if hashlib.sha256(content).hexdigest() != sample["sha256"]:
                        raise ValueError("Test recording checksum mismatch")
                    destination = directory / sample["file"]
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    destination.write_bytes(content)
                    del missing[Path(member.name).name]
                    if not missing:
                        break
        if missing:
            raise ValueError("Selected recordings missing from pinned archive")


def aggregate(samples):
    out = {}
    for language in sorted({s["language"] for s in samples}):
        for selection in ["all", "initial", "expansion"]:
            rows = [s for s in samples if s["language"] == language
                    and (selection == "all" or s["selection"] == selection)]
            errors = sum(s["errors"] for s in rows)
            count = sum(s["words"] for s in rows)
            out[f"{language}/{selection}"] = {
                "recordings": len(rows), "errors": errors, "words": count,
                "werPercent": round(100 * errors / count, 4) if count else None}
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", required=True, type=Path)
    parser.add_argument("--model", required=True, type=Path)
    parser.add_argument("--runtime", required=True, type=Path)
    parser.add_argument("--audio-dir", required=True, type=Path)
    parser.add_argument("--manifest", type=Path, default=Path(__file__).resolve().parents[1]
                        / "tests/quality/fleurs-manifest.json")
    parser.add_argument("--fetch", action="store_true")
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text())
    samples = manifest["samples"]
    # Manifest paths must stay within the caller's chosen QA audio directory.
    for sample in samples:
        path = Path(sample["file"])
        if path.is_absolute() or ".." in path.parts:
            raise ValueError("Invalid test recording path")
    if args.fetch:
        fetch(samples, args.audio_dir)
    for sample in samples:
        if not verify(args.audio_dir / sample["file"], sample["sha256"]):
            raise ValueError("Missing/changed recording; use --fetch: " + sample["file"])
    started = time.monotonic()
    command = [str(args.exe.resolve()), str(args.model.resolve()),
               str(args.runtime.resolve())]
    command += [str((args.audio_dir / s["file"]).resolve()) for s in samples]
    result = subprocess.run(command, capture_output=True, text=True, check=True)
    hypotheses = {}
    for line in result.stdout.splitlines():
        path, separator, text = line.partition(": ")
        if separator:
            hypotheses[path] = text
    rows = []
    for sample in samples:
        text = hypotheses[str((args.audio_dir / sample["file"]).resolve())]
        reference = words(sample["reference"])
        rows.append({**sample, "hypothesis": text, "words": len(reference),
                     "errors": distance(reference, words(text))})
    report = {
        "dataset": manifest["dataset"], "samples": rows,
        "aggregate": aggregate(rows), "seconds": round(time.monotonic() - started, 3),
        "executableSha256": hashlib.sha256(args.exe.read_bytes()).hexdigest(),
        "normalization": "NFKC casefold; Unicode words; punctuation ignored; no number expansion",
        "limits": "Small selected set; German recordings marked male only; expansion set was also inspected for final variant selection; not a blind test, ChatGPT/Gemini comparison or general WER benchmark. Timings require a separate controlled performance test."}
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2))
    print(json.dumps(report["aggregate"], indent=2))


if __name__ == "__main__":
    main()
