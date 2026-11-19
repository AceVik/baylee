"""What R was worth beside N (`distill_probe_v1`), and the pairwise advantage loss.

A data run (eval4 `--search-rival distill`) plays the raw net, labels each
decision with N (the net's greedy answer) and R (the tactical layer's), and
where R != N at priority plays both on in the same P fair worlds. `convert4
--bundle-distill` writes the result beside the shards, in `bundle-distill/`
(outside the store and its fingerprint, as the coarse classes are), and names
it in meta.json (`bundle_distillation.probe`):
- `probe.index.bin`: one 56-byte little-endian row (`INDEX`) per probed frame,
  sorted by (game, seat, cursor): N's and R's menu entries, the label's subtype
  and fired groups, `mean`, R's mean fork advantage over N in win-probability
  units, `se`, its standard error over the worlds (NaN with one world), and
  what the rollouts did;
- `probe.json`: the header (`contract`, `rows`, the layout, `index.sha256`).
The Rust writer is crates/baylee-train/src/bin/convert4/probes.rs.

`Probes.annotate` gives every decoded frame `probe_n_idx` (N's candidate, -1
where the frame has no probe), `probe_worlds` (0 there), `probe_mean` and
`probe_se`, and fails closed on a frame the sidecar contradicts: a row at a
frame that is no labelled priority decision, R other than the frame's target,
N or R outside the menu or masked, N = R or a frame flagged N = R, another
subtype or other fired groups.

`--distill-advantage pairwise` (`Scope.advantage`, through `distill.distill_loss`)
teaches the student R only where the probes say it pays. At a probed frame with
at least `advantage_min_worlds` worlds, with the student's candidate logits s
and the frozen r1's r: z_s = s[R] - s[N], z_r = r[R] - r[N], and the target
q = sigmoid(z_r + clamp(mean / beta, -clip, clip)); the term is the binary
cross-entropy of sigmoid(z_s) against q, softplus(z_s) - q z_s
(`bce_adv_pairwise`). R ahead of N raises R's log-odds over N, N ahead lowers
it (the net keeps its own good answers), and no advantage leaves r1's odds.
Every other priority frame keeps r1 by the KL, where N = R
(`kl_distilled_n_eq_r`) and where N != R without a usable probe
(`kl_adv_unprobed`); both are reported apart and train as one pooled mean.
Nothing is cross-entropy to R.

`AdvantageCounter` is the held-out side: over probed rows where R leads by more
than one standard error, the share the student (and r1) answers R; where N
leads, the share it answers N; and the correlation between the student's shift
z_s - z_r and the clamped target shift.
"""
from __future__ import annotations

import hashlib
import json
import math
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

from .coarse import key

CONTRACT = "distill_probe_v1"
VERSION = 1
INDEX = np.dtype([("game", "<u4"), ("cursor", "<u4"), ("seat", "u1"), ("subtype", "u1"), ("worlds", "<u2"),
                  ("n_idx", "<u2"), ("r_idx", "<u2"), ("fired", "<u8"), ("mean", "<f4"), ("se", "<f4"),
                  ("half_even", "<f4"), ("half_odd", "<f4"), ("mean_n", "<f4"), ("mean_r", "<f4"),
                  ("terminal", "<u2"), ("stops", "<u2"), ("house_answers", "<u4")])
assert INDEX.itemsize == 56
NO_SUBTYPE = 255   # the label's subtype code where it has none (the frame reads -1)
PRIORITY = 0
MODES = (None, "pairwise")


def named(meta):
    """meta.json's `bundle_distillation.probe`, or None where the dataset has no probe sidecar."""
    value = (meta or {}).get("bundle_distillation")
    probe = value.get("probe") if isinstance(value, dict) else None
    return probe if isinstance(probe, dict) else None


