"""Rebuild the shipped sample bank from three pinned CC0 sources.

    uv run --with numpy python3 art/music/prepare.py      # from the repository root

Needs numpy and ffmpeg. Downloads go to $BAYLEE_ORCHESTRA_RAW (default
/tmp/baylee-orchestra-raw) and are checked against the SHA-256 pinned in
`samples.json`; a row without a pin is pinned on its first download (the
reviewer reads the diff). Writes, for every row of `SELECTION`:

- `crates/baylee-client-core/assets/orchestra/<name>.pcm`: mono, 44,100 Hz,
  16-bit little-endian PCM, DC removed, peak-normalised, leading silence
  trimmed; a sustain carries its loop (crossfade baked in, and the sample one
  past the loop's end is the loop's first, so interpolation reads straight
  across the seam);
- its row in `art/music/samples.json` (provenance, pitch facts, loop);
- `crates/baylee-client-core/src/music/bank.rs`, the generated table the
  sampler reads (`include_bytes!` needs literal names).

Mono: most VCSL/VSCO files are spaced microphone pairs whose channels are
weakly or negatively correlated (measured per file, `stereo_corr`); their
sum comb-filters. A file whose channels correlate at 0.5 or more is
averaged, any other ships its louder channel. The sampler pans every voice
and the room is stereo, so no file ships in stereo (the bells' and the
organ's correlation: see `samples.json`).

Pitch: the name is a prior, the measurement decides. VCSL names, and the
VSCO contrabass, cello and horn names, read one octave low (file `C4` sounds
MIDI 72); the VSCO solo violin and the FreePats bagpipe read as written. A
pitched sample whose measurement lands more than 60 cents from its expected
note is refused. The bagpipe chanter is recorded 24-65 cents flat (it is not
an equal-tempered instrument): the SFZ's `tune=` per region is the authority
for its `cents`, the measurement is recorded beside it as the check.
"""
import hashlib
import json
import os
import re
import subprocess
import sys
import urllib.parse
import urllib.request
from pathlib import Path

import numpy as np

RATE = 44100
OUT = Path("crates/baylee-client-core/assets/orchestra")
TABLE = Path("crates/baylee-client-core/src/music/bank.rs")
MANIFEST = Path("art/music/samples.json")
RAW = Path(os.environ.get("BAYLEE_ORCHESTRA_RAW", "/tmp/baylee-orchestra-raw"))

SOURCES = {
    "VCSL": {
        "repository": "https://github.com/sgossner/VCSL",
        "revision": "c1ea7bcc3c7309650ab0da9d15c9cd1fbc4a4c7e",
        "raw": "https://raw.githubusercontent.com/sgossner/VCSL/{revision}/{path}",
    },
    "VSCO 2 CE": {
        "repository": "https://github.com/sgossner/VSCO-2-CE",
        "revision": "440300901dfe9275fd84e0b7763af1f8443ae62e",
        "raw": "https://raw.githubusercontent.com/sgossner/VSCO-2-CE/{revision}/{path}",
    },
    "FreePats Bagpipe": {
        "repository": "https://github.com/freepats/bagpipe",
        "revision": "496f2f6e82f226d650e270c0f0ad5febbebec249",
        "raw": "https://raw.githubusercontent.com/freepats/bagpipe/{revision}/{path}",
    },
}
SFZ = "Bagpipe 20260806.sfz"
SFZ_SHA256 = "e83006bad23f09d3294393fe66f9942cc91392e1fdea6daaf9721b2d723b152f"

NOTE = {"C": 0, "C#": 1, "D": 2, "D#": 3, "E": 4, "F": 5, "F#": 6, "G": 7, "G#": 8, "A": 9, "A#": 10, "B": 11}

