"""Prepare a flowing grand-piano/strings performance from two public-domain scores.
Run with uv run --with mido==1.3.3 art/music/memorial/prepare.py.
Pitches and written durations come from the source, performance decisions below.
"""

import bisect
import hashlib
import itertools
import json
import math
import struct
from collections import defaultdict
from pathlib import Path

import mido

HERE = Path(__file__).resolve().parent
OUT = HERE.parents[2] / "crates/baylee-client-core/src/music/score/memorial"
RATE = 48000


def read(work):
    midi = mido.MidiFile(HERE / "sources" / f"{work}.mid")
    notes = []
    for part, track in enumerate(midi.tracks):
        tick, active = 0, defaultdict(list)
        for msg in track:
            tick += msg.time
            beat = tick / midi.ticks_per_beat
            if msg.type == "note_on" and msg.velocity:
                active[msg.note].append((beat, msg.velocity))
            elif msg.type in ("note_on", "note_off") and active[msg.note]:
                start, velocity = active[msg.note].pop(0)
                notes.append((start, beat - start, part, msg.note, velocity))
        assert not any(active.values())
    # The published Satie MIDI writes 31 common bars and both eight-bar endings
    # once. Perform its written repeat: common+ending 1, common+ending 2.
    if work == "gymnopedie-1":
        common = [n for n in notes if n[0] < 93]
        first = [n for n in notes if n[0] < 117]
        second = [(n[0] + 117, *n[1:]) for n in common]
        ending = [(n[0] + 93, *n[1:]) for n in notes if n[0] >= 117]
        notes = first + second + ending
    return sorted(notes)


def interpolate(points, x):
    for (a, va), (b, vb) in itertools.pairwise(points):
        if x <= b:
            return va + (vb - va) * max(0, min(1, (x - a) / (b - a)))
    return points[-1][1]


def phrasing(work, beat):
    if work == "clair-de-lune":
        bar = beat / 4.5
        # Tempo rubato, Un poco mosso, En animant, Calmato, a Tempo 1º, morendo.
        bpm = interpolate(
            [
                (0, 65),
                (13, 65),
                (16, 59),
                (25, 63),
                (27, 77),
                (35, 81),
                (39, 83),
                (42, 69),
                (49, 68),
                (51, 64),
                (64, 63),
                (69, 55),
                (72, 46),
            ],
            bar,
        )
        level = interpolate(
            [
                (0, 54),
                (7, 62),
                (13, 57),
                (17, 72),
                (22, 63),
                (26, 68),
                (31, 79),
                (37, 88),
                (40, 94),
                (43, 65),
                (49, 61),
                (51, 53),
                (63, 48),
                (69, 41),
                (72, 34),
            ],
            bar,
        )
        phase = (bar % 4) / 4
    else:
        bar = beat / 3
        bpm = interpolate(
            [
                (0, 72),
                (20, 74),
                (30, 70),
                (38, 60),
                (40, 72),
                (59, 74),
                (70, 70),
                (77, 58),
                (78, 48),
            ],
            bar,
        )
        level = interpolate(
            [
                (0, 51),
                (5, 61),
                (12, 50),
                (21, 65),
                (29, 58),
                (35, 67),
                (39, 46),
                (44, 61),
                (57, 57),
                (65, 69),
                (72, 61),
                (78, 36),
            ],
            bar,
        )
        phase = (bar % 4) / 4
    # A continuous, small breath at phrase ends, not independently jittered beats.
    bpm *= 1 - 0.045 * math.exp(-(((phase - 0.96) / 0.11) ** 2))
    return bpm, level


def clock(work, end):
    grid, elapsed = [0.0], 0.0
    for i in range(math.ceil(end * 192) + 1):
        bpm, _ = phrasing(work, (i + 0.5) / 192)
        elapsed += 60 / bpm / 192
        grid.append(elapsed)

    def at(beat):
        pos = max(0, beat * 192)
        i = int(pos)
        return grid[i] + (grid[i + 1] - grid[i]) * (pos - i)

    return at


