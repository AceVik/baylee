"""Coarse classes of priority answers (`coarse_class_v1`), from the dataset's sidecar.

The data side writes, beside the shards in `bundle-distill/` (outside the store
and its fingerprint, so a window-state cache stays valid):
- `coarse-classes.index.bin`: one row per labelled priority frame of a recorded
  seat, sorted by (game, seat, cursor), dtype `INDEX`;
- `coarse-classes.classes.bin`: per candidate, in menu order, one u8 class
  (`CLASSES`), row i's at `[offset, offset + n_cand)`;
- `coarse-classes.json`: the header (`rule`, `rows`, counts).

A mana ability taken through ActivateAbility is `tap` and a CancelCast is
`pass` there, which the candidate features alone cannot tell. `annotate` gives
every decoded frame `coarse_class` (one per candidate, -1 where the frame has
none) and `coarse_n_idx` (N's candidate, -1 likewise), and fails closed on a
labelled priority frame the sidecar does not cover or does not match.

Version 2 (house teacher) adds `coarse-classes.played.bin`, one `PLAYED` row per
index row: what the seat played there, its class, and whether the priority was
free (no cost owed). Then `annotate` also gives `coarse_p_class` (-1 for none)
and `coarse_free`, so the probe can set the behaviour beside H at the same states.
"""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

RULE = "coarse_class_v1"
CLASSES = ("pass", "tap", "cast", "activate", "land", "other")
INDEX = np.dtype([("game", "<u4"), ("cursor", "<u4"), ("seat", "u1"), ("r_class", "u1"), ("n_class", "u1"),
                  ("subtype", "u1"), ("r_idx", "<u4"), ("n_idx", "<u4"), ("n_cand", "<u4"), ("offset", "<u8")])
PRIORITY = 0


CHAIN_ACTS = (CLASSES.index("tap"), CLASSES.index("cast"), CLASSES.index("activate"))


def chain_starts(index):
    """Per row: R starts an acting chain there, a tap, cast or activation after R's
    last labelled priority answer in the trajectory was a pass (or at its first);
    land drops and other answers are neutral (`distill.ChainCounter`'s rule)."""
    starts = np.zeros(len(index), dtype=bool)
    group = (index["game"].astype(np.uint64) << np.uint64(32)) | index["seat"].astype(np.uint64)
    previous, acting = None, False
    for row, (where, answer) in enumerate(zip(group.tolist(), index["r_class"].tolist())):
        if where != previous:
            previous, acting = where, False
        if answer in CHAIN_ACTS:
            starts[row], acting = not acting, True
        elif answer == CLASSES.index("pass"):
            acting = False
    return starts


def key(game, seat, cursor):
    """One sortable u64 per (game, seat, cursor): the sidecar's own order."""
    return (np.asarray(game, np.uint64) << np.uint64(32)) | (np.asarray(seat, np.uint64) << np.uint64(28)) \
        | np.asarray(cursor, np.uint64)


# Version 2 (house teacher): per index row, what the seat played (the record's own
# answer before H replaced the label; u32::MAX for none), its coarse class (255 for
# none) and whether the priority was free (no cost owed, eval4's `free`).
PLAYED = np.dtype([("p_idx", "<u4"), ("p_class", "u1"), ("free", "u1"), ("pad", "V2")])


