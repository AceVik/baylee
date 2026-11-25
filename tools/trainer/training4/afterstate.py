"""Arm V of the search distillation: the net judges an answer by the position it leads to.

    python -m training4.afterstate capture --rows rows-c1.npz --state-dir STATES_C1 \
        --afterstates AFTER_C1 --model R1_DIR --out afterstates-c1.npz --device cuda
    python -m training4.afterstate fit-v0 --rows ... --afterstates afterstates-c1.npz ... --out V0 \
        --preregistration PREREG.json --seed 1
    python -m training4.afterstate fit-v1 --rows ... --afterstates ... --state-dir ... \
        --after-data AFTER_C1 ... --model R1_DIR --out V1 --device cuda --preregistration PREREG.json
    python -m training4.afterstate decide --preregistration PREREG.json --stage V0=DIR --stage V1=DIR \
        --rows ... --afterstates ... --out decision.json

Root-state heads (arm B) read the root and a candidate token; no path computed the board an
answer leads to. Here the engine imagines it: `convert4 --search-afterstates` forks the table
at each searched root, plays every searched candidate as the search played it (the planner
pays), stops at the next question and keeps the seat's observation of that position where it
depends on no dealt hidden card (`baylee_train::convert4::afterstates`, `search_afterstate_v1`).
The net evaluates each afterstate s'_i from the root's stored memory: it observes s'_i (K = 0),
thinks twice (K = 2) and reads its side value V(s'_i). The head's gap is q_i - q_own, the
same quantity arm B's report reads, so the held-out halves, the margin, the gain and its SE by
game are arm B's (`training4.advantage.report`).

- V0: r1 frozen; q_i = MLP([e(s'_i), V(s'_i), V_K0(s'_i), coarse class, payable-added]).
- V1: a trainable tower, a copy of r1 (r1 untouched): trunk layers 9-12, trunk_norm, h_norm,
  thinking_block and seat_value_head train; q_i = alpha * V_tower(s'_i).

A candidate without a valid afterstate scores gap 0, and so does every candidate of a root
whose own afterstate is missing. Creativity: a candidate whose coarse class differs from the
own answer's (a tap merged with the spell it pays for) and whose world-by-world lead is
confirmed on one half of the worlds (mean - 1.64 SE > 0.03) and positive on the other weighs
twice in the loss (training roots only); the target is the measured advantage clipped to
+-0.5. Each fit writes its selection-half numbers and keeps its gaps sealed for `decide`, which
alone reads the report half, once, under the pre-registration
(`v6t-campaign/afterstate/preregistration-afterstate-armv.json`).
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path
import time

import numpy as np
import torch

from . import advantage as V
from .coarse import CLASSES

ROWS_CONTRACT = "search_afterstate_rows_v1"
FIT_CONTRACT = "search_afterstate_fit_v1"
DECISION_CONTRACT = "search_afterstate_decision_v1"
PREREGISTRATION_CONTRACT = "searchadv_armv_preregistration_v1"
SIDECAR_CONTRACT = "search_afterstate_v1"
FRAMES_SPLIT = "search-afterstate-frames"
SHARDS_KEY = "search_afterstates_shards"
SIDECAR_INDEX = np.dtype([("game", "<u4"), ("root_cursor", "<u4"), ("item", "<u4"), ("frame_cursor", "<u4"),
                          ("seat", "u1"), ("candidate", "u1"), ("stop", "u1"), ("flags", "u1"),
                          ("answers", "<u2"), ("next_kind", "u1"), ("worlds", "u1"),
                          ("even_mean", "<f4"), ("even_se", "<f4"), ("odd_mean", "<f4"), ("odd_se", "<f4"),
                          ("card", "<u4"), ("pad", "V4")])
VALID = 1
NONE = 0xFFFFFFFF
TAP = CLASSES.index("tap")
CAST = CLASSES.index("cast")
CONFIRM_Z = 1.64
CONFIRM_MARGIN = 0.03
NOVELTY = 2.0
CLIP = 0.5
TAU = 0.02
THINKING_STEPS = 2
# Trunk layers 9-12 (0-based 8-11) and what reads them in the tower; recurrent_cell and every
# layer below stay r1's.
UPPER_FROM = 8
V1_MODULES = ("seat_value_head", "h_norm", "thinking_block", "trunk_layers.8", "trunk_layers.9",
              "trunk_layers.10", "trunk_layers.11", "trunk_norm")
PER_CANDIDATE = ("valid", "candidate", "stop", "card", "halves", "frame_cursor")


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class Sidecar:
    """`<dataset>/search-afterstates/`: one row per searched candidate of each labelled root,
    read fail-closed against its header."""

    def __init__(self, index, header, directory):
        self.index, self.header, self.directory = index, header, directory

    @classmethod
    def load(cls, dataset):
        directory = Path(dataset) / "search-afterstates"
        header = json.loads((directory / "header.json").read_text())
        if header.get("contract") != SIDECAR_CONTRACT:
            raise ValueError(f"{directory} holds {header.get('contract')!r}, not {SIDECAR_CONTRACT}")
        part = header["index"]
        if np.dtype([tuple(f) for f in part["dtype"]]) != SIDECAR_INDEX or part["row_bytes"] != SIDECAR_INDEX.itemsize:
            raise ValueError("Afterstate index has another layout")
        raw = (directory / "index.bin").read_bytes()
        if hashlib.sha256(raw).hexdigest() != part["sha256"]:
            raise ValueError("Afterstate index does not match its header's digest")
        index = np.frombuffer(raw, dtype=SIDECAR_INDEX)
        if len(index) != part["rows"]:
            raise ValueError("Afterstate header and index disagree on the row count")
        return cls(index, header, directory)

    def join(self, rows):
        """Per candidate of `rows` (one source): the afterstate's fields, each searched
        candidate exactly once; `valid` False and `candidate` -1 elsewhere."""
        from .coarse import key
        n = len(rows.cand)
        out = dict(valid=np.zeros(n, dtype=bool), candidate=np.full(n, -1, dtype=np.int64),
                   stop=np.full(n, 255, dtype=np.int64), card=np.full(n, NONE, dtype=np.int64),
                   halves=np.full((n, 4), np.nan, dtype=np.float32),
                   frame_cursor=np.full(n, NONE, dtype=np.int64))
        root_keys = key(rows.roots["game"], rows.roots["seat"], rows.roots["cursor"])
        order = np.argsort(root_keys, kind="stable")
        wanted = key(self.index["game"], self.index["seat"], self.index["root_cursor"])
        at = np.minimum(np.searchsorted(root_keys[order], wanted), len(order) - 1)
        known = root_keys[order][at] == wanted
        if not known.all():
            raise ValueError(f"{(~known).sum()} afterstates name no root of the rows")
        root = order[at]
        item = self.index["item"].astype(np.int64)
        if (item >= rows.roots["n"][root]).any():
            raise ValueError("An afterstate's item lies off its root's menu")
        where = rows.roots["offset"][root] + item
        if len(np.unique(where)) != len(where):
            raise ValueError("Two afterstates name one candidate")
        searched = rows.labels["searched"].astype(bool)
        covered = np.zeros(n, dtype=bool)
        covered[where] = True
        if not np.array_equal(covered, searched):
            raise ValueError("The afterstates do not cover exactly the searched candidates")
        own = rows.roots["offset"] + rows.roots["own"]
        if not (self.index["candidate"][np.isin(where, own)] == 0).all():
            raise ValueError("A root's own answer is not its search's candidate 0")
        out["valid"][where] = (self.index["flags"] & VALID).astype(bool)
        out["candidate"][where] = self.index["candidate"]
        out["stop"][where] = self.index["stop"]
        out["card"][where] = self.index["card"]
        out["halves"][where] = np.stack([self.index[f] for f in ("even_mean", "even_se", "odd_mean", "odd_se")], 1)
        out["frame_cursor"][where] = np.where(out["valid"][where], self.index["frame_cursor"], NONE)
        return out


def root_intents(dataset, rows):
    """Per root of `rows`: the spell a bare tap at the root was for (intents.bin), u32::MAX none."""
    directory = Path(dataset) / "search-advantage"
    sidecar = V.SearchAdvantage.load(directory)
    raw = (directory / "intents.bin").read_bytes()
    if hashlib.sha256(raw).hexdigest() != (sidecar.header.get("intents") or {}).get("sha256"):
        raise ValueError("intents.bin does not match its header's digest")
    intents = np.frombuffer(raw, dtype="<u4")
    if len(intents) != len(sidecar.index):
        raise ValueError("intents.bin and the index disagree")
    from .coarse import key
    keys = key(sidecar.index["game"], sidecar.index["seat"], sidecar.index["root_cursor"])
    order = np.argsort(keys, kind="stable")
    wanted = key(rows.roots["game"], rows.roots["seat"], rows.roots["cursor"])
    at = np.minimum(np.searchsorted(keys[order], wanted), len(order) - 1)
    if not (keys[order][at] == wanted).all():
        raise ValueError("A root has no row in the search-advantage index")
    return intents[order[at]].astype(np.int64)


def creativity(rows, cls, card, halves, intent):
    """(non_prior [N], confirmed [N]) per candidate. Non-prior: another coarse class than the own
    answer's, except a cast of the very spell a bare-tap own answer was for. Confirmed: on one
    half of the worlds mean - 1.64 SE > 0.03 and on the other mean > 0 (NaN fails)."""
    root_of = rows.root_of()
    own = rows.roots["offset"] + rows.roots["own"]
    own_cls = cls[own][root_of]
    merged = (own_cls == TAP) & (cls == CAST) & (card != NONE) & (card == intent[root_of])
    is_own = np.zeros(len(cls), dtype=bool)
    is_own[own] = True
    non_prior = (cls != own_cls) & ~merged & ~is_own & rows.labels["searched"].astype(bool)
    even, even_se, odd, odd_se = (halves[:, i].astype(np.float64) for i in range(4))
    with np.errstate(invalid="ignore"):
        on_even = (even - CONFIRM_Z * even_se > CONFIRM_MARGIN) & (odd > 0)
        on_odd = (odd - CONFIRM_Z * odd_se > CONFIRM_MARGIN) & (even > 0)
    confirmed = np.nan_to_num(on_even, nan=False).astype(bool) | np.nan_to_num(on_odd, nan=False).astype(bool)
    return non_prior, confirmed & ~is_own


class AfterRows:
    """The rows (arm B's, in their order) with each candidate's afterstate: `rows`, `valid` [N],
    `emb` [N, 3d] (zeros where invalid), `v`, `v0` [N], creativity flags and the sources'
    provenance."""

    def __init__(self, rows, joined, emb, v, v0, intent, meta):
        valid = joined["valid"]
        # A missing afterstate reads 0, never the capture's NaN: a NaN input poisons every
        # gradient through the head even where the gate drops its gap.
        self.v, self.v0 = (np.where(valid, x, 0.).astype(np.float32) for x in (v, v0))
        self.emb = np.where(valid[:, None], emb, 0).astype(np.float16)
        self.rows, self.meta = rows, meta
        for name in PER_CANDIDATE:
            setattr(self, name, joined[name])
        self.intent = intent
        own = rows.roots["offset"] + rows.roots["own"]
        self.own = own
        self.root_ok = self.valid[own]
        # A candidate scores only where it and its root's own answer have afterstates.
        self.scored = self.valid & self.root_ok[rows.root_of()]
        self.non_prior, self.confirmed = creativity(rows, rows.labels["cls"].astype(np.int64), self.card,
                                                    self.halves, intent)

    @classmethod
    def load(cls, rows_paths, after_paths, halves_judged=True):
        rows, parts, sources, judged_from = V.load_rows(rows_paths, halves_judged)
        if len(after_paths) != len(parts):
            raise ValueError("One --afterstates file per --rows file, in the same order")
        fields, metas = {}, []
        for path, part, rows_path in zip(after_paths, parts, rows_paths):
            data = np.load(path, allow_pickle=False)
            meta = json.loads(str(data["meta"]))
            if meta.get("contract") != ROWS_CONTRACT:
                raise ValueError(f"{path} is not {ROWS_CONTRACT}")
            if meta.get("smoke_candidates"):
                raise ValueError(f"{path} is a smoke capture")
            if meta.get("rows_sha256") != sha256(rows_path):
                raise ValueError(f"{path} was captured for other rows than {rows_path}")
            if meta.get("model_sha256") not in sources:
                raise ValueError(f"{path} comes from another net than the rows")
            check_captured(meta, path)
            if len(data["v"]) != len(part.cand):
                raise ValueError(f"{path} has another number of candidates than its rows")
            for name in (*PER_CANDIDATE, "emb", "v", "v0", "intent"):
                fields.setdefault(name, []).append(data[name])
            metas.append(dict(meta, path=str(path), sha256=sha256(path)))
        cat = {name: np.concatenate(value) for name, value in fields.items()}
        joined = {name: cat[name] for name in PER_CANDIDATE}
        valid = joined["valid"]
        if not (np.isfinite(cat["v"][valid]).all() and np.isfinite(cat["v0"][valid]).all()
                and np.isfinite(cat["emb"][valid].astype(np.float32)).all()):
            raise ValueError("A valid afterstate has a non-finite readout")
        out = cls(rows, joined, cat["emb"], cat["v"], cat["v0"],
                  cat["intent"], dict(sources=metas, judged_from=judged_from, model_sha256=next(iter(sources))))
        if int(out.valid[~rows.labels["searched"].astype(bool)].sum()):
            raise ValueError("An unsearched candidate has an afterstate")
        return out

    def weights(self):
        """(target clip(A), w, nu) per candidate: w = 1 / (SE^2 + tau^2) over the scored
        candidates other than the own answer; nu = 2 where confirmed non-prior."""
        is_own = np.zeros(len(self.v), dtype=bool)
        is_own[self.own] = True
        A, base_w, _ = V.item_labels(self.rows, TAU)
        w = np.where(self.scored & ~is_own, base_w, 0.).astype(np.float32)
        nu = np.where(self.non_prior & self.confirmed, NOVELTY, 1.).astype(np.float32)
        return np.clip(A, -CLIP, CLIP).astype(np.float32), w, nu

    def rule_weights(self):
        """The rule's weights, arm B's: w = 1 / (SE^2 + tau^2) over every searched candidate other
        than the own answer, an afterstate or not (a missing one counts at gap 0)."""
        return V.item_labels(self.rows, TAU)[1].astype(np.float64)

    def gate(self, gaps):
        """The rule's gaps: 0 for a candidate without an afterstate or whose root's own has none."""
        return np.where(self.scored, gaps, 0.)


def discovery(after, chosen):
    """Per chosen root: whether it has a confirmed non-prior candidate; the rate per 100 roots."""
    root_of = after.rows.root_of()
    flag = after.non_prior & after.confirmed
    has = np.bincount(root_of, weights=flag, minlength=len(after.rows.state)) > 0
    roots = np.flatnonzero(chosen)
    return has, (float(has[roots].mean() * 100) if roots.size else None)


def reproduction(after, gaps, margin, chosen):
    """Of the chosen roots with a discovery: the share where the head's play (its best non-own
    candidate where the gap exceeds the margin) is a confirmed non-prior candidate, and the
    summed A of those plays over the summed A of each root's best confirmed non-prior candidate."""
    rows = after.rows
    has, rate = discovery(after, chosen)
    flag = after.non_prior & after.confirmed
    A = rows.labels["A"].astype(np.float64)
    searched = rows.labels["searched"].astype(bool)
    hits, captured, available, roots = 0, 0., 0., 0
    for r in np.flatnonzero(chosen & has):
        lo, n, own = int(rows.roots["offset"][r]), int(rows.roots["n"][r]), int(rows.roots["own"][r])
        items = np.flatnonzero(searched[lo:lo + n])
        others = items[items != own]
        roots += 1
        good = others[flag[lo + others]]
        available += float(A[lo + good].max())
        if not len(others):
            continue
        best = int(others[np.argmax(gaps[lo + others])])
        if gaps[lo + best] > margin and flag[lo + best]:
            hits += 1
            captured += float(A[lo + best])
    return dict(discovery_roots=roots, discovery_rate_per_100_roots=rate,
                reproduced=hits, reproduction_share=hits / roots if roots else None,
                captured_A_share=captured / available if available > 0 else None)