class Probes:
    def __init__(self, index, header=None):
        if len(index) and (int(index["cursor"].max()) >= 1 << 28 or int(index["seat"].max()) >= 16):
            raise ValueError("Probe keys exceed (seat < 16, cursor < 2^28)")
        if len(index) and not np.isfinite(index["mean"]).all():
            raise ValueError("A probe's mean advantage is not finite")
        if len(index) and ((index["worlds"] == 0).any() or (index["n_idx"] == index["r_idx"]).any()):
            raise ValueError("A probe without a world, or whose N is its R")
        self.index, self.header = index, header or {}
        self.keys = key(index["game"], index["seat"], index["cursor"])
        if len(self.keys) > 1 and not (self.keys[1:] > self.keys[:-1]).all():
            raise ValueError("Probe rows are not sorted by (game, seat, cursor) or repeat a key")

    @classmethod
    def load(cls, directory, entry=None):
        """The sidecar in `directory`; `entry` (meta.json's `bundle_distillation.probe`)
        must name the same contract, rows and index digest."""
        directory = Path(directory)
        header = json.loads((directory / "probe.json").read_text())
        if header.get("contract") != CONTRACT or int(header.get("version", 0)) != VERSION:
            raise ValueError(f"Probe sidecar follows {header.get('contract')!r} version {header.get('version')}, "
                             f"not {CONTRACT} version {VERSION}")
        layout = header.get("index") or {}
        try:
            declared = np.dtype([tuple(field) for field in layout.get("dtype") or ()])
        except (TypeError, ValueError) as error:
            raise ValueError("Probe sidecar declares no readable row layout") from error
        if declared != INDEX or int(layout.get("row_bytes", 0)) != INDEX.itemsize:
            raise ValueError("Probe sidecar rows differ from distill_probe_v1's layout")
        raw = (directory / layout.get("file", "probe.index.bin")).read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        if digest != layout.get("sha256"):
            raise ValueError("Probe index differs from its header's SHA-256")
        if len(raw) % INDEX.itemsize or len(raw) // INDEX.itemsize != header.get("rows"):
            raise ValueError("Probe header and index disagree on the row count")
        if entry is not None and (entry.get("contract", CONTRACT) != CONTRACT
                                  or entry.get("index_sha256", digest) != digest
                                  or entry.get("rows", header["rows"]) != header["rows"]):
            raise ValueError("meta.json's bundle_distillation.probe names another probe sidecar")
        return cls(np.frombuffer(raw, dtype=INDEX), header)

    def annotate(self, frame, game, seat, cursor):
        frame["probe_n_idx"] = np.array(-1, dtype=np.int64)
        frame["probe_worlds"] = np.array(0, dtype=np.int64)
        frame["probe_mean"] = np.array(0., dtype=np.float32)
        frame["probe_se"] = np.array(np.nan, dtype=np.float32)
        wanted = key(game, seat, cursor)
        at = int(np.searchsorted(self.keys, wanted))
        if at >= len(self.keys) or self.keys[at] != wanted:
            return frame
        where = f"(game {game}, seat {seat}, cursor {cursor})"
        labelled = (bool(frame["is_decision"]) and float(frame["loss_weight"]) > 0
                    and int(frame["pending_kind"]) == PRIORITY and int(frame["head"]) == 0)
        if not labelled:
            raise ValueError(f"A probe at a frame that is no labelled priority decision {where}")
        row = self.index[at]
        n, r, mask = int(row["n_idx"]), int(row["r_idx"]), frame["cand_mask"]
        if r != int(frame["single_idx"]) or max(n, r) >= len(mask) or not (mask[n] and mask[r]):
            raise ValueError(f"Probe does not match frame {where}: N {n}, R {r}, target {int(frame['single_idx'])}")
        subtype = -1 if int(row["subtype"]) == NO_SUBTYPE else int(row["subtype"])
        if ("n_ne_r" in frame and not bool(frame["n_ne_r"])
                or "distill_subtype" in frame and int(frame["distill_subtype"]) != subtype
                or "fired_mask" in frame and int(frame["fired_mask"]) != int(row["fired"])):
            raise ValueError(f"Probe disagrees with the frame's N != R flag, subtype or fired groups {where}")
        frame["probe_n_idx"] = np.array(n, dtype=np.int64)
        frame["probe_worlds"] = np.array(int(row["worlds"]), dtype=np.int64)
        frame["probe_mean"] = np.array(row["mean"], dtype=np.float32)
        frame["probe_se"] = np.array(row["se"], dtype=np.float32)
        return frame


def pairwise(student, reference, mask, rows, n_idx, r_idx, mean, scope):
    """[B] each of z_s, z_r, the target shift clamp(mean / beta, -clip, clip) and the
    pairwise term softplus(z_s) - sigmoid(z_r + shift) z_s; read only at `rows`."""
    width = student.shape[1]
    pick = lambda index: torch.where(rows, index, 0).clamp(0, width - 1)[:, None]
    n, r = pick(n_idx), pick(r_idx)

    def gap(logits):
        # Masked candidates may hold anything; N and R are on the menu where `rows` is set.
        safe = torch.where(mask, logits, 0.)
        return (safe.gather(1, r) - safe.gather(1, n))[:, 0]

    z_s, z_r = gap(student), gap(reference)
    shift = (torch.where(rows, mean, 0.) / scope.advantage_beta).clamp(-scope.advantage_clip, scope.advantage_clip)
    target = torch.sigmoid(z_r + shift)
    return z_s, z_r, shift, F.softplus(z_s) - target * z_s


def probed_rows(get, t, scope):
    """[B] frames of step t whose probe has at least `advantage_min_worlds` worlds."""
    return get("probe_worlds")[:, t] >= max(1, scope.advantage_min_worlds)


