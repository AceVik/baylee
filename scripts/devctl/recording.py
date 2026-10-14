#!/usr/bin/env python3
# /// script
# requires-python = ">=3.9"
# dependencies = ["Pillow>=11,<12"]
# ///
"""Local dev-control capture/encoding. Raw PNGs and metadata are never modified.

python3 scripts/devctl/recording.py record --duration-ms 3000 --fps 15
python3 scripts/devctl/recording.py encode /tmp/baylee-recording-... --format both

Pillow is an offline conversion dependency only; alternatively uv run this file.
No desktop capture, audio, uploads, or generated imagery are involved.
"""

import argparse
import csv
import json
import math
import re
import sys
import time
import urllib.request
from pathlib import Path

MAX_PIXELS = 64 * 1024 * 1024  # decoded preview frames: at most 256 MiB RGBA
MAX_FRAME_PIXELS = 8 * 1024 * 1024
MAX_FRAMES = 900
TERMINAL = {"complete", "cancelled", "failed"}


def read_json(path, limit=1024 * 1024):
    if path.stat().st_size > limit:
        raise ValueError(f"metadata exceeds {limit} bytes: {path}")
    return json.loads(path.read_text())


def load_recording(directory, allow_incomplete=False):
    directory = Path(directory).resolve(strict=True)
    summary = read_json(directory / "recording.json")
    if summary.get("schema") != 1:
        raise ValueError("unsupported recording schema")
    if summary.get("phase") not in TERMINAL and not allow_incomplete:
        raise ValueError(
            "recording has not drained; poll status or use --allow-incomplete to recover"
        )
    ledger = directory / "frames.jsonl"
    if ledger.stat().st_size > 2 * 1024 * 1024:
        raise ValueError("frame ledger exceeds 2 MiB")
    frames = []
    data = ledger.read_bytes()
    boundary = 0
    for line in data.splitlines(keepends=True):
        try:
            if not line.endswith(b"\n"):
                raise ValueError("unterminated ledger row")
            frame = json.loads(line)
        except (ValueError, UnicodeDecodeError) as error:
            if not allow_incomplete:
                raise ValueError(
                    "damaged frame ledger; use --allow-incomplete for its valid prefix"
                ) from error
            summary = {
                **summary,
                "ledger_recovery": {
                    "valid_rows": len(frames),
                    "ignored_bytes": len(data) - boundary,
                    "note": "Stopped at the first damaged/unterminated row; later bytes were not interpreted.",
                },
            }
            break
        boundary += len(line)
        name = frame.get("file", "")
        if not re.fullmatch(r"frame-\d{6}\.png", name):
            raise ValueError("unexpected frame filename")
        if (directory / name).resolve(strict=True).parent != directory:
            raise ValueError("frame must be inside its recording directory")
        for field in (
            "slot",
            "nominal_us",
            "requested_us",
            "virtual_us",
            "received_us",
        ):
            if type(frame.get(field)) is not int or frame[field] < 0:
                raise ValueError(f"invalid {field}")
        if frame["slot"] >= MAX_FRAMES or name != f"frame-{frame['slot']:06}.png":
            raise ValueError("invalid frame slot")
        frames.append(frame)
        if len(frames) > MAX_FRAMES:
            raise ValueError("too many frames")
    frames.sort(key=lambda f: f["slot"])
    if not frames:
        raise ValueError("recording contains no saved frames")
    if len({f["slot"] for f in frames}) != len(frames):
        raise ValueError("duplicate frame slots")
    return directory, summary, frames