def pairwise(after, gaps, chosen, weight=None, target=None):
    """Arm B's pairwise loss of full-size gated `gaps` over the `chosen` roots (numpy): every
    searched candidate other than the own answer, a missing afterstate at gap 0."""
    w = after.rule_weights() if weight is None else weight
    target = after.rows.labels["A"].astype(np.float64) if target is None else target
    in_root = chosen[after.rows.root_of()]
    w = np.where(in_root, w, 0.)
    if not w.sum() > 0:
        raise ValueError("No searched candidate besides the own answers in these roots: there is no loss to select on")
    return float((w * (gaps - target) ** 2).sum() / w.sum())


def selection_half(after):
    """The selection half of the held-out roots; a fit or a decision without one is refused."""
    select, _ = V.held_out_halves(after.rows)
    if not select.any():
        raise ValueError("The rows hold out no game: the selection half is empty")
    return select


def selection_report(after, gaps, select):
    """The selection half's report and margin, with the early falsifier's Pearson."""
    chooser = V.report(after.rows, gaps, after.rule_weights(), select)
    margin, gain = V.best_margin(chooser)
    return chooser, margin, gain


def falsified(pearson):
    """The early falsifier: no correlation, one that is no number, or one at or below 0.25."""
    return pearson is None or not pearson > 0.25