# Held-out counts: probed rows; R ahead, of them the student's and r1's top-1 is R; N ahead,
# of them the student's and r1's top-1 is N; then the shift sums for the correlation and
# the mean shift where R and where N leads.
COUNTS = ("probed", "r_ahead", "r_ahead_student_r", "r_ahead_r1_r", "n_ahead", "n_ahead_student_n",
          "n_ahead_r1_n", "sum_x", "sum_y", "sum_xx", "sum_yy", "sum_xy", "shift_r_ahead", "shift_n_ahead")


class AdvantageCounter:
    """Where the student answers R and N at the held-out probes (`val_adv_*`).

    Rows are labelled priority decisions with a usable probe (as the loss reads
    them). R ahead: mean - se > 0; N ahead: mean + se < 0 (a single world has no
    standard error and counts as neither). The correlation is Pearson's between
    the student's shift z_s - z_r and clamp(mean / beta) over every probed row;
    none where the shift does not vary (an untouched student is r1)."""

    def __init__(self, scope):
        self.scope = scope
        self.totals = None

    def observe(self, prepared, reference, batch, plans):
        if "probe_worlds" not in batch:
            return
        device = prepared[0][4].device
        get = lambda name: batch[name].to(device, non_blocking=True)
        labelled = (get("is_decision") & get("step_valid") & (get("loss_weight") > 0) & (get("head") == 0)
                    & (get("pending_kind") == PRIORITY))
        if self.totals is None:
            self.totals = torch.zeros(len(COUNTS), dtype=torch.float64, device=device)
        for t, plan in enumerate(plans):
            if not plan.heads[0]:
                continue
            student, ref = prepared[t][1].float(), reference[t][1].float()
            mask = get("cand_mask")[:, t, :student.shape[1]]
            rows = labelled[:, t] & (mask.sum(-1) > 1) & probed_rows(get, t, self.scope)
            n_idx, r_idx = get("probe_n_idx")[:, t], get("single_idx")[:, t]
            mean, se = get("probe_mean")[:, t], get("probe_se")[:, t]
            z_s, z_r, shift, _ = pairwise(student, ref, mask, rows, n_idx, r_idx, mean, self.scope)
            top = lambda logits: torch.where(mask, logits, -1e9).argmax(-1)
            mine, theirs = top(student), top(ref)
            r_ahead, n_ahead = rows & (mean - se > 0), rows & (mean + se < 0)
            x = torch.where(rows, z_s - z_r, 0.).double()
            y = torch.where(rows, shift, 0.).double()
            counts = torch.stack((rows.sum(), r_ahead.sum(), (r_ahead & (mine == r_idx)).sum(),
                                  (r_ahead & (theirs == r_idx)).sum(), n_ahead.sum(),
                                  (n_ahead & (mine == n_idx)).sum(), (n_ahead & (theirs == n_idx)).sum()))
            sums = torch.stack((x.sum(), y.sum(), (x * x).sum(), (y * y).sum(), (x * y).sum(),
                                torch.where(r_ahead, x, 0.).sum(), torch.where(n_ahead, x, 0.).sum()))
            self.totals += torch.cat((counts.double(), sums))

    @property
    def seen(self):
        return self.totals is not None

    def metrics(self):
        totals = dict(zip(COUNTS, (self.totals.cpu().tolist() if self.totals is not None else [0.] * len(COUNTS))))
        share = lambda hits, rows: hits / rows if rows else None
        n = totals["probed"]
        corr = None
        if n >= 2:
            mx, my = totals["sum_x"] / n, totals["sum_y"] / n
            vx, vy = totals["sum_xx"] / n - mx * mx, totals["sum_yy"] / n - my * my
            if vx > 1e-12 and vy > 1e-12:
                corr = max(-1., min(1., (totals["sum_xy"] / n - mx * my) / math.sqrt(vx * vy)))
        out = dict(val_adv_probed_rows=int(n),
                   val_adv_r_ahead_rows=int(totals["r_ahead"]),
                   val_adv_r_ahead_top1_r=share(totals["r_ahead_student_r"], totals["r_ahead"]),
                   val_adv_r_ahead_top1_r_r1=share(totals["r_ahead_r1_r"], totals["r_ahead"]),
                   val_adv_n_ahead_rows=int(totals["n_ahead"]),
                   val_adv_n_ahead_top1_n=share(totals["n_ahead_student_n"], totals["n_ahead"]),
                   val_adv_n_ahead_top1_n_r1=share(totals["n_ahead_r1_n"], totals["n_ahead"]),
                   val_adv_shift_corr=corr,
                   val_adv_shift_mean_r_ahead=share(totals["shift_r_ahead"], totals["r_ahead"]),
                   val_adv_shift_mean_n_ahead=share(totals["shift_n_ahead"], totals["n_ahead"]))
        sides = [value for value in (out["val_adv_r_ahead_top1_r"], out["val_adv_n_ahead_top1_n"])
                 if value is not None]
        # `--checkpoint-selection distill_advantage`: the sides with rows (the same in every
        # validation: the rows depend on the data, not on the student).
        out["val_adv_selection"] = sum(sides) / len(sides) if sides else None
        return out