class CoarseClasses:
    def __init__(self, index, classes, header=None, played=None):
        if len(index) and (int(index["cursor"].max()) >= 1 << 28 or int(index["seat"].max()) >= 16):
            raise ValueError("Coarse class keys exceed (seat < 16, cursor < 2^28)")
        if played is not None and len(played) != len(index):
            raise ValueError("Coarse class played rows and index rows differ")
        self.index, self.classes, self.header, self.played = index, classes, header or {}, played
        self.keys = key(index["game"], index["seat"], index["cursor"])
        if len(self.keys) > 1 and not (self.keys[1:] > self.keys[:-1]).all():
            raise ValueError("Coarse class rows are not sorted by (game, seat, cursor) or repeat a key")
        self.chain_start = chain_starts(index)

    @classmethod
    def load(cls, directory):
        directory = Path(directory)
        header = json.loads((directory / "coarse-classes.json").read_text())
        if header.get("rule") != RULE:
            raise ValueError(f"Coarse classes follow {header.get('rule')!r}, not {RULE}")
        if list(header.get("classes", CLASSES)) != list(CLASSES):
            raise ValueError("Coarse class names or order differ")
        index = np.fromfile(directory / "coarse-classes.index.bin", dtype=INDEX)
        if header.get("rows", len(index)) != len(index):
            raise ValueError("Coarse class header and index disagree on the row count")
        classes = np.memmap(directory / "coarse-classes.classes.bin", dtype=np.uint8, mode="r")
        played = None
        if int(header.get("version", 1)) >= 2 and header.get("played_file"):
            played = np.fromfile(directory / header["played_file"].get("file", "coarse-classes.played.bin"),
                                 dtype=PLAYED)
        return cls(index, classes, header, played)

    def annotate(self, frame, game, seat, cursor):
        width = len(frame["cand_mask"])
        frame["coarse_class"] = np.full(width, -1, dtype=np.int64)
        frame["coarse_n_idx"] = np.array(-1, dtype=np.int64)
        frame["coarse_chain_start"] = np.array(False)
        if self.played is not None:
            frame["coarse_p_class"] = np.array(-1, dtype=np.int64)
            frame["coarse_free"] = np.array(False)
        labelled = (bool(frame["is_decision"]) and float(frame["loss_weight"]) > 0
                    and int(frame["pending_kind"]) == PRIORITY and int(frame["head"]) == 0)
        if not labelled:
            return frame
        wanted = key(game, seat, cursor)
        at = int(np.searchsorted(self.keys, wanted))
        if at >= len(self.keys) or self.keys[at] != wanted:
            raise ValueError(f"No coarse classes for labelled priority frame (game {game}, seat {seat}, cursor {cursor})")
        row = self.index[at]
        n = int(row["n_cand"])
        if n != width or int(row["r_idx"]) != int(frame["single_idx"]) or int(row["n_idx"]) >= n:
            raise ValueError(f"Coarse classes do not match frame (game {game}, seat {seat}, cursor {cursor})")
        classes = np.asarray(self.classes[int(row["offset"]):int(row["offset"]) + n], dtype=np.int64)
        if classes[int(row["r_idx"])] != row["r_class"] or classes[int(row["n_idx"])] != row["n_class"]:
            raise ValueError(f"Coarse class rows disagree with their classes (game {game}, cursor {cursor})")
        frame["coarse_class"] = classes
        frame["coarse_n_idx"] = np.array(int(row["n_idx"]), dtype=np.int64)
        frame["coarse_chain_start"] = np.array(bool(self.chain_start[at]))
        if self.played is not None:
            played = self.played[at]
            frame["coarse_p_class"] = np.array(-1 if int(played["p_class"]) == 255 else int(played["p_class"]),
                                               dtype=np.int64)
            frame["coarse_free"] = np.array(bool(played["free"]))
        return frame


def main(argv=None):
    """Join a dataset's frames with its sidecar and count what joined (fails on a mismatch)."""
    import argparse
    import dataset4 as D4
    from .distill import check_datasets
    parser = argparse.ArgumentParser(description=main.__doc__)
    parser.add_argument("--data", required=True)
    parser.add_argument("--split", default="val")
    parser.add_argument("--every", type=int, default=50, help="Every k-th trajectory")
    parser.add_argument("--encoder-version", type=int, default=10)
    args = parser.parse_args(argv)
    dataset = D4.TrajectoryDataset4(Path(args.data).expanduser(), split=args.split, unroll_len=32,
                                    encoder_version=args.encoder_version)
    check_datasets([dataset], True)
    dataset.coarse = CoarseClasses.load(Path(args.data).expanduser() / "bundle-distill")
    joined, frames, r_classes, n_classes, differ = 0, 0, np.zeros(len(CLASSES), int), np.zeros(len(CLASSES), int), 0
    for trajectory in dataset.trajectories[::args.every]:
        for segment in trajectory:
            for row in dataset.segments[segment]:
                frame = dataset.encode_single_step(int(row))
                frames += 1
                n_idx = int(frame["coarse_n_idx"])
                if n_idx >= 0:
                    joined += 1
                    r_class = int(frame["coarse_class"][int(frame["single_idx"])])
                    r_classes[r_class] += 1
                    n_classes[int(frame["coarse_class"][n_idx])] += 1
                    differ += r_class != int(frame["coarse_class"][n_idx])
    print(json.dumps(dict(split=args.split, trajectories=len(dataset.trajectories[::args.every]), frames=frames,
                          joined=joined, class_differs=differ, r=dict(zip(CLASSES, r_classes.tolist())),
                          n=dict(zip(CLASSES, n_classes.tolist())))), flush=True)


if __name__ == "__main__":
    main()