# How each kind is played unless the score says otherwise: attack and release
# in seconds, the share sent into the room, and how long a one-shot is kept.
KIND = {
    "sus": dict(attack=0.03, release=0.35, room=0.30),
    "drone": dict(attack=0.40, release=1.20, room=0.30),
    "pluck": dict(attack=0.005, release=0.90, room=0.22, keep=2.0),
    "bell": dict(attack=0.003, release=1.50, room=0.35, keep=2.6),
    "drum": dict(attack=0.002, release=0.40, room=0.14, keep=1.0),
    # The orchestra's big drums ring longer; rolls and swells are kept whole.
    "boom": dict(attack=0.002, release=0.80, room=0.22, keep=1.8),
    "swell": dict(attack=0.01, release=0.80, room=0.30, keep=3.4),
    "short": dict(attack=0.004, release=0.20, room=0.26, keep=0.55),
}
# A sustain's loop: where it starts after the onset and how long it is.
LOOP = {"sus": (0.30, 1.25), "drone": (0.60, 2.40)}
XFADE = 0.06

VCSL_ALTO = "Aerophones/Edge-blown Aerophones/Baroque Alto Recorder/Sustain/AltRecorder_Sus_{}_rr1_Main.wav"
VCSL_TENOR = "Aerophones/Edge-blown Aerophones/Baroque Tenor Recorder/Sustain/TenRecorder_Sus_{}_rr1_Main.wav"
VCSL_PSALTERY = "Chordophones/Zithers/Psaltery, Bowed and Plucked/{}/BowedPsaltery_{}_Main_{}_{}.wav"
VCSL_HARP = "Chordophones/Composite Chordophones/Folk Harp/EWHarp_Normal_{}_v3_RR1.wav"
VCSL_STRUM = "Chordophones/Composite Chordophones/Strumstick/Finger/Strumstick_Finger_Str1_Main_{}_vl2_rr1.wav"
VCSL_ORGAN = "Aerophones/Edge-blown Aerophones/Renaissance Organ/8'/RenOrgan_8foot_Room_{}_rr1.wav"
PERC = "Membranophones/Struck Membranophones/"
IDIO = "Idiophones/Struck Idiophones/"
VSCO_CTB = "Strings/Solo Contrabass/{0}/BKCtbss_{0}_{1}_v1_rr1.wav"
VSCO_CELLO = "Strings/Cello Section/susvib/susvib_{}_v1_1.wav"
VSCO_VIOLIN = "Strings/Solo Violin/Arco Vib/LLVln_ArcoVib_{}_p.wav"
VSCO_HORN = "Brass/F Horn/sus/MOHorn_sus_{}_v2_1.wav"
VSCO_VIOLINS = "Strings/Violin Section/susVib/VlnEns_susVib_{}_v1.wav"
VSCO_VIOLAS = "Strings/Viola Section/susvib/ViolaEns_susvib_{}_v1_1.wav"
VSCO_VIOLINS_SPIC = "Strings/Violin Section/Spic/VlnEns_Spic_{}_v1_rr1.wav"
VSCO_VIOLAS_SPIC = "Strings/Viola Section/spic/Violas_spic_{}_v1_rr1.wav"
VSCO_CELLOS_SPIC = "Strings/Cello Section/spic/spic_{}_v1_RR1.wav"
VSCO_TROMBONE = "Brass/Tenor Trombone/sus/tenortbn_sus_{}_v2_1.wav"
VSCO_TUBA = "Brass/Tuba/sus/Tuba3_sus_{}_v2_rr1_Mid.wav"
VSCO1 = "VSCO 1 Percussion/"


def row(family, name, source, path, kind, note=None, octave=0, rr=None):
    """One sample: `note` is the name's pitch, `octave` how far it reads low."""
    return dict(family=family, name=name, source=source, path=path, kind=kind, note=note, octave=octave)