def perform(work, offset):
    notes = read(work)
    end = max(n[0] + n[1] for n in notes)
    at = clock(work, end + 6)
    tops, lows = {}, {}
    for beat, duration, part, pitch, velocity in notes:
        if part == 1:
            tops[beat] = max(tops.get(beat, 0), pitch)
        if part == 2:
            lows[beat] = min(lows.get(beat, 127), pitch)
    onsets = sorted({n[0] for n in notes if n[2] == 2})
    events = []
    for i, (beat, duration, part, pitch, source_velocity) in enumerate(notes):
        start, stop = at(beat), at(beat + duration)
        _, level = phrasing(work, beat)
        melody = part == 1 and pitch == tops[beat]
        # The melody speaks over the accompaniment. In Satie's transcription,
        # MIDI dynamics on an accompaniment voice must not bury the top line.
        velocity = level + (8 if melody else -8 if part == 1 else -12)
        velocity += 2.0 * math.sin(i * 1.731)
        if work == "gymnopedie-1" and melody:
            velocity += (source_velocity - 75) * 0.10
        velocity = round(max(29, min(104, velocity)))
        # Gentle half-pedal until the next left-hand harmony, bounded to avoid
        # smearing arpeggios. Natural decay continues during the release envelope.
        next_harmony = bisect.bisect_right(onsets, beat + 0.03)
        pedal = at(onsets[next_harmony]) if next_harmony < len(onsets) else at(end)
        hold = max(stop, min(pedal, stop + (0.65 if work == "clair-de-lune" else 0.9)))
        late = (0.010 if melody else 0.002) + 0.003 * math.sin(i * 2.17)
        events.append(
            (offset + start + late, hold - start, 0.60, 0.0, 0, pitch, velocity, 0)
        )
        bar = beat / (4.5 if work == "clair-de-lune" else 3)
        # Only sustained melodic notes receive a quiet solo-violin response.
        # Leave the opening and fast arpeggios to the pianist.
        violin = (
            (14 <= bar < 26 or 43 <= bar < 50)
            if work == "clair-de-lune"
            else 43 <= bar < 73
        )
        if melody and duration >= 0.85 and violin and 55 <= pitch <= 88:
            events.append(
                (
                    offset + start + 0.025,
                    max(0.15, stop - start - 0.03),
                    0.105,
                    -0.22,
                    1,
                    pitch,
                    velocity,
                    0,
                )
            )
        # Cello only on phrase basses, with no artificial bass octave added.
        cello = (14 <= bar < 50) if work == "clair-de-lune" else 39 <= bar < 76
        if (
            part == 2
            and pitch == lows[beat]
            and duration >= 1.7
            and cello
            and 36 <= pitch <= 59
        ):
            events.append(
                (
                    offset + start + 0.018,
                    min(5.5, stop - start),
                    0.085,
                    0.25,
                    2,
                    pitch,
                    max(35, velocity),
                    0,
                )
            )
    return events, {
        "work": work,
        "start_seconds": offset,
        "duration_seconds": at(end),
        "source_notes_performed": len(notes),
        "arranged_notes": len(events),
        "written_beats": end,
        "piano_velocity_range": [min(e[6] for e in events), max(e[6] for e in events)],
    }


def main():
    for work in json.loads((HERE / "sources.json").read_text()):
        for source in work["files"]:
            data = (HERE / "sources" / source["path"]).read_bytes()
            assert hashlib.sha256(data).hexdigest() == source["sha256"], source["path"]
    events, works, offset = [], [], 0.65
    for work in ["clair-de-lune", "gymnopedie-1"]:
        arranged, report = perform(work, offset)
        events.extend(arranged)
        works.append(report)
        offset += report["duration_seconds"] + 7.0
    events.sort()
    raw = struct.pack("<4sII", b"BAY2", len(events), round(offset * RATE))
    for start, length, gain, pan, instrument, pitch, velocity, reserved in events:
        raw += struct.pack(
            "<IIffBBBB",
            round(start * RATE),
            round(length * RATE),
            gain,
            pan,
            instrument,
            pitch,
            velocity,
            reserved,
        )
    (OUT / "performance.bin").write_bytes(raw)
    (HERE / "performance.json").write_text(
        json.dumps(
            {
                "rate": RATE,
                "seconds": offset,
                "notes": len(events),
                "sha256": hashlib.sha256(raw).hexdigest(),
                "works": works,
            },
            indent=2,
        )
        + "\n"
    )
    print(f"{len(events)} notes, {offset:.2f} seconds, {len(raw)} bytes")


if __name__ == "__main__":
    main()
