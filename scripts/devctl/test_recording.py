"""Isolated local fixtures: no client, viewport, network or Cargo invocation."""

import csv
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import recording
from PIL import Image, ImageDraw


class RecordingTests(unittest.TestCase):
    def fixture(self, directory, phase="complete"):
        root = Path(directory)
        summary = {
            "schema": 1,
            "phase": phase,
            "reason": "duration",
            "options": {"fps": 10, "duration_ms": 500},
            "capture_end_us": 500_000,
            "dropped": 2,
            "discarded": 0,
        }
        frames = []
        for slot, real in [(0, 0), (1, 100_000), (4, 400_000)]:
            name = f"frame-{slot:06}.png"
            with Image.new("RGB", (80, 48), "#161b22") as image:
                ImageDraw.Draw(image).rectangle(
                    (slot * 12, 8, slot * 12 + 16, 32), fill="white"
                )
                image.save(root / name)
            frames.append(
                {
                    "slot": slot,
                    "file": name,
                    "width": 80,
                    "height": 48,
                    "nominal_us": real,
                    "requested_us": real,
                    "virtual_us": 5_000_000 + real // 2,
                    "received_us": real + 23_000,
                }
            )
        (root / "recording.json").write_text(json.dumps(summary))
        (root / "frames.jsonl").write_text(
            "".join(json.dumps(f) + "\n" for f in reversed(frames))
        )
        return summary, frames

    def test_real_delays_preserve_drops_and_final_hold(self):
        with tempfile.TemporaryDirectory() as directory:
            summary, frames = self.fixture(directory)
            self.assertEqual(
                recording.frame_delays(summary, frames, "real", 10), [100, 300, 100]
            )
            self.assertEqual(
                recording.frame_delays(summary, frames, "virtual", 1), [50, 150, 150]
            )

    def test_gif_rounding_preserves_sixty_fps_duration(self):
        summary = {"options": {"fps": 60}, "capture_end_us": 1_000_000}
        frames = [{"requested_us": i * 1_000_000 // 60} for i in range(60)]
        delays = recording.frame_delays(summary, frames, "real", 10)
        self.assertEqual(sum(delays), 1000)
        self.assertEqual(set(delays), {10, 20})

    def test_paused_virtual_time_is_not_faked(self):
        with self.assertRaisesRegex(ValueError, "do not increase"):
            recording.frame_delays(
                {"options": {"fps": 10}},
                [{"virtual_us": 1}, {"virtual_us": 1}],
                "virtual",
                1,
            )

    def test_encode_both_and_sheet_keep_raw_frames(self):
        with tempfile.TemporaryDirectory() as directory:
            _, frames = self.fixture(directory)
            root = Path(directory)
            originals = {f["file"]: (root / f["file"]).read_bytes() for f in frames}
            result = recording.encode(root, output_format="both", width=80)
            self.assertEqual(result["contact_sheet_slots"], [0, 1, 4])
            self.assertEqual(result["dropped"], 2)
            self.assertEqual(len(result["outputs"]), 3)
            for extension in ("gif", "webp"):
                with Image.open(root / f"preview-real.{extension}") as preview:
                    self.assertEqual(preview.n_frames, 3)
                    self.assertEqual(preview.size, (80, 48))
                    # GIF headers carry individual delays; WebP delays are
                    # verified through our explicit conversion timing report.
                    if extension == "gif":
                        delays = []
                        for index in range(preview.n_frames):
                            preview.seek(index)
                            delays.append(preview.info["duration"])
                        self.assertEqual(delays, [100, 300, 100])
            with Image.open(root / "contact-sheet.png") as sheet:
                self.assertEqual(sheet.size, (240, 90))
            for name, content in originals.items():
                self.assertEqual((root / name).read_bytes(), content)

    def test_difference_report_is_zero_for_identical_and_exact_for_changed(self):
        with tempfile.TemporaryDirectory() as directory:
            _, frames = self.fixture(directory)
            root = Path(directory)
            # Same first two frames: counter-test for the heuristic.
            (root / frames[1]["file"]).write_bytes(
                (root / frames[0]["file"]).read_bytes()
            )
            report = recording.encode(root, width=80, differences=True)
            with (root / "differences.csv").open() as source:
                rows = list(csv.DictReader(source))
            self.assertEqual(float(rows[0]["changed_pixel_fraction"]), 0)
            self.assertEqual(float(rows[0]["max_channel_delta"]), 0)
            self.assertGreater(float(rows[1]["changed_pixel_fraction"]), 0)
            self.assertEqual(float(rows[1]["max_channel_delta"]), 255 - 22)
            self.assertIn("not automatic", report["difference_report"])

    def test_unfinished_requires_recovery_opt_in(self):
        with tempfile.TemporaryDirectory() as directory:
            self.fixture(directory, "starting")
            with self.assertRaisesRegex(ValueError, "not drained"):
                recording.load_recording(directory)
            self.assertEqual(len(recording.load_recording(directory, True)[2]), 3)

    def test_path_escape_duplicate_and_negative_time_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            _, frames = self.fixture(directory)
            ledger = Path(directory) / "frames.jsonl"
            for replacement in [
                {**frames[0], "file": "../elsewhere.png"},
                {**frames[0], "requested_us": -1},
            ]:
                ledger.write_text(json.dumps(replacement) + "\n")
                with self.assertRaises(ValueError):
                    recording.load_recording(directory)
            ledger.write_text((json.dumps(frames[0]) + "\n") * 2)
            with self.assertRaisesRegex(ValueError, "duplicate"):
                recording.load_recording(directory)

    def test_recovery_uses_only_valid_prefix_and_reports_skipped_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            _, frames = self.fixture(directory)
            root = Path(directory)
            ledger = root / "frames.jsonl"
            prefix = (json.dumps(frames[0]) + "\n").encode()
            suffixes = [
                b'{"slot":',  # short write
                b'{"slot":"\xff',  # interrupted UTF-8 byte sequence
                json.dumps(frames[1]).encode(),  # no newline commit
                b'{"slot":broken}\n' + (json.dumps(frames[2]) + "\n").encode(),
                b'{"slot":' + (json.dumps(frames[2]) + "\n").encode(),
            ]
            for suffix in suffixes:
                with self.subTest(suffix=suffix):
                    ledger.write_bytes(prefix + suffix)
                    with self.assertRaisesRegex(ValueError, "damaged frame ledger"):
                        recording.load_recording(root)
                    _, summary, recovered = recording.load_recording(root, True)
                    self.assertEqual([f["slot"] for f in recovered], [0])
                    self.assertEqual(
                        summary["ledger_recovery"]["ignored_bytes"], len(suffix)
                    )
                    self.assertEqual(summary["ledger_recovery"]["valid_rows"], 1)
                    report = recording.encode(root, width=80, allow_incomplete=True)
                    self.assertEqual(
                        report["ledger_recovery"], summary["ledger_recovery"]
                    )
                    self.assertEqual(ledger.read_bytes(), prefix + suffix)

    def test_memory_budget_is_enforced_before_adding_frames(self):
        with tempfile.TemporaryDirectory() as directory:
            self.fixture(directory)
            with (
                patch.object(recording, "MAX_PIXELS", 80 * 48),
                self.assertRaisesRegex(ValueError, "smaller --width"),
            ):
                recording.encode(directory, width=80)
            self.assertFalse((Path(directory) / "preview-real.webp").exists())

    def test_sheet_spans_first_and_last_samples_without_duplicates(self):
        self.assertEqual(recording.sheet_indices(1, 12), [0])
        self.assertEqual(recording.sheet_indices(3, 12), [0, 1, 2])
        indices = recording.sheet_indices(900, 12)
        self.assertEqual((indices[0], indices[-1]), (0, 899))
        self.assertEqual(len(set(indices)), 12)


if __name__ == "__main__":
    unittest.main()