def selection():
    rows = []
    for n in "F3 G#3 A#3 C4 D4 E4 F#4 G#4".split():
        rows.append(row("alto", "alto-" + n, "VCSL", VCSL_ALTO.format(n), "sus", n, 1))
    for n in "C3 D3 E3 F#3 G#3 A#3 C4".split():
        rows.append(row("tenor", "tenor-" + n, "VCSL", VCSL_TENOR.format(n), "sus", n, 1))
    for n, rr in [("A#3", "rr1"), ("C4", "rr1"), ("D4", "rr1"), ("E4", "rr2"), ("F#4", "rr1"), ("G#4", "rr3"), ("A#4", "rr1"), ("C5", "rr1")]:
        rows.append(row("psaltery", "psaltery-" + n, "VCSL", VCSL_PSALTERY.format("Pluck", n, "Pluck", rr), "pluck", n, 1))
    for n, rr in [("A#3", "rr2"), ("C4", "rr1"), ("D4", "rr1")]:
        rows.append(row("longbow", "longbow-" + n, "VCSL", VCSL_PSALTERY.format("LongBow", n, "LongBow", rr), "sus", n, 1))
    for n in "A#1 D2 F#2 A#2 D3 F#3 A#3 D4 F#4".split():
        rows.append(row("harp", "harp-" + n, "VCSL", VCSL_HARP.format(n), "pluck", n, 1))
    for n in "D2 F#2 G2".split():
        rows.append(row("strumstick", "strumstick-" + n, "VCSL", VCSL_STRUM.format(n), "pluck", n, 1))
    for n in "A#1 C2 F#2".split():
        rows.append(row("organ", "organ-" + n, "VCSL", VCSL_ORGAN.format(n), "drone", n, 1))
    rows.append(row("chimes", "chimes-G#3", "VCSL", IDIO + "Tubular Bells 1/chimes_G#3_p_rr2.wav", "bell", "G#3", 1))
    rows.append(row("chimes", "chimes-C4", "VCSL", IDIO + "Tubular Bells 1/chimes_C4_p_rr1.wav", "bell", "C4", 1))
    for i in (1, 2):
        rows.append(row("handbell", f"handbell-{i}", "VCSL", IDIO + f"Hand Bells, Nepalese/HB_{i}.wav", "bell"))
    rows.append(row("fingercymbal", "fingercymbal", "VCSL", IDIO + "Finger Cymbals/Fing_Cymb.wav", "bell"))
    for name, f in [("framedrum-1", "HDrumL_Hit_v2_rr1_Sum"), ("framedrum-2", "HDrumL_Hit_v3_rr1_Sum"),
                    ("framedrum-muted", "HDrumL_HitMuted_v2_rr1_Sum"), ("framedrum-small", "HDrumS_Hit_v2_rr1_Sum"),
                    ("framedrum-small-muted", "HDrumS_HitMuted_v2_rr1_Sum")]:
        rows.append(row("framedrum", name, "VCSL", PERC + f"Frame Drum/{f}.wav", "drum"))
    for name, f in [("davul-1", "BDrumNew_hit_v3_rr1_Sum"), ("davul-2", "BDrumNew_hit_v3_rr2_Sum"), ("davul-forte", "BDrumNew_hit_v5_rr1_Sum")]:
        rows.append(row("davul", name, "VCSL", PERC + f"Bass Drum 1/{f}.wav", "drum"))
    for name, f in [("naker-1", "Timpani3_Hit_v3_rr1_Sum"), ("naker-2", "Timpani3_Hit_v3_rr2_Sum"), ("naker-high", "Timpani4_Hit_v3_rr1_Sum")]:
        rows.append(row("naker", name, "VCSL", PERC + f"Timpani 1/Hit/{f}.wav", "drum"))
    for i in (1, 2):
        rows.append(row("ropesnare", f"ropesnare-{i}", "VCSL", PERC + f"Snare Drum, Rope Tension/Low/RopeSnare_low_sn_Main_vl2_rr{i}.wav", "drum"))
    rows.append(row("tambourine", "tambourine-hit", "VCSL", IDIO + "Tambourine 1/Tamb1_Hit_v2_rr1_Mid.wav", "drum"))
    for n in "C1 E1 G#1 A1".split():
        rows.append(row("contrabass", "contrabass-" + n, "VSCO 2 CE", VSCO_CTB.format("SusNV", n), "sus", n, 1))
    for n in "C1 E1 G#1 A1".split():
        rows.append(row("pizzicato", "pizzicato-" + n, "VSCO 2 CE", VSCO_CTB.format("Pizz", n), "pluck", n, 1))
    for n in "B1 D2 F2 A2".split():
        rows.append(row("cello", "cello-" + n, "VSCO 2 CE", VSCO_CELLO.format(n), "sus", n, 1))
    for n in "G3 C4 E4 G4 A4 C5".split():
        rows.append(row("violin", "violin-" + n, "VSCO 2 CE", VSCO_VIOLIN.format(n), "sus", n, 0))
    for n in "D2 F2 A2 C3".split():
        rows.append(row("horn", "horn-" + n, "VSCO 2 CE", VSCO_HORN.format(n), "sus", n, 1))
    # The orchestral body (cinematic hybrid): sections, low brass, big drums.
    for n in "D3 F#3 A3 C4 E4".split():
        rows.append(row("violins", "violins-" + n, "VSCO 2 CE", VSCO_VIOLINS.format(n), "sus", n, 1))
    for n in "D2 G2 B2 D3".split():
        rows.append(row("violas", "violas-" + n, "VSCO 2 CE", VSCO_VIOLAS.format(n), "sus", n, 1))
    for n in "D3 F#3 A3 C4".split():
        rows.append(row("violins-spic", "violins-spic-" + n, "VSCO 2 CE", VSCO_VIOLINS_SPIC.format(n), "short", n, 1))
    for n in "C2 E2 G2".split():
        rows.append(row("violas-spic", "violas-spic-" + n, "VSCO 2 CE", VSCO_VIOLAS_SPIC.format(n), "short", n, 1))
    for n in "G1 B1 D2 F2 A2".split():
        rows.append(row("cellos-spic", "cellos-spic-" + n, "VSCO 2 CE", VSCO_CELLOS_SPIC.format(n), "short", n, 1))
    for n in "A#1 D2 F2 C3".split():
        rows.append(row("trombone", "trombone-" + n, "VSCO 2 CE", VSCO_TROMBONE.format(n), "sus", n, 1))
    for n in "A#0 D#1 F1 A#1".split():
        rows.append(row("tuba", "tuba-" + n, "VSCO 2 CE", VSCO_TUBA.format(n), "sus", n, 1))
    giant = VSCO1 + "drums/other/ethnic/giant/"
    for name, f in [("taiko-ff", "mallet/EthnicLargeMallet_hit_ff_1"), ("taiko-f", "mallet/EthnicLargeMallet_hit_f_1"),
                    ("taiko-mf", "mallet/EthnicLargeMallet_hit_mf_1"), ("taiko-sticks", "sticks/EthnicLargeSticks_hit_f_1")]:
        rows.append(row("taiko", name, "VSCO 2 CE", giant + f + ".wav", "boom"))
    rows.append(row("bigdrum", "bigdrum-hit", "VSCO 2 CE", VSCO1 + "drums/bass/bdrum_fff_1.wav", "boom"))
    rows.append(row("bigdrum", "bigdrum-roll", "VSCO 2 CE", VSCO1 + "drums/bass/bdrum_roll_long1.wav", "swell"))
    rows.append(row("toms", "tom-high", "VCSL", PERC + "Tom 1/Mallet/TomH_HitM_v3_rr1_Mid.wav", "drum"))
    rows.append(row("toms", "tom-low", "VCSL", PERC + "Tom 2/Mallet/TomL_HitM_v3_rr1_Mid.wav", "drum"))
    rows.append(row("timpani", "timpani-roll", "VSCO 2 CE", "Percussion/Timpani/Rolls/Timpani2_Roll_v5_rr1_Sum.wav", "swell"))
    rows.append(row("cymbal", "cymbal-swell", "VSCO 2 CE", VSCO1 + "varMetal/Cymbals/susp/susp_hit_softmall_roll2_cresc.wav", "swell"))
    rows.append(row("gong", "gong", "VCSL", IDIO + "Gong 1/gong_f.wav", "swell"))
    for n in "F4 G4 A4 A#4 C5 D5 E5 F5 G5".split():
        f = f"{n}_32" if n == "C5" else f"{n}_31"
        rows.append(row("chanter", "chanter-" + n, "FreePats Bagpipe", f"samples/{f}.flac", "sus", n, 0))
    rows.append(row("drone", "drone-G2", "FreePats Bagpipe", "samples/drone_G2_1.flac", "drone", "G2", 0))
    rows.append(row("drone", "drone-G3", "FreePats Bagpipe", "samples/drone_G3_3.flac", "drone", "G3", 0))
    return rows