def sources_of(meta):
    """The digests of the afterstate files a fit or a decision read, in order."""
    return [source["sha256"] for source in (meta or {}).get("sources", ())]


def read_preregistration(path):
    data = json.loads(Path(path).read_text())
    if data.get("contract") != PREREGISTRATION_CONTRACT:
        raise ValueError(f"{path} is not a {PREREGISTRATION_CONTRACT}")
    return data, sha256(path)


def strict_json(value):
    """JSON that strict readers (train_ops) accept: a non-finite float is written as null."""
    def clean(x):
        if isinstance(x, dict):
            return {k: clean(v) for k, v in x.items()}
        if isinstance(x, (list, tuple)):
            return [clean(v) for v in x]
        if isinstance(x, (float, np.floating)):
            return float(x) if np.isfinite(x) else None
        if isinstance(x, np.integer):
            return int(x)
        if isinstance(x, np.bool_):
            return bool(x)
        return x
    return json.dumps(clean(value), indent=1, allow_nan=False)


def seal(out, gaps, result):
    """Writes a fit's selection-half result (`fit.json`) and its sealed gaps, which only
    `decide` reads."""
    out = Path(out)
    sealed = out / "sealed"
    sealed.mkdir(parents=True)
    np.save(sealed / "gaps.npy", np.asarray(gaps, dtype=np.float64))
    (sealed / "README").write_text("Gaps over every candidate, held-out included. Read only by "
                                   "`training4.afterstate decide`; nothing else reads the report half.\n")
    result = dict(result, gaps_sha256=sha256(sealed / "gaps.npy"))
    (out / "fit.json").write_text(strict_json(result))
    return result


# --- V0: frozen embeddings and a siamese MLP ---------------------------------------------------

def v0_inputs(after):
    """[N, 3d + 2 + |CLASSES| + 1]: e(s'), V, V_K0, the coarse class one-hot, payable-added."""
    cls = after.rows.labels["cls"].astype(np.int64)
    onehot = np.zeros((len(cls), len(CLASSES)), dtype=np.float32)
    known = (cls >= 0) & (cls < len(CLASSES))
    onehot[np.flatnonzero(known), cls[known]] = 1.
    return np.concatenate((after.emb.astype(np.float32), after.v[:, None], after.v0[:, None], onehot,
                           after.rows.labels["payable"].astype(np.float32)[:, None]), 1)


