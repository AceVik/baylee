"""Losslessly pack stereo recordings, pitched off-line with libsoxr, for the memorial.
uv run --with numpy==2.4.3 --with soundfile==0.13.1 --with soxr==1.0.0 art/music/memorial/prepare_bank.py
Raw downloads are cached outside the repository.
"""

import concurrent.futures
import hashlib
import json
import re
import struct
import threading
import urllib.parse
import urllib.request
import zlib
from pathlib import Path

import numpy as np
import soundfile as sf
import soxr

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUT = ROOT / "crates/baylee-client-core/assets/chamber"
CACHE = Path("/tmp/baylee-piano-revision/raw")
OSIRIS = "18c6afccb60cff458edbf7c394571783e074e1e9"
VSCO = "440300901dfe9275fd84e0b7763af1f8443ae62e"
RATE = 48000
DOWNLOAD = threading.Lock()
FILE_LOCKS = {}


def fetch(repo, revision, path):
    url = (
        "https://raw.githubusercontent.com/"
        + repo
        + "/"
        + revision
        + "/"
        + urllib.parse.quote(path)
    )
    file = CACHE / repo / revision / path
    with DOWNLOAD:
        lock = FILE_LOCKS.setdefault(str(file), threading.Lock())
    with lock:
        if not file.exists():
            file.parent.mkdir(parents=True, exist_ok=True)
            with urllib.request.urlopen(url, timeout=60) as response:
                file.write_bytes(response.read())
            print(f"Downloaded {path}", flush=True)
    return file, url


def regions(layer):
    path = f"Programs/modules/mappings/uc_sus_mica_vl{layer}_map.sfz"
    file, _ = fetch("sfzinstruments/Osiris_Piano", OSIRIS, path)
    result = {}
    for block in file.read_text().split("<region>")[1:]:
        row = dict(re.findall(r"(\w+)=([^\n\r]+)", block))
        root = int(row.get("pitch_keycenter", 60))
        for pitch in range(int(row["lokey"]), int(row["hikey"]) + 1):
            if pitch not in result or abs(pitch - root) < abs(pitch - result[pitch][0]):
                result[pitch] = (root, row["sample"].removeprefix("../"))
    return result


def midi(note):
    match = re.fullmatch(r"([A-G])(#?)(\d)", note)
    return (
        (int(match[3]) + 1) * 12
        + {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}[match[1]]
        + bool(match[2])
    )


def sources():
    data = (
        ROOT / "crates/baylee-client-core/src/music/score/memorial/performance.bin"
    ).read_bytes()
    lengths = {}
    for _, length, _, _, instrument, pitch, _, _ in struct.iter_unpack(
        "<IIffBBBB", data[12:]
    ):
        key = instrument, pitch
        lengths[key] = max(lengths.get(key, 0), length / RATE)
    tree_path = CACHE / "vsco-tree.json"
    if not tree_path.exists():
        tree_path.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(
            "https://api.github.com/repos/sgossner/VSCO-2-CE/git/trees/"
            + VSCO
            + "?recursive=1"
        ) as response:
            tree_path.write_bytes(response.read())
    tree = json.loads(tree_path.read_text())
    strings = [[], []]
    old = json.loads((ROOT / "art/music/samples.json").read_text())
    cents = {row["path"]: row.get("cents", 0) for row in old}
    for row in tree["tree"]:
        path = row["path"]
        if "Solo Violin/Arco Vib/" in path and path.endswith("_p.wav"):
            note = path.split("_")[-2]
            strings[0].append((midi(note), path, cents.get(path, 0)))
        elif "Cello Section/susvib/" in path and path.endswith("_v1_1.wav"):
            note = path.split("_")[-3]
            strings[1].append((midi(note) + 12, path, cents.get(path, 0)))
    maps = {layer: regions(layer) for layer in [1, 2]}
    rows = []
    for (instrument, pitch), hold in sorted(lengths.items()):
        for layer in [1, 2] if instrument == 0 else [1]:
            if instrument == 0:
                root, path = maps[layer][pitch]
                repo, rev, tuning = "sfzinstruments/Osiris_Piano", OSIRIS, 0
            else:
                root, path, tuning = min(
                    strings[instrument - 1], key=lambda r: abs(r[0] - pitch)
                )
                repo, rev = "sgossner/VSCO-2-CE", VSCO
            rows.append(
                {
                    "instrument": instrument,
                    "pitch": pitch,
                    "layer": layer,
                    "root": root,
                    "path": path,
                    "repo": repo,
                    "revision": rev,
                    "cents": tuning,
                    "keep_seconds": max(6.0, hold + 1.7),
                }
            )
    return rows