def midi_of(note):
    m = re.fullmatch(r"([A-G]#?)(-?\d)", note)
    return 12 * (int(m.group(2)) + 1) + NOTE[m.group(1)]


def fetch(source, path, sha=None):
    """The file at the pinned revision, from the cache or the network."""
    revision = SOURCES[source]["revision"]
    local = RAW / source.replace(" ", "_") / revision / path
    if not local.exists():
        local.parent.mkdir(parents=True, exist_ok=True)
        url = SOURCES[source]["raw"].format(revision=revision, path=urllib.parse.quote(path))
        local.write_bytes(urllib.request.urlopen(url, timeout=120).read())
    data = local.read_bytes()
    got = hashlib.sha256(data).hexdigest()
    if sha is not None and got != sha:
        sys.exit(f"{source} {path}: sha256 {got}, pinned {sha}")
    return local, got, len(data)


def decode(path):
    """Every channel at 44.1 kHz as float32, shape (frames, channels)."""
    channels = int(subprocess.check_output(
        ["ffprobe", "-v", "error", "-show_entries", "stream=channels", "-of", "csv=p=0", str(path)]).strip())
    data = subprocess.check_output(["ffmpeg", "-v", "error", "-i", str(path), "-ar", str(RATE), "-f", "f32le", "-"])
    return np.frombuffer(data, dtype="<f4").reshape(-1, channels).astype(np.float64), channels