def fit_v0(after, lr=1e-3, weight_decay=1e-2, epochs=40, batch=256, patience=6, seed=1, hidden=512, log=print):
    torch.manual_seed(seed)
    rng = np.random.default_rng(seed)
    rows = after.rows
    x = torch.from_numpy(v0_inputs(after))
    head = V.SearchHead(x.shape[1], "mlp", hidden)
    optimizer = torch.optim.AdamW(head.parameters(), lr=lr, weight_decay=weight_decay)
    target, w, nu = (torch.from_numpy(a) for a in after.weights())
    root_of = torch.from_numpy(rows.root_of())
    own = torch.from_numpy(after.own.astype(np.int64))
    scored = torch.from_numpy(after.scored)
    select = selection_half(after)
    zero_select = pairwise(after, np.zeros(len(rows.cand)), select)   # what a head of no gaps scores there
    train = V.training_roots(rows, rng)

    def gaps_all():
        head.eval()
        with torch.no_grad():
            q = head(x)
            return torch.where(scored, q - q[own][root_of], 0.).double().numpy()

    best, best_state, history, waited = None, None, [], 0
    began = time.perf_counter()
    for epoch in range(1, epochs + 1):
        head.train()
        order = rng.permutation(train)
        total, steps = 0., 0
        for start in range(0, len(order), batch):
            chosen = torch.zeros(len(rows.state), dtype=torch.bool)
            chosen[torch.from_numpy(order[start:start + batch])] = True
            members = chosen[root_of]
            q = torch.zeros(len(x))
            q[members] = head(x[members])
            gaps = torch.where(scored, q - q[own][root_of], 0.)
            weight = torch.where(chosen[root_of], w * nu, 0.)
            loss = (weight * (gaps - target).square()).sum() / weight.sum().clamp(min=1e-12)
            if not torch.isfinite(loss):
                raise FloatingPointError(f"V0's loss is not finite at epoch {epoch}")
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            optimizer.step()
            total, steps = total + float(loss.detach()), steps + 1
        select_loss = pairwise(after, gaps_all(), select)
        history.append(dict(epoch=epoch, train=total / max(steps, 1), select=select_loss,
                            minutes=(time.perf_counter() - began) / 60))
        log(f"[afterstate] V0 epoch={epoch} train={total / max(steps, 1):.6f} select={select_loss:.6f}")
        if best is None or select_loss < best:
            best, waited = select_loss, 0
            best_state = {k: v.clone() for k, v in head.state_dict().items()}
        else:
            waited += 1
            if waited >= patience:
                break
    head.load_state_dict(best_state)
    gaps = gaps_all()
    return head, gaps, dict(history=history, best_select_loss=best, zero_select_loss=zero_select,
                            train_roots=int(len(train)),
                            epochs_run=len(history))


# --- the afterstate readout, shared by the capture and the tower --------------------------------

def collate(frames):
    import dataset4 as D4
    items = [{**{name: value[None] for name, value in frame.items()}, "step_valid": np.array([True]),
              "is_new_traj": np.array(True), "segment_index": np.array(-1, dtype=np.int64)} for frame in frames]
    return D4.collate_trajectories(items)


BOARD_INPUTS = ("entity_feat", "entity_dsl", "entity_mask", "seat_feat", "global_feat", "edge_index",
                "edge_type", "edge_mask", "entity_rules")


def board_inputs(host, device):
    return [host[name][:, 0].to(device) for name in BOARD_INPUTS]


def readout(net, board, pad, hidden, seat_feat, thinking_steps=THINKING_STEPS):
    """From the root's memory `hidden` [B, 9, d]: observe the afterstate's board (K = 0), then
    think `thinking_steps` scratchpad steps on it; (V [B], V_K0 [B], e [B, 3d]) with
    e = [g_ref, s_ref self, s_ref opponent] of the thought state, as the model's own forward
    computes a decision frame's readout after its observation."""
    from model4.config import RECURRENT_SLOTS
    b = board.shape[0]
    observed = net.observe_memory(board, hidden, torch.zeros(b, dtype=torch.long, device=board.device))
    z = observed + board[:, :RECURRENT_SLOTS]
    z_obs = net.h_norm(z)
    v0, _ = net.seat_values(z_obs[:, 1:RECURRENT_SLOTS], z_obs[:, 0], seat_feat)
    steps = torch.full((b,), thinking_steps, dtype=torch.long, device=board.device)
    depth = net.scratchpad_depth(steps)
    for k in range(net.scratchpad_limit(thinking_steps)):
        step_t = torch.full((b,), k, dtype=torch.long, device=board.device)
        z_next = net.thinking_block(z, board, pad, step_t, None)
        active = (depth > k).view(b, 1, 1).to(z.dtype)
        z = z + active * (z_next - z)
    z_out = net.h_norm(z)
    v, _ = net.seat_values(z_out[:, 1:RECURRENT_SLOTS], z_out[:, 0], seat_feat)
    emb = torch.cat((z_out[:, 0], z_out[:, 1], z_out[:, 2]), 1)
    return v, v0, emb


class FrameStore:
    """An afterstate dataset's frames, found by (game, seat, frame cursor). A frame's rule
    references are node ids of its dataset's catalog, so the dataset's must be the net's."""

    def __init__(self, dataset, net, encoder_version=10):
        import dataset4 as D4
        from trajectory_store4 import TrajectoryStore4
        from .coarse import key
        self.dataset = Path(dataset)
        meta = json.loads((self.dataset / "meta.json").read_text())
        V.check_catalog(meta["structure_sha256"], net, f"The afterstate dataset {self.dataset}")
        self.store = TrajectoryStore4(self.dataset, FRAMES_SPLIT, meta[SHARDS_KEY])
        # The train split's reader decodes any BST4 frame of the dataset (its sizes and vocab).
        self.decoder = D4.TrajectoryDataset4(self.dataset, split="train", unroll_len=32,
                                             encoder_version=encoder_version)
        index = self.store.index
        self.keys = key(index["game"], index["seat"], index["cursor"])
        self.order = np.argsort(self.keys, kind="stable")

    def rows_of(self, game, seat, cursor):
        from .coarse import key
        wanted = key(game, seat, cursor)
        at = np.minimum(np.searchsorted(self.keys[self.order], wanted), len(self.order) - 1)
        if not (self.keys[self.order][at] == wanted).all():
            raise ValueError("An afterstate frame is missing from its split")
        return self.order[at]

    def frame(self, row):
        frame = self.decoder._encode_bst4(self.store.read_raw(int(row)))
        if bool(frame["is_decision"]):
            raise ValueError("An afterstate frame is not an observation frame")
        return frame


def new_npz(out):
    """The file `np.savez` writes for `--out`, which must not exist: it appends .npz to a name
    without it, so that is the name to look for."""
    out = Path(out)
    out = out if out.suffix == ".npz" else out.with_name(out.name + ".npz")
    if out.exists():
        raise FileExistsError(f"{out} exists")
    return out


def check_structures(data, after_dir, net):
    """The card structure digest the rows' dataset, the afterstate dataset and the net share."""
    digests = [json.loads((Path(d) / "meta.json").read_text())["structure_sha256"] for d in (data, after_dir)]
    if digests[0] != digests[1]:
        raise ValueError(f"The rows' dataset {data} and the afterstate dataset {after_dir} use different public "
                         "rule catalogs")
    V.check_catalog(digests[0], net, f"The rows' dataset {data}")
    return digests[0]