def prepare(row):
    file, url = fetch(row["repo"], row["revision"], row["path"])
    original, rate = sf.read(file, dtype="float64", always_2d=True)
    assert original.shape[1] == 2, file
    # Retain the source's stereo image, dynamic differences and natural decay.
    # Remove DC and only leading pre-note silence, retaining 3 ms before onset.
    original -= original.mean(axis=0)
    envelope = np.max(np.abs(original), axis=1)
    onset = np.flatnonzero(envelope > max(0.00008, float(envelope.max()) * 0.012))[0]
    start = max(0, int(onset) - round(rate * 0.003))
    original = original[start:]
    # Current VSCO recordings already have measured tuning corrections for the
    # original bank. Unmeasured roots retain their publisher's natural tuning.
    ratio = 2 ** ((row["pitch"] - row["root"] - row["cents"] / 100) / 12)
    audio = soxr.resample(original, rate * ratio, RATE, quality="VHQ").astype(
        np.float32
    )
    wanted = round(row["keep_seconds"] * RATE)
    repeated = 0
    if row["instrument"] and len(audio) < wanted:
        # Long held bows use a multi-second natural middle, crossfaded over
        # 100 ms. Never cycle the tiny 0.9-second game-bank loop.
        middle = audio[len(audio) // 3 : len(audio) * 4 // 5].copy()
        fade = min(round(RATE * 0.10), len(middle) // 8)
        while len(audio) < wanted:
            w = np.linspace(0, 1, fade)[:, None]
            audio[-fade:] = audio[-fade:] * (1 - w) + middle[:fade] * w
            audio = np.concatenate([audio, middle[fade:]])
            repeated += 1
    audio = audio[:wanted]
    fade = min(round(RATE * 0.08), len(audio) // 4)
    audio[-fade:] *= np.linspace(1, 0, fade)[:, None]
    audio[:48] *= np.linspace(0, 1, 48)[:, None]
    natural_frames = len(audio)
    # A high piano key can decay completely before key-up. Preserve its natural
    # end and pad silence, never stretch or loop a dying piano recording.
    if len(audio) < wanted:
        audio = np.pad(audio, ((0, wanted - len(audio)), (0, 0)))
    assert np.isfinite(audio).all() and np.abs(audio).max() < 1.0
    pcm = np.round(audio * 32767).astype("<i2").tobytes()
    info = {
        **row,
        "url": url,
        "source_sha256": hashlib.sha256(file.read_bytes()).hexdigest(),
        "source_rate": rate,
        "source_frames": len(envelope),
        "trim_start": start,
        "source_subtype": sf.info(file).subtype,
        "frames": len(audio),
        "natural_frames": natural_frames,
        "extended_bows": repeated,
        "pcm_sha256": hashlib.sha256(pcm).hexdigest(),
    }
    return info, pcm


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    rows = sources()
    print(f"Preparing {len(rows)} stereo clips", flush=True)
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        prepared = list(pool.map(prepare, rows))
    raw = bytearray(struct.pack("<4sII", b"CHM1", RATE, len(rows)))
    for row, pcm in prepared:
        raw.extend(
            struct.pack(
                "<BBBBI",
                row["instrument"],
                row["pitch"],
                row["layer"],
                0,
                row["frames"],
            )
        )
        raw.extend(pcm)
    packed = zlib.compress(raw, level=9)
    # Four lossless chunks keep each ordinary Git blob below hosting limits.
    size = (len(packed) + 3) // 4
    chunks = {}
    for i in range(4):
        name = f"stereo-{i + 1:02}.pcm.zlib"
        chunk = packed[i * size : (i + 1) * size]
        (OUT / name).write_bytes(chunk)
        chunks[name] = hashlib.sha256(chunk).hexdigest()
    manifest = {
        "rate": RATE,
        "channels": 2,
        "bits": 16,
        "records": len(rows),
        "resampler": "libsoxr " + soxr.__libsoxr_version__,
        "quality": "VHQ",
        "decoded_bytes": len(raw),
        "packed_bytes": len(packed),
        "sha256": hashlib.sha256(packed).hexdigest(),
        "chunks": chunks,
        "samples": [row for row, _ in prepared],
    }
    (HERE / "instruments.json").write_text(json.dumps(manifest, indent=2) + "\n")
    for repo, revision, target in [
        ("sfzinstruments/Osiris_Piano", OSIRIS, "LICENSE-Osiris.txt"),
        ("sgossner/VSCO-2-CE", VSCO, "LICENSE-VSCO.txt"),
    ]:
        file, _ = fetch(repo, revision, "LICENSE")
        (OUT / target).write_bytes(file.read_bytes())
    print(
        f"{len(rows)} stereo clips; {len(raw) / 1048576:.1f} MiB decoded; {len(packed) / 1048576:.1f} MiB packed",
        flush=True,
    )


if __name__ == "__main__":
    main()