def to_mono(x):
    """Average correlated channels; otherwise the louder one (no comb filter)."""
    if x.shape[1] == 1:
        return x[:, 0], None, "mono"
    left, right = x[:, 0], x[:, 1]
    corr = float(np.corrcoef(left, right)[0, 1])
    if corr >= 0.5:
        return (left + right) / 2, corr, "mid"
    if (left ** 2).sum() >= (right ** 2).sum():
        return left, corr, "left"
    return right, corr, "right"


def f0_near(x, midi, onset):
    """The strongest partial within 6 % of `midi`'s frequency, in a window
    after the attack, and its magnitude."""
    s = onset + RATE // 5
    w = x[s:s + RATE] if len(x) > s + RATE else x[-RATE:]
    w = w * np.hanning(len(w))
    n = 1 << 18
    spectrum = np.abs(np.fft.rfft(w, n))
    freqs = np.fft.rfftfreq(n, 1 / RATE)
    want = 440 * 2 ** ((midi - 69) / 12)
    lo, hi = np.searchsorted(freqs, want * 0.94), np.searchsorted(freqs, want * 1.06)
    k = lo + int(np.argmax(spectrum[lo:hi]))
    return float(1200 * np.log2(freqs[k] / want)), float(spectrum[k])


def measure(x, expected, onset):
    """(cents off `expected`, whether an octave below holds no partial): the
    measurement that confirms or refuses the name."""
    cents, here = f0_near(x, expected, onset)
    _, below = f0_near(x, expected - 12, onset)
    return cents, below < here * 0.25