def check_captured(meta, path):
    """A capture names the catalog its two datasets and its net share. One from before captures
    compared them names none: its datasets are compared now, where they are still there."""
    if meta.get("structure_sha256"):
        return
    files = [Path(meta[name]) / "meta.json" for name in ("data", "afterstates") if meta.get(name)]
    if len({json.loads(file.read_text()).get("structure_sha256") for file in files if file.is_file()}) > 1:
        raise ValueError(f"{path} was captured from rows and afterstates of different public rule catalogs")


# --- capture: r1's afterstate readouts ----------------------------------------------------------

def capture(args):
    from torch_io4 import file_digest
    from value_calibration4.checkpoint import load_source
    from .advantage_tower import Roots
    out = new_npz(args.out)
    torch.set_num_threads(args.threads)
    device = torch.device(args.device)
    path, _, net = load_source(args.model)
    source = file_digest(path)
    net.decision_chunk, net.board_token_budget = 4, 32768
    net.prune_unused_policy_heads, net.gpu_prefetch = False, 0
    part = V.AdvantageRows.load(args.rows)
    if part.meta.get("model_sha256") != source:
        raise ValueError("The rows were captured from another net than --model")
    data = Path(part.meta["data"])
    after_dir = Path(args.afterstates)
    structure = check_structures(data, after_dir, net)
    original = json.loads((data / "search-advantage" / "header.json").read_text())
    again = json.loads((after_dir / "search-advantage" / "header.json").read_text())
    for name in ("index", "cands"):
        if original[name]["sha256"] != again[name]["sha256"]:
            raise ValueError(f"The afterstate dataset's search-advantage {name} is not the rows' dataset's")
    roots = Roots.load([part], [args.state_dir], net, source)
    sidecar = Sidecar.load(after_dir)
    joined = sidecar.join(part)
    intent = root_intents(data, part)
    frames = FrameStore(after_dir, net)
    net = net.to(device).eval()
    n, d = len(part.cand), net.cfg.d_model
    emb = np.zeros((n, 3 * d), dtype=np.float16)
    v, v0 = np.full(n, np.nan, dtype=np.float32), np.full(n, np.nan, dtype=np.float32)
    todo = np.flatnonzero(joined["valid"])
    if args.smoke_candidates:
        todo = todo[:args.smoke_candidates]   # a smoke file: never fit on (AfterRows.load refuses it)
    root_of = part.root_of()
    store_rows = frames.rows_of(part.roots["game"][root_of[todo]], part.roots["seat"][root_of[todo]],
                                joined["frame_cursor"][todo])
    began = time.perf_counter()
    with torch.no_grad():
        for start in range(0, len(todo), args.batch):
            chunk = todo[start:start + args.batch]
            batch = [frames.frame(r) for r in store_rows[start:start + args.batch]]
            host = collate(batch)
            net.clear_structure()
            net.prepare_structure((host["entity_rules"], host["cand_rules"]))
            board, pad = net.encode_board(*board_inputs(host, device))
            hidden = roots.hidden[torch.from_numpy(root_of[chunk])].to(device)
            out = readout(net, board, pad, hidden, host["seat_feat"][:, 0].to(device))
            net.clear_structure()
            v[chunk], v0[chunk] = out[0].float().cpu().numpy(), out[1].float().cpu().numpy()
            emb[chunk] = out[2].float().cpu().numpy().astype(np.float16)
            if (start // args.batch) % 50 == 0:
                print(f"[afterstate] capture {start + len(chunk)}/{len(todo)} "
                      f"{time.perf_counter() - began:.0f} s", flush=True)
    meta = dict(contract=ROWS_CONTRACT, rows=str(args.rows), rows_sha256=sha256(args.rows), data=str(data),
                afterstates=str(after_dir), structure_sha256=structure,
                sidecar_sha256=sidecar.header["index"]["sha256"],
                sidecar_counts=sidecar.header.get("counts"), model=str(path), model_sha256=source,
                device=str(device), thinking_steps=THINKING_STEPS, valid=int(joined["valid"].sum()),
                searched=int(part.labels["searched"].sum()), seconds=time.perf_counter() - began,
                smoke_candidates=args.smoke_candidates, captured=int(len(todo)),
                trainer_source=str(Path(__file__).resolve().parents[3]))
    np.savez(new_npz(out), meta=json.dumps(meta), emb=emb, v=v, v0=v0, intent=intent,
             **{name: joined[name] for name in PER_CANDIDATE})
    print("[afterstate] capture " + json.dumps({k: meta[k] for k in ("valid", "searched", "seconds")}), flush=True)
    return meta


# --- V1: the tower ------------------------------------------------------------------------------

def make_tower(net):
    """(lower, tower): `lower` runs r1's board up to trunk layer 9 without gradients (a view of
    r1's own modules, r1 unchanged); `tower` is a deep copy of r1 in which only V1_MODULES train."""
    for parameter in net.parameters():
        parameter.requires_grad_(False)
    tower = copy.deepcopy(net)
    tower.cfg.activation_checkpointing = False
    for name in V1_MODULES:
        for parameter in tower.get_submodule(name).parameters():
            parameter.requires_grad_(True)
    return tower


def trainable_state(tower):
    return {f"{name}.{key}": value.detach().cpu().clone()
            for name in V1_MODULES for key, value in tower.get_submodule(name).state_dict().items()}


class TowerBoard:
    """r1's trunk below layer 9 (frozen, no gradient), then the tower's layers 9-12 and
    trunk_norm (with gradient)."""

    def __init__(self, frozen, tower):
        self.frozen, self.tower = frozen, tower
        self.layers = len(frozen.trunk_layers)

    def __call__(self, inputs):
        seen = []
        frozen_layers = self.frozen.trunk_layers
        handle = self.frozen.trunk_norm.register_forward_pre_hook(lambda _m, args: seen.append(args[0]))
        try:
            # r1's own encoder, stopped after layer 8: the upper layers are the tower's.
            self.frozen.trunk_layers = torch.nn.ModuleList(list(frozen_layers)[:UPPER_FROM])
            with torch.no_grad():
                _, pad = self.frozen.encode_board(*inputs)
        finally:
            self.frozen.trunk_layers = frozen_layers
            handle.remove()
        x = seen[0].detach()
        for layer in self.tower.trunk_layers[UPPER_FROM:]:
            x = layer(x, src_key_padding_mask=pad)
        return self.tower.trunk_norm(x), pad


class TowerData:
    """Every afterstate frame of the scored candidates, decoded once, with its root's memory."""

    def __init__(self, after, rows_paths, state_dirs, after_dirs, net, source, log=print, roots_needed=None):
        """`roots_needed` ([R] bool, smoke tests only): decode only those roots' afterstates."""
        from .advantage_tower import Roots
        parts = [V.AdvantageRows.load(p) for p in rows_paths]
        if not len(parts) == len(state_dirs) == len(after_dirs):
            raise ValueError("One --state-dir and one --after-data per --rows file, in the same order")
        for part, after_dir in zip(parts, after_dirs):
            check_structures(part.meta["data"], after_dir, net)
        hidden = []
        for part, state_dir in zip(parts, state_dirs):
            hidden.append(Roots.load([part], [state_dir], net, source, log=log).hidden)
        self.hidden = torch.cat(hidden)
        if len(self.hidden) != len(after.rows.state):
            raise ValueError("The roots' memories and the rows disagree")
        self.frames = {}
        root_of = after.rows.root_of()
        base = 0
        began = time.perf_counter()
        for part, after_dir in zip(parts, after_dirs):
            store = FrameStore(after_dir, net)
            span = np.arange(base, base + len(part.cand))
            todo = span[after.scored[span]]
            if roots_needed is not None:
                todo = todo[roots_needed[root_of[todo]]]
            if not np.array_equal(Sidecar.load(after_dir).join(part)["valid"], after.valid[span]):
                raise ValueError(f"{after_dir}: the afterstates are not the captured ones")
            store_rows = store.rows_of(after.rows.roots["game"][root_of[todo]], after.rows.roots["seat"][root_of[todo]],
                                       after.frame_cursor[todo])
            for i, r in zip(todo, store_rows):
                self.frames[int(i)] = store.frame(r)
            base += len(part.cand)
        log(f"[afterstate] V1 decoded {len(self.frames)} afterstates in {time.perf_counter() - began:.0f} s")


def smoke_roots_of(after, count, seed=1):
    """Smoke tests only: the first `count` training and selection roots with afterstates."""
    select, _ = V.held_out_halves(after.rows)
    train = V.training_roots(after.rows, np.random.default_rng(seed))
    keep = np.zeros(len(after.rows.state), dtype=bool)
    keep[train[after.root_ok[train]][:count]] = True
    keep[np.flatnonzero(select & after.root_ok)[:count]] = True
    return keep


def fit_v1(after, data, frozen, device, tower_lr=3e-5, alpha_lr=1e-3, epochs=25, batch=16, patience=4,
           minutes=95., seed=1, log=print, smoke_roots=0):
    """`smoke_roots` > 0 (smoke tests only): one epoch on that many training roots, the selection
    loss on that many selection roots, no held-out gaps."""
    torch.manual_seed(seed)
    rng = np.random.default_rng(seed)
    rows = after.rows
    select = selection_half(after)
    zero_select = pairwise(after, np.zeros(len(rows.cand)), select)
    began = time.perf_counter()
    root_of_all = rows.root_of()
    tower = make_tower(frozen).to(device)
    frozen.eval()
    board_of = TowerBoard(frozen, tower)
    alpha = torch.nn.Parameter(torch.ones((), device=device))
    params = [p for p in tower.parameters() if p.requires_grad]
    optimizer = torch.optim.AdamW([dict(params=params, lr=tower_lr, weight_decay=0.),
                                   dict(params=[alpha], lr=alpha_lr, weight_decay=0.)])
    target, w, nu = after.weights()
    offset, n_of, own_of = (rows.roots[k].astype(np.int64) for k in ("offset", "n", "own"))
    train = V.training_roots(rows, rng)
    train = train[after.root_ok[train]]
    if smoke_roots:
        train, epochs = train[:smoke_roots], 1

    def values(items):
        frames = [data.frames[int(i)] for i in items]
        host = collate(frames)
        for net in (frozen, tower):
            net.clear_structure()
            net.prepare_structure((host["entity_rules"], host["cand_rules"]))
        try:
            board, pad = board_of(board_inputs(host, device))
            hidden = data.hidden[torch.from_numpy(root_of_all[items])].to(device)
            v, _, _ = readout(tower, board, pad, hidden, host["seat_feat"][:, 0].to(device))
        finally:
            frozen.clear_structure()
            tower.clear_structure()
        return v

    def root_items(chosen):
        return [np.arange(offset[r], offset[r] + n_of[r])[after.scored[offset[r]:offset[r] + n_of[r]]]
                for r in chosen]

    def gaps_of(chosen, grad=False):
        """[items], gaps of the chosen roots' scored candidates (own included at 0)."""
        per_root = root_items(chosen)
        items = np.concatenate(per_root) if per_root else np.zeros(0, dtype=np.int64)
        if not len(items):
            return items, torch.zeros(0, device=device)
        q = alpha * values(items)
        local = torch.from_numpy(np.repeat(np.arange(len(per_root)), [len(p) for p in per_root])).to(device)
        own_pos = torch.from_numpy(np.array([int(np.flatnonzero(p == offset[r] + own_of[r])[0]) + s
                                             for p, r, s in zip(per_root, chosen,
                                                                np.cumsum([0] + [len(p) for p in per_root[:-1]]))]))
        return items, q - q[own_pos.to(device)][local]

    def full_gaps(chosen):
        out = np.zeros(len(rows.cand), dtype=np.float64)
        tower.eval()
        with torch.no_grad():
            for start in range(0, len(chosen), batch * 2):
                items, gaps = gaps_of(chosen[start:start + batch * 2])
                out[items] = gaps.double().cpu().numpy()
        return out

    check = reproduce(after, data, frozen, tower, board_of, values, train[:batch], device)
    log(f"[afterstate] V1 trainable={sum(p.numel() for p in params)} reproduce={json.dumps(check)}")
    out_of_time = lambda: minutes > 0 and time.perf_counter() - began > minutes * 60
    selecting = np.flatnonzero(select & after.root_ok)
    if smoke_roots:
        selecting = selecting[:smoke_roots]
    best, best_state, history, waited, stopped = None, None, [], 0, "epochs"
    tw, tt, tn = (torch.from_numpy(a).to(device) for a in (w, target, nu))
    for epoch in range(1, epochs + 1):
        tower.train()
        order = rng.permutation(train)
        total, steps = 0., 0
        for start in range(0, len(order), batch):
            items, gaps = gaps_of(order[start:start + batch], grad=True)
            ti = torch.from_numpy(items).to(device)
            weight = tw[ti] * tn[ti]
            loss = (weight * (gaps - tt[ti]).square()).sum() / weight.sum().clamp(min=1e-12)
            if not torch.isfinite(loss):
                raise FloatingPointError(f"V1's loss is not finite at epoch {epoch}")
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(params + [alpha], 1.0)
            optimizer.step()
            total, steps = total + float(loss.detach()), steps + 1
            if out_of_time():
                break
        select_loss = pairwise(after, full_gaps(selecting), select)
        if not np.isfinite(select_loss):
            raise FloatingPointError(f"V1's selection loss is not finite at epoch {epoch}")
        history.append(dict(epoch=epoch, train=total / max(steps, 1), select=select_loss, steps=steps,
                            alpha=float(alpha.detach()), minutes=(time.perf_counter() - began) / 60))
        log(f"[afterstate] V1 epoch={epoch} train={total / max(steps, 1):.6f} select={select_loss:.6f} "
            f"alpha={float(alpha.detach()):.4f} minutes={(time.perf_counter() - began) / 60:.1f}")
        if best is None or select_loss < best:
            best, waited = select_loss, 0
            best_state = dict(tower=trainable_state(tower), alpha=float(alpha.detach()))
        else:
            waited += 1
        if waited >= patience:
            stopped = "patience"
            break
        if out_of_time():
            stopped = "minutes"
            break
    missing, unexpected = tower.load_state_dict(best_state["tower"], strict=False)
    if unexpected:
        raise ValueError("Tower weights do not load into the tower")
    with torch.no_grad():
        alpha.fill_(best_state["alpha"])
    held = np.flatnonzero(rows.held_out & after.root_ok)
    gaps = None if smoke_roots else full_gaps(held)
    return best_state, gaps, dict(history=history, best_select_loss=best, zero_select_loss=zero_select,
                                  stopped=stopped, reproduce=check,
                                  train_roots=int(len(train)), minutes=(time.perf_counter() - began) / 60,
                                  trainable_parameters=sum(p.numel() for p in params))


@torch.no_grad()
def reproduce(after, data, frozen, tower, board_of, values, chosen, device):
    """The untrained tower (lower r1 + its own upper layers) gives the capture's V back."""
    tower.eval()
    items = np.concatenate([np.arange(after.rows.roots["offset"][r], after.rows.roots["offset"][r]
                                      + after.rows.roots["n"][r]) for r in chosen])
    items = items[after.scored[items]]
    got = values(items).float().cpu().numpy()
    want = after.v[items]
    out = dict(items=int(len(items)), max_abs=float(np.abs(got - want).max()))
    if out["max_abs"] > 2e-3:
        raise ValueError(f"The untrained tower does not reproduce the captured values: {out}")
    return out


# --- decide -------------------------------------------------------------------------------------

def decide(prereg_path, stage_dirs, after, out_path):
    """The pre-registered decision over the fits of `stage_dirs`, in tie-break order: the stage by
    the selection half, its verdict by the report half, which is read once (`out_path` must not
    exist). A stage without its outputs is no candidate and is named in `left_out`."""
    from .advantage_tower import verdict, margin_key
    if Path(out_path).exists():
        raise FileExistsError(f"{out_path} exists: the report half is read once")
    prereg, digest = read_preregistration(prereg_path)
    names = [name for name, _ in stage_dirs]
    if len(set(names)) != len(names):
        raise ValueError(f"A stage is named twice: {names}")
    fits, left_out = {}, []
    for name, directory in stage_dirs:
        fit_path = Path(directory) / "fit.json"
        if not fit_path.is_file():
            left_out.append(dict(stage=name, directory=str(directory)))
            print(f"[afterstate] decide leaves out stage {name}: {directory} holds no fit.json", flush=True)
            continue
        fit = json.loads(fit_path.read_text())
        if fit.get("contract") != FIT_CONTRACT:
            raise ValueError(f"{directory} holds a fit of {fit.get('contract')!r}, not {FIT_CONTRACT}")
        if fit.get("preregistration_sha256") != digest:
            raise ValueError(f"{directory} was not fit under this pre-registration")
        gaps = np.load(Path(directory) / "sealed" / "gaps.npy")
        if sha256(Path(directory) / "sealed" / "gaps.npy") != fit["gaps_sha256"]:
            raise ValueError(f"{directory}: the sealed gaps are not the fit's")
        if len(gaps) != len(after.rows.cand):
            raise ValueError(f"{directory}: the gaps do not cover the rows")
        sealed, given = (sources_of(meta) for meta in (fit.get("afterstates"), after.meta))
        if sealed != given:
            raise ValueError(f"{directory} was fit on other afterstate files than these, or in another order")
        fits[name] = (fit, after.gate(gaps))
    if not fits:
        raise ValueError("No stage produced its outputs")
    # The selection half again, with the rule's weights (the gain and the margin do not read
    # weights; the falsifier's Pearson does): the stage and its margin come from here.
    select, (_, final) = selection_half(after), V.held_out_halves(after.rows)
    if not final.any():
        raise ValueError("The rows hold out a single game per source: the report half is empty")
    selection = {}
    for name, (fit, gaps) in fits.items():
        chooser, margin, gain = selection_report(after, gaps, select)
        if margin != fit["margin"]:
            raise ValueError(f"{name}: the selection half's margin is not the fit's")
        selection[name] = dict(gain_per_100_roots=gain, margin=margin, pearson_pooled=chooser["pearson_pooled"],
                               falsifier_triggered=falsified(chooser["pearson_pooled"]))
    gains = {name: cell["gain_per_100_roots"] for name, cell in selection.items()}
    chosen = max(fits, key=lambda name: (gains[name], -list(fits).index(name)))
    w = after.rule_weights()
    reports, verdicts, creative, post_hoc = {}, {}, {}, {}
    first4 = after.candidate < 4
    for name, (fit, gaps) in fits.items():
        report = V.report(after.rows, gaps, w, final, fit["margin"])
        reports[name] = report
        verdicts[name] = verdict(dict(held_out=report, margin=fit["margin"]), prereg["rule"])
        creative[name] = reproduction(after, gaps, fit["margin"], final)
        # Post hoc, outside the pre-registration (coordinator, 19 Nov): the play-time budget
        # searches K <= 4 macro candidates; the choice restricted to the event's first four.
        restricted = np.where(first4, gaps, -np.inf)
        cut = V.report(after.rows, np.where(np.isfinite(restricted), restricted, -1e9), w, final, fit["margin"])
        post_hoc[name] = {m: cut["margins"][margin_key(m)] for m in (fit["margin"],)}
    by_source = {name: {str(int(s)): (lambda c: dict(pearson_pooled=c["pearson_pooled"],
                                                     gain_per_100_roots=c["margins"][margin_key(fit["margin"])]["gain_per_100_roots"],
                                                     se=c["margins"][margin_key(fit["margin"])]["se"],
                                                     search_halves=c["search_halves"]))(
        V.report(after.rows, gaps, w, final & (after.rows.source == s), fit["margin"]))
        for s in np.unique(after.rows.source)} for name, (fit, gaps) in fits.items()}
    raw = after.gate(np.where(after.scored, after.v - after.v[after.own][after.rows.root_of()], 0.))
    out = dict(contract=DECISION_CONTRACT, preregistration=str(prereg_path), preregistration_sha256=digest,
               rule=prereg["rule"],
               stages={name: dict(stage=fit.get("stage"), seed=fit.get("seed"), directory=str(dict(stage_dirs)[name]))
                       for name, (fit, _) in fits.items()}, left_out=left_out,
               selection_gain_per_100_roots=gains, selection=selection, chosen_stage=chosen,
               verdict=verdicts[chosen],
               creativity=dict(chosen=creative[chosen], by_stage=creative,
                               discovery_rate_all_roots_per_100=discovery(after, np.ones(len(after.rows.state), bool))[1]),
               informational=dict(verdicts=verdicts, by_source=by_source,
                                  raw_r1_gap=dict(pearson_pooled=V.report(after.rows, raw, w, final)["pearson_pooled"]),
                                  valid_share=float(after.scored[after.rows.labels["searched"].astype(bool)].mean()),
                                  roots_own_valid=float(after.root_ok.mean()),
                                  search_halves=reports[chosen]["search_halves"]),
               post_hoc_not_preregistered=dict(
                   note="the stages' play restricted to the event's first four candidates (the search's macro "
                        "order, own first), at each stage's own margin; requested by the coordinator after the seal",
                   first_four=post_hoc),
               reports=reports)
    with open(out_path, "x") as stream:
        stream.write(strict_json(out))
    print("[afterstate] decision " + json.dumps(dict(chosen_stage=chosen, left_out=[s["stage"] for s in left_out],
                                                     verdict=out["verdict"],
                                                     creativity=out["creativity"]["chosen"])), flush=True)
    return out


# --- command line -------------------------------------------------------------------------------

def stage_result(stage, after, gaps, extra, args, digest):
    select = selection_half(after)
    gaps = after.gate(gaps)
    chooser, margin, gain = selection_report(after, gaps, select)
    raw = after.gate(np.where(after.scored, after.v - after.v[after.own][after.rows.root_of()], 0.))
    raw_chooser = selection_report(after, raw, select)[0]
    return dict(contract=FIT_CONTRACT, stage=stage, preregistration=args.preregistration,
                preregistration_sha256=digest, seed=args.seed, margin=margin,
                selection_gain_per_100_roots=gain, margin_selection=chooser,
                falsifier=dict(selection_pearson_pooled=chooser["pearson_pooled"],
                               triggered=falsified(chooser["pearson_pooled"])),
                raw_r1_gap_selection=dict(pearson_pooled=raw_chooser["pearson_pooled"],
                                          margins=raw_chooser["margins"]),
                afterstates=after.meta, scored_share=float(after.scored[after.rows.labels["searched"].astype(bool)].mean()),
                confirmed_non_prior=int((after.non_prior & after.confirmed).sum()),
                trainer_source=str(Path(__file__).resolve().parents[3]), **extra)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    grab = commands.add_parser("capture")
    grab.add_argument("--rows", required=True)
    grab.add_argument("--state-dir", required=True)
    grab.add_argument("--afterstates", required=True, help="The afterstate dataset (convert4 --search-afterstates)")
    grab.add_argument("--model", required=True)
    grab.add_argument("--out", required=True)
    grab.add_argument("--device", default="cpu")
    grab.add_argument("--batch", type=int, default=64)
    grab.add_argument("--threads", type=int, default=8)
    grab.add_argument("--runtime", default="")
    grab.add_argument("--smoke-candidates", type=int, default=0,
                      help="Smoke tests only: capture the first N valid afterstates (the file is never fit on)")
    for name in ("fit-v0", "fit-v1", "decide"):
        sub = commands.add_parser(name)
        sub.add_argument("--rows", action="append", required=True)
        sub.add_argument("--afterstates", action="append", required=True, help=f"{ROWS_CONTRACT} files, as --rows")
        sub.add_argument("--preregistration", required=True)
        if name != "decide":
            sub.add_argument("--out", required=True)
            sub.add_argument("--seed", type=int, required=True)
            sub.add_argument("--threads", type=int, default=8)
    v1 = commands.choices["fit-v1"]
    v1.add_argument("--state-dir", action="append", required=True)
    v1.add_argument("--after-data", action="append", required=True)
    v1.add_argument("--model", required=True)
    v1.add_argument("--device", default="cuda")
    v1.add_argument("--minutes", type=float, default=95.)
    v1.add_argument("--epochs", type=int, default=25)
    v1.add_argument("--batch", type=int, default=16)
    v1.add_argument("--runtime", default="")
    v1.add_argument("--smoke-roots", type=int, default=0,
                    help="Smoke tests only: one epoch on N training roots; writes smoke.json, seals nothing")
    dec = commands.choices["decide"]
    dec.add_argument("--stage", action="append", required=True, help="NAME=DIR, in tie-break order")
    dec.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    if getattr(args, "runtime", ""):
        import os
        runtime = Path(args.runtime).expanduser()
        if not (runtime / "trajectory_cache4").is_file():
            raise FileNotFoundError(f"{runtime} lacks trajectory_cache4")
        os.environ["PATH"] = f"{runtime}{os.pathsep}{os.environ.get('PATH', '')}"
    if args.command == "capture":
        return capture(args)
    after = AfterRows.load(args.rows, args.afterstates)
    if args.command == "decide":
        return decide(args.preregistration, [tuple(a.split("=", 1)) for a in args.stage], after, args.out)
    torch.set_num_threads(args.threads)
    digest = read_preregistration(args.preregistration)[1]
    out = Path(args.out)
    if out.exists():
        raise FileExistsError(f"{out} exists; a stage gets its own directory")
    if args.command == "fit-v0":
        head, gaps, extra = fit_v0(after, seed=args.seed)
        result = stage_result("V0", after, gaps, extra, args, digest)
        result = seal(out, gaps, result)
        torch.save(dict(state_dict=head.state_dict(), width=v0_inputs(after).shape[1], hidden=512), out / "head.pt")
    else:
        from torch_io4 import file_digest
        from value_calibration4.checkpoint import load_source
        path, _, net = load_source(args.model)
        source = file_digest(path)
        if source != after.meta["model_sha256"]:
            raise ValueError("The afterstates were captured from another net than --model")
        net.decision_chunk, net.board_token_budget = 4, 32768
        net.prune_unused_policy_heads, net.gpu_prefetch = False, 0
        keep = smoke_roots_of(after, args.smoke_roots, args.seed) if args.smoke_roots else None
        data = TowerData(after, args.rows, args.state_dir, args.after_data, net, source, roots_needed=keep)
        device = torch.device(args.device)
        net = net.to(device)
        state, gaps, extra = fit_v1(after, data, net, device, epochs=args.epochs, batch=args.batch,
                                    minutes=args.minutes, seed=args.seed, smoke_roots=args.smoke_roots)
        if args.smoke_roots:
            # A smoke run: timing and the reproduction check only; no gaps, nothing sealed.
            out.mkdir(parents=True)
            smoke = dict(smoke_roots=args.smoke_roots, **{k: v for k, v in extra.items()})
            (out / "smoke.json").write_text(strict_json(smoke))
            print("[afterstate] V1 smoke " + json.dumps(smoke), flush=True)
            return smoke
        result = stage_result("V1", after, gaps, dict(extra, modules=list(V1_MODULES)), args, digest)
        result = seal(out, gaps, result)
        torch.save(dict(contract=FIT_CONTRACT, stage="V1", modules=list(V1_MODULES), tower=state["tower"],
                        alpha=state["alpha"], source_checkpoint=str(path), source_checkpoint_sha256=source),
                   out / "tower.pt")
    print("[afterstate] " + json.dumps(dict(stage=result["stage"], margin=result["margin"],
                                            selection_gain=result["selection_gain_per_100_roots"],
                                            falsifier=result["falsifier"])), flush=True)
    return result


if __name__ == "__main__":
    main()
