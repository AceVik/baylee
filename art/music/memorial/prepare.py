"""Build the memorial performance from pinned OpenScore MIDI, never a recording.
Run: uv run --with mido==1.3.3 art/music/memorial/prepare.py
MIDI exports: MuseScore 4.4.4, using the checked-in complete MSCX editions.
"""

import hashlib
import json
import math
import struct
from pathlib import Path

import mido

HERE = Path(__file__).resolve().parent
OUT = HERE.parents[2] / "crates/baylee-client-core/src/music/score/memorial"
RATE = 48000
WORKS = [
    ("brahms-wiegenlied", 1.12),
    ("schumann-mondnacht", 1.0),
    ("schubert-baches-wiegenlied", 1.04),
]
events, report, offset = [], [], 0.65
for work, stretch in WORKS:
    midi = mido.MidiFile(HERE / "sources" / f"{work}.mid")
    timeline = []
    for track, messages in enumerate(midi.tracks):
        tick = 0
        for message in messages:
            tick += message.time
            timeline.append((tick, track, message))
    timeline.sort(key=lambda row: row[0])
    tempo, old_tick, seconds = 500000, 0, 0.0
    active, notes = {}, []
    for tick, track, msg in timeline:
        seconds += (
            mido.tick2second(tick - old_tick, midi.ticks_per_beat, tempo) * stretch
        )
        old_tick = tick
        if msg.type == "set_tempo":
            tempo = msg.tempo
        elif msg.type in ("note_on", "note_off"):
            key = (track, msg.channel, msg.note)
            if msg.type == "note_on" and msg.velocity:
                assert key not in active, (work, key)
                active[key] = (seconds, msg.velocity)
            elif key in active:
                start, velocity = active.pop(key)
                notes.append((start, seconds - start, track, msg.note, velocity))
    assert not active
    notes.sort()
    for i, (start, duration, part, pitch, velocity) in enumerate(notes):
        # The written melody is carried by a solo voice; both piano staves
        # retain every pitch and rest. Timbre changes only between strophes.
        if part == 0:
            instrument = (
                3 if work != "schubert-baches-wiegenlied" or int(start / 68) % 2 else 13
            )
            gain, pan = (0.32 if instrument == 13 else 0.34), -0.10
        elif part == 1:
            instrument = 11 if work == "schumann-mondnacht" else 0
            gain, pan = (
                (0.17 if instrument == 11 else 0.20),
                -0.30 + (pitch - 60) * 0.008,
            )
        else:
            instrument = 11 if work == "schumann-mondnacht" else 5
            gain, pan = (0.15 if instrument == 11 else 0.14), 0.22
        # Keep the edition's dynamics and rubato; add a restrained four-bar
        # breath and gentle asymmetric touch, reproducibly, without randomness.
        arc = 0.94 + 0.06 * math.sin(start * math.pi / 8)
        gain *= (velocity / 80) ** 1.2 * arc
        delay = 0.006 + 0.003 * math.sin(i * 1.73) + (0.005 if part == 0 else 0)
        frame = round((offset + start + delay) * RATE)
        length = round(max(0.04, duration) * RATE)
        events.append((frame, length, gain, pan, instrument, pitch, 0))
    end = max(start + duration for start, duration, *_ in notes)
    report.append(
        {
            "work": work,
            "start_seconds": offset,
            "duration_seconds": end,
            "notes": len(notes),
            "tempo_scale": stretch,
            "midi_sha256": hashlib.sha256(
                (HERE / "sources" / f"{work}.mid").read_bytes()
            ).hexdigest(),
        }
    )
    offset += end + 5.5

events.sort(key=lambda row: row[0])
end_frame = round(offset * RATE)
raw = struct.pack("<4sII", b"BAY1", len(events), end_frame)
raw += b"".join(struct.pack("<IIffBBH", *event) for event in events)
(OUT / "performance.bin").write_bytes(raw)
(HERE / "performance.json").write_text(
    json.dumps(
        {
            "rate": RATE,
            "seconds": offset,
            "notes": len(events),
            "sha256": hashlib.sha256(raw).hexdigest(),
            "works": report,
        },
        indent=2,
    )
    + "\n"
)
print(f"{len(events)} notes, {offset:.2f} seconds, {len(raw)} bytes")