def find_loop(x, start, length):
    """A loop beginning at `start`, about `length` frames long, ending where a
    window best matches its beginning (normalised correlation)."""
    win = 2048
    ref = x[start:start + win]
    want = start + length
    best, end = -2.0, want
    span = int(0.25 * RATE)
    for cand in range(max(start + RATE // 2, want - span), min(len(x) - win - 1, want + span)):
        seg = x[cand:cand + win]
        c = float(np.dot(ref, seg) / (np.linalg.norm(ref) * np.linalg.norm(seg) + 1e-12))
        if c > best:
            best, end = c, cand
    return end, best


def bake_loop(x, start, end):
    """Crossfade the loop's tail into the material before its start, and make
    the sample at `end` the loop's first: the file is `end + 1` long."""
    xf = min(int(XFADE * RATE), start)
    t = np.linspace(0, 1, xf, endpoint=False)
    y = x[:end + 1].copy()
    y[end - xf:end] = x[end - xf:end] * (1 - t) + x[start - xf:start] * t
    y[end] = y[start]
    return y


def sfz_regions(text):
    out = {}
    for m in re.finditer(r"(?:tune=(-?\d+)\s+)?loop_start=(\d+) loop_end=(\d+)\s+sample=samples/([^\s]+)\.flac", text):
        out[m.group(4)] = dict(tune=int(m.group(1) or 0), loop=[int(m.group(2)), int(m.group(3))])
    return out


def prepare(spec, pinned, sfz):
    local, sha, size = fetch(spec["source"], spec["path"], pinned.get("sha256"))
    x, channels = decode(local)
    mono, corr, how = to_mono(x)
    mono = mono - mono.mean()
    peak = np.abs(mono).max()
    onset = max(0, int(np.flatnonzero(np.abs(mono) > peak * 0.012)[0]) - 220)
    kind = spec["kind"]
    out = dict(spec)
    out.update(revision=SOURCES[spec["source"]]["revision"], sha256=sha, bytes=size, channels=channels,
               stereo_corr=None if corr is None else round(corr, 3), mono=how)
    stem = Path(spec["path"]).stem
    region = sfz.get(stem) if spec["source"] == "FreePats Bagpipe" else None
    if spec["note"] is not None and kind != "bell":
        expected = midi_of(spec["note"]) + 12 * spec["octave"]
        cents, clean_below = measure(mono, expected, onset)
        # A bagpipe note is held against its SFZ tuning, any other against
        # the equal-tempered note its name and octave say.
        claim = -float(region["tune"]) if region is not None else 0.0
        if abs(cents - claim) > (15 if region is not None else 60) or not clean_below:
            sys.exit(f"{spec['name']}: measured {cents:+.1f} c off MIDI {expected}, expected {claim:+.1f} "
                     f"(octave below clean: {clean_below})")
        out.update(midi=expected, measured_cents=round(cents, 1))
        if region is not None and region["tune"]:
            out.update(cents=-float(region["tune"]), pitch_basis="sfz tune")
        else:
            out.update(cents=round(cents, 1), pitch_basis="measured")
    else:
        expected = midi_of(spec["note"]) + 12 * spec["octave"] if spec["note"] else 60
        out.update(midi=expected, cents=0.0, measured_cents=None, pitch_basis="named" if spec["note"] else "unpitched")
    if kind in LOOP:
        if region is not None:
            # The SFZ's loop start is kept; a shorter end is found inside its loop.
            sfz_start, sfz_end = region["loop"]
            start = sfz_start - (onset if sfz_start > onset else 0)
            body = mono[onset:]
            length = min(int(LOOP[kind][1] * RATE), sfz_end - sfz_start - 1)
            out["sfz_loop"] = region["loop"]
        else:
            body = mono[onset:]
            start = int(LOOP[kind][0] * RATE)
            length = int(LOOP[kind][1] * RATE)
        end, match = find_loop(body, start, length)
        y = bake_loop(body, start, end)
        out.update(loop=[start, end], loop_match=round(match, 4))
    else:
        keep = int(KIND[kind]["keep"] * RATE)
        y = mono[onset:onset + keep].copy()
        n = min(int(0.25 * RATE), len(y) // 4)
        y[-n:] *= np.linspace(1, 0, n) ** 2
        y[:32] *= np.linspace(0, 1, 32)
        out.update(loop=None)
    y *= 0.89 / np.abs(y).max()
    pcm = np.round(np.clip(y, -1, 1) * 32767).astype("<i2").tobytes()
    (OUT / (spec["name"] + ".pcm")).write_bytes(pcm)
    params = KIND[kind]
    out.update(attack=params["attack"], release=params["release"], room_send=params["room"],
               frames=len(y), rate=RATE, pcm_sha256=hashlib.sha256(pcm).hexdigest())
    for k in ("note", "octave"):
        out.pop(k)
    return out


def rust_table(rows):
    kinds = {"sus": "Sus", "drone": "Drone", "pluck": "Pluck", "bell": "Bell", "drum": "Drum",
             "boom": "Drum", "swell": "Drum", "short": "Pluck"}
    lines = [
        "// GENERATED by art/music/prepare.py from art/music/samples.json; do not edit.",
        "//! The sample bank as shipped: one row per recording, in the families the",
        "//! score plays, each family sorted by pitch.",
        "#![allow(clippy::unreadable_literal)]",
        "use super::orchestra::{Def, Family, Kind};",
        "",
        "/// Every recording in the bank, families contiguous and sorted by pitch.",
        "pub(super) const BANK: &[Def] = &[",
    ]
    families = []
    for i, r in enumerate(rows):
        if not families or families[-1][0] != r["family"]:
            families.append([r["family"], i, 0])
        families[-1][2] += 1
        loop = "None" if r["loop"] is None else f"Some(({r['loop'][0]}, {r['loop'][1]}))"
        lines.append(
            f"    Def {{ name: {json.dumps(r['name'])}, pcm: include_bytes!(\"../../assets/orchestra/{r['name']}.pcm\"), "
            f"midi: {r['midi']}, cents: {float(r['cents'])!r}, looped: {loop}, kind: Kind::{kinds[r['kind']]}, "
            f"attack: {r['attack']!r}, release: {r['release']!r}, room_send: {r['room_send']!r} }},")
    lines.append("];")
    lines.append("")
    for name, first, count in families:
        const = name.upper().replace("-", "_")
        lines.append(f"/// The {name} family: {count} recording{'s' if count > 1 else ''}.")
        lines.append("#[allow(dead_code)] // a drum or a bell is played by its recording's name")
        lines.append(f"pub(super) const {const}: Family = Family {{ first: {first}, len: {count} }};")
    lines.append("")
    lines.append("/// Each recording's place in [`BANK`], by name (`#` spelt `S`).")
    lines.append("#[allow(dead_code)] // the score names the ones it plays by hand")
    lines.append("pub(super) mod at {")
    for i, r in enumerate(rows):
        const = r["name"].upper().replace("-", "_").replace("#", "S")
        lines.append(f"    pub(in crate::music) const {const}: usize = {i};")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    pinned = {r["name"]: r for r in json.loads(MANIFEST.read_text())} if MANIFEST.exists() else {}
    sfz_path, _, _ = fetch("FreePats Bagpipe", SFZ, SFZ_SHA256)
    sfz = sfz_regions(sfz_path.read_text())
    rows = []
    for spec in selection():
        r = prepare(spec, pinned.get(spec["name"], {}), sfz)
        rows.append(r)
        print(f"{r['name']:24s} midi {r['midi']:3d} {r['cents']:+6.1f} c ({r['pitch_basis']}), "
              f"{r['frames'] / RATE:4.2f} s, {r['mono']}, loop {r['loop']}")
    order = {}
    for r in rows:
        order.setdefault(r["family"], len(order))
    rows.sort(key=lambda r: (order[r["family"]], r["midi"], r["name"]))
    keep = {r["name"] + ".pcm" for r in rows}
    for stale in OUT.glob("*.pcm"):
        if stale.name not in keep:
            stale.unlink()
    MANIFEST.write_text(json.dumps(rows, indent=2, ensure_ascii=False) + "\n")
    TABLE.write_text(rust_table(rows))
    # Formatted as `cargo fmt` would, so a rebuild changes nothing it need not.
    subprocess.run(["rustfmt", "--edition", "2024", str(TABLE)], check=True)
    total = sum(r["frames"] * 2 for r in rows)
    print(f"{len(rows)} recordings, {total} bytes ({total / 1e6:.2f} MB)")


if __name__ == "__main__":
    main()