def frame_delays(summary, frames, cadence, quantum_ms):
    """Preserve gaps in the chosen timeline; never infer presentation times."""
    key = {"real": "requested_us", "virtual": "virtual_us", "nominal": "nominal_us"}[
        cadence
    ]
    times = [f[key] for f in frames]
    differences = [b - a for a, b in zip(times, times[1:])]  # noqa: RUF007 - macOS Python 3.9
    if any(delta <= 0 for delta in differences):
        raise ValueError(
            f"{cadence} times do not increase; use real or nominal cadence"
        )
    fps = summary["options"]["fps"]
    if type(fps) is not int or not 1 <= fps <= 60:
        raise ValueError("invalid nominal fps")
    last = differences[-1] if differences else round(1_000_000 / fps)
    if cadence == "real" and summary.get("capture_end_us") is not None:
        last = max(1, summary["capture_end_us"] - times[-1])
    differences.append(last)
    if any(delta > 60_000_000 for delta in differences):
        raise ValueError("a frame delay exceeds 60 seconds")
    # Quantize cumulative deadlines, so rounding at 60 fps does not turn a
    # second into 1.2 s. Minimum one format tick; recorded values remain intact.
    quantum_us = quantum_ms * 1000
    deadlines = []
    elapsed = 0
    emitted_ticks = 0
    for delta in differences:
        elapsed += delta
        target = max(emitted_ticks + 1, (elapsed + quantum_us // 2) // quantum_us)
        deadlines.append((target - emitted_ticks) * quantum_ms)
        emitted_ticks = target
    return deadlines


def difference_report(directory, frames):
    """Exact RGB differences, two raw frames at a time; a heuristic, not bug proof."""
    from PIL import Image, ImageChops, ImageStat

    destination = directory / "differences.csv"
    previous = None
    previous_meta = None
    try:
        with destination.open("w", newline="") as output:
            writer = csv.DictWriter(
                output,
                fieldnames=[
                    "previous_slot",
                    "slot",
                    "request_gap_ms",
                    "virtual_gap_ms",
                    "max_channel_delta",
                    "mean_absolute_channel_delta",
                    "changed_pixel_fraction",
                ],
            )
            writer.writeheader()
            for frame in frames:
                with Image.open(directory / frame["file"]) as raw:
                    if raw.width * raw.height > MAX_FRAME_PIXELS:
                        raise ValueError("raw frame exceeds recorder's pixel budget")
                    current = raw.convert("RGB")
                if previous is not None:
                    if current.size != previous.size:
                        current.close()
                        raise ValueError("frame dimensions changed")
                    diff = ImageChops.difference(previous, current)
                    channels = diff.split()
                    union = ImageChops.lighter(channels[0], channels[1])
                    mask = ImageChops.lighter(union, channels[2])
                    changed = 1 - mask.histogram()[0] / (diff.width * diff.height)
                    writer.writerow(
                        {
                            "previous_slot": previous_meta["slot"],
                            "slot": frame["slot"],
                            "request_gap_ms": (
                                frame["requested_us"] - previous_meta["requested_us"]
                            )
                            / 1000,
                            "virtual_gap_ms": (
                                frame["virtual_us"] - previous_meta["virtual_us"]
                            )
                            / 1000,
                            "max_channel_delta": max(
                                high for _, high in diff.getextrema()
                            ),
                            "mean_absolute_channel_delta": sum(
                                ImageStat.Stat(diff).mean
                            )
                            / 3,
                            "changed_pixel_fraction": changed,
                        }
                    )
                    for temporary in (*channels, union, mask, diff, previous):
                        temporary.close()
                previous, previous_meta = current, frame
    finally:
        if previous is not None:
            previous.close()
    return str(destination)


def sheet_indices(count, requested):
    take = min(count, requested)
    if take == 1:
        return [0]
    return [round(i * (count - 1) / (take - 1)) for i in range(take)]


def encode(
    directory,
    *,
    output_format="webp",
    width=640,
    cadence="real",
    sheet_frames=12,
    allow_incomplete=False,
    differences=False,
):
    from PIL import Image, ImageDraw, features

    if not 16 <= width <= 1920 or not 1 <= sheet_frames <= 48:
        raise ValueError("width must be 16..1920 and sheet-frames 1..48")
    formats = ["gif", "webp"] if output_format == "both" else [output_format]
    if "webp" in formats and not features.check("webp"):
        raise ValueError("this Pillow has no WebP encoder; use --format gif")
    directory, summary, frames = load_recording(directory, allow_incomplete)
    diff_path = difference_report(directory, frames) if differences else None
    images = []
    pixel_count = 0
    canvas = None
    try:
        for frame in frames:
            with Image.open(directory / frame["file"]) as raw:
                if raw.width * raw.height > MAX_FRAME_PIXELS:
                    raise ValueError("raw frame exceeds recorder's pixel budget")
                if canvas is None:
                    target_width = min(width, raw.width)
                    canvas = (
                        target_width,
                        max(1, round(raw.height * target_width / raw.width)),
                    )
                pixel_count += canvas[0] * canvas[1]
                if pixel_count > MAX_PIXELS:
                    raise ValueError(
                        "preview exceeds 64M pixels; use a smaller --width (raw frames are kept)"
                    )
                if (raw.width, raw.height) != (frames[0]["width"], frames[0]["height"]):
                    raise ValueError("frame dimensions changed")
                images.append(
                    raw.convert("RGB").resize(canvas, Image.Resampling.LANCZOS)
                )
        outputs = [diff_path] if diff_path else []
        timing = {}
        for fmt in formats:
            delays = frame_delays(summary, frames, cadence, 10 if fmt == "gif" else 1)
            destination = directory / f"preview-{cadence}.{fmt}"
            extra = (
                {"lossless": True, "method": 4}
                if fmt == "webp"
                else {"optimize": False, "disposal": 2}
            )
            images[0].save(
                destination,
                format=fmt.upper(),
                save_all=True,
                append_images=images[1:],
                duration=delays,
                loop=0,
                **extra,
            )
            outputs.append(str(destination))
            timing[fmt] = {"duration_ms": delays, "total_ms": sum(delays)}
        indices = sheet_indices(len(frames), sheet_frames)
        columns = min(4, len(indices))
        tile_width = min(320, canvas[0])
        tile_height = max(1, round(canvas[1] * tile_width / canvas[0]))
        cell_height = tile_height + 42
        sheet = Image.new(
            "RGB",
            (columns * tile_width, math.ceil(len(indices) / columns) * cell_height),
            "#161b22",
        )
        draw = ImageDraw.Draw(sheet)
        for ordinal, index in enumerate(indices):
            x, y = ordinal % columns * tile_width, ordinal // columns * cell_height
            tile = images[index].resize(
                (tile_width, tile_height), Image.Resampling.LANCZOS
            )
            sheet.paste(tile, (x, y))
            tile.close()
            f = frames[index]
            label = f"slot {f['slot']}  request {f['requested_us'] / 1e6:.3f}s\nvirtual {f['virtual_us'] / 1e6:.3f}s  delivery {f['received_us'] / 1e6:.3f}s"
            draw.text((x + 5, y + tile_height + 4), label, fill="white")
        contact = directory / "contact-sheet.png"
        sheet.save(contact)
        sheet.close()
        outputs.append(str(contact))
        report = {
            "source": str(directory),
            "saved_frames": len(frames),
            "phase": summary.get("phase"),
            "ledger_recovery": summary.get("ledger_recovery"),
            "reason": summary.get("reason"),
            "dropped": summary.get("dropped"),
            "discarded": summary.get("discarded"),
            "cadence": cadence,
            "timing": timing,
            "contact_sheet_slots": [frames[i]["slot"] for i in indices],
            "difference_report": "Consecutive raw RGB deltas over the full viewport; heuristic only, not automatic flicker/bug proof. Legitimate motion also changes pixels."
            if differences
            else None,
            "note": "Real=request timestamp, virtual=game clock, nominal=requested FPS. No presentation timestamps. GIF quantizes to 10 ms; WebP to 1 ms. Previews may merge identical frames; raw PNGs and frames.jsonl retain all saved samples. Preview scaling/palette changes are unsuitable for exact pixel diffs.",
            "outputs": outputs,
        }
        (directory / f"preview-{cadence}.json").write_text(
            json.dumps(report, indent=2) + "\n"
        )
        return report
    finally:
        for image in images:
            image.close()


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError("dev-control must not redirect")


def request(port, endpoint, body=None):
    if not 1 <= port <= 65535:
        raise ValueError("port must be 1..65535")
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/recording/{endpoint}",
        data=None if body is None else json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
    )
    with opener.open(req, timeout=15) as response:
        payload = response.read(1024 * 1024 + 1)
    if len(payload) > 1024 * 1024:
        raise ValueError("status response exceeds 1 MiB")
    result = json.loads(payload)
    if "error" in result:
        raise ValueError(result["error"])
    return result


def record(args):
    result = request(
        args.port, "start", {"duration_ms": args.duration_ms, "fps": args.fps}
    )
    run = result["recording"]
    print(f"Local frames: {run['directory']}", file=sys.stderr)
    deadline = time.monotonic() + args.duration_ms / 1000 + 30
    try:
        while run["phase"] not in TERMINAL:
            if time.monotonic() >= deadline:
                request(args.port, "stop", {})
                raise ValueError(
                    "drain timed out; inspect status and the local frame ledger"
                )
            time.sleep(0.2)
            run = request(args.port, "status")["recording"]
    except KeyboardInterrupt:
        request(args.port, "stop", {"cancel": True})
        print(
            "Cancelled; saved frames remain local. Poll status before encoding.",
            file=sys.stderr,
        )
        raise
    if run["phase"] == "failed":
        raise ValueError(
            f"recording failed: {run.get('error')}; frames: {run['directory']}"
        )
    return encode(
        run["directory"],
        output_format=args.format,
        width=args.width,
        cadence=args.cadence,
        sheet_frames=args.sheet_frames,
        differences=args.diff,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    capture = sub.add_parser(
        "record", help="record real render frames and encode after drain"
    )
    capture.add_argument("--duration-ms", type=int, default=3000)
    capture.add_argument("--fps", type=int, default=15)
    conversion = sub.add_parser(
        "encode", help="encode existing local PNGs and timing metadata"
    )
    conversion.add_argument("directory", type=Path)
    conversion.add_argument("--allow-incomplete", action="store_true")
    for command in (capture, conversion):
        command.add_argument(
            "--format", choices=("gif", "webp", "both"), default="webp"
        )
        command.add_argument("--width", type=int, default=640)
        command.add_argument(
            "--cadence", choices=("real", "virtual", "nominal"), default="real"
        )
        command.add_argument("--sheet-frames", type=int, default=12)
        command.add_argument(
            "--diff",
            action="store_true",
            help="write heuristic raw RGB differences.csv",
        )
    status = sub.add_parser("status")
    stop = sub.add_parser("stop")
    stop.add_argument("--cancel", action="store_true")
    for command in (capture, status, stop):
        command.add_argument("--port", type=int, default=28770)
    args = parser.parse_args()
    try:
        if args.command == "record":
            result = record(args)
        elif args.command == "encode":
            result = encode(
                args.directory,
                output_format=args.format,
                width=args.width,
                cadence=args.cadence,
                sheet_frames=args.sheet_frames,
                allow_incomplete=args.allow_incomplete,
                differences=args.diff,
            )
        else:
            result = request(
                args.port,
                args.command,
                {"cancel": args.cancel} if args.command == "stop" else None,
            )
        print(json.dumps(result, indent=2))
    except (OSError, ValueError, KeyError, ImportError) as error:
        parser.exit(1, f"recording: {error}\n")


if __name__ == "__main__":
    main()
