"""The search head: a per-candidate advantage over the bundle's own answer, on frozen r1 states.

    CUDA_VISIBLE_DEVICES= python -m training4.advantage fit --rows c1-roots.npz --out HEAD_DIR \
        --head mlp --tau 0.02 --seed 1

The fair long-horizon search labels each searched priority root of the bundle's
play with an advantage A_i per candidate of the payable menu (the planner-paid
"cast X" items included) over the bundle's own answer, with its standard error
SE_i, the 4-world choice and the two half-searches' choices. The head scores one
row per candidate, `[state_emb | cand_emb | tail]` (the `--embeddings` outputs of
r1's graph: the scalar head's input and `cand_scorer[3]`'s, i.e. the state the
policy's last linear reads; the tail is the coarse class one-hot, whether the
payable menu added the item, and whether it is the bundle's own answer), as
`netplay4::tactician::head` scores its rows. The bundle stays in charge: at
play an item replaces the bundle's answer only where score_i - score_own clears
the margin.

`fit` trains by weighted pairwise regression, sum_i w_i ((q_i - q_own) - A_i)^2
over the searched candidates, w_i = 1 / (SE_i^2 + tau^2), and reports on held-out
games: the rank correlation of q_i - q_own with A_i per root and pooled; top-1
agreement with the 4-world choice where it replaced the bundle's answer, by root
kind, beside the two halves' agreement on the same roots (the label's own noise,
so no fixed 80 % bar); the calibration slope of A on q_i - q_own; a margin sweep
with the summed held-out A of the overrides (margin chosen on one half of the
held-out games, reported on the other); the overrides by direction; and the
payable-added items apart from the ordinary ones. It exports `advantage.onnx`
(`rows` [N, 2d + TAIL] -> `score` [N]) with `advantage.json`.

Rows (`AdvantageRows`, one .npz) come from eval4's recordings at the roots or
from a converted dataset through the frozen net; both give the same arrays.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import time

import numpy as np
import torch
from torch import nn

from .coarse import CLASSES

CONTRACT = "search_advantage_head_v1"
ROWS_CONTRACT = "search_advantage_rows_v1"
PASS = CLASSES.index("pass")
TAIL = len(CLASSES) + 2   # class one-hot, payable-added, the bundle's own answer
MARGINS = (0.0, 0.005, 0.01, 0.02, 0.03, 0.05, 0.08, 0.12)
NONE = -1
ROOT_FIELDS = ("game", "seat", "cursor", "kind", "own", "choice4", "half_a", "half_b", "offset", "n")
CAND_FIELDS = ("A", "SE", "searched", "payable", "cls")


# The search-advantage sidecar beside a converted dataset (`search-advantage/`), one index row
# per root keyed by its twin frame (the payable menu's extra frame) and one row per menu item.
SIDECAR_CONTRACT = "search_advantage_v1"
SIDECAR_INDEX = np.dtype([("game", "<u4"), ("cursor", "<u4"), ("root_cursor", "<u4"), ("seat", "u1"),
                          ("root_kind", "u1"), ("pad0", "V2"), ("own_idx", "<u4"), ("n_cand", "<u4"),
                          ("choice4_idx", "<u4"), ("half_a_idx", "<u4"), ("half_b_idx", "<u4"),
                          ("offset", "<u8"), ("pad1", "V4")])
SIDECAR_ITEM = np.dtype([("A", "<f4"), ("SE", "<f4"), ("searched", "u1"), ("payable_added", "u1"),
                         ("cls", "u1"), ("pad", "u1")])
MISSING = 0xFFFFFFFF


class SearchAdvantage:
    """The sidecar, read fail-closed: every root's items tile the item file, its own item is
    searched with A = 0, and its indices lie on its menu."""

    def __init__(self, index, items, header):
        if index.dtype.itemsize != 48 or items.dtype.itemsize != 12:
            raise ValueError("Search-advantage rows have another width")
        n, offset = index["n_cand"].astype(np.int64), index["offset"].astype(np.int64)
        if len(items) != int(n.sum()) or not (offset == np.concatenate(([0], np.cumsum(n)[:-1]))).all():
            raise ValueError("Search-advantage items do not tile their roots")
        own = index["own_idx"].astype(np.int64)
        if ((own >= n) | (n < 1)).any():
            raise ValueError("Search-advantage root without its own answer on the menu")
        at = offset + own
        if not (items["searched"][at].astype(bool).all() and (items["A"][at] == 0).all()):
            raise ValueError("Search-advantage root whose own answer is not the searched baseline (A = 0)")
        for name in ("choice4_idx", "half_a_idx", "half_b_idx"):
            value = index[name].astype(np.int64)
            if ((value != MISSING) & (value >= n)).any():
                raise ValueError(f"Search-advantage {name} off the menu")
        self.index, self.items, self.header = index, items, header
        self.kinds = tuple(header.get("root_kinds", header.get("kinds", ())))
        from .coarse import key
        self.keys = key(index["game"], index["seat"], index["cursor"])
        self.order = np.argsort(self.keys, kind="stable")
        if (np.diff(self.keys[self.order]) == 0).any():
            raise ValueError("Search-advantage roots repeat a twin key")

    def find(self, game, seat, cursor):
        """Sidecar rows of these twin frames; -1 where none."""
        from .coarse import key
        wanted = key(game, seat, cursor)
        sorted_keys = self.keys[self.order]
        at = np.minimum(np.searchsorted(sorted_keys, wanted), len(sorted_keys) - 1)
        return np.where(sorted_keys[at] == wanted, self.order[at], -1) if len(sorted_keys) else np.full(len(wanted), -1)

    @classmethod
    def load(cls, directory):
        """`<dataset>/search-advantage/`: header.json, index.bin, cands.bin."""
        directory = Path(directory)
        header = json.loads((directory / "header.json").read_text())
        if header.get("contract") != SIDECAR_CONTRACT:
            raise ValueError(f"{directory} holds {header.get('contract')!r}, not {SIDECAR_CONTRACT}")
        index = np.fromfile(directory / "index.bin", dtype=SIDECAR_INDEX)
        items = np.fromfile(directory / "cands.bin", dtype=SIDECAR_ITEM)
        # The header describes each file: its row count, its digest, and the names its codes index.
        for name, rows, part in (("index.bin", len(index), header.get("index", {})),
                                 ("cands.bin", len(items), header.get("cands", {}))):
            if part.get("rows", rows) != rows:
                raise ValueError(f"Search-advantage header and {name} disagree on the row count")
            if "sha256" in part and hashlib.sha256((directory / name).read_bytes()).hexdigest() != part["sha256"]:
                raise ValueError(f"Search-advantage {name} does not match its header's digest")
        if list(header.get("cands", {}).get("coarse_classes", CLASSES)) != list(CLASSES):
            raise ValueError("Search-advantage coarse classes are named in another order")
        kinds = header.get("index", {}).get("root_kinds", header.get("root_kinds", ()))
        return cls(index, items, dict(header, root_kinds=list(kinds)))

    def labels(self, roots):
        """The root fields and item labels of `AdvantageRows` for sidecar rows `roots`, in order."""
        index = self.index[roots]
        n = index["n_cand"].astype(np.int64)
        items = np.concatenate([self.items[int(o):int(o) + int(k)] for o, k in zip(index["offset"], n)]) \
            if len(roots) else self.items[:0]
        none = lambda name: np.where(index[name] == MISSING, NONE, index[name].astype(np.int64))
        root_fields = dict(game=index["game"].astype(np.int64), seat=index["seat"].astype(np.int64),
                           cursor=index["root_cursor"].astype(np.int64), kind=index["root_kind"].astype(np.int64),
                           own=index["own_idx"].astype(np.int64), choice4=none("choice4_idx"),
                           half_a=none("half_a_idx"), half_b=none("half_b_idx"),
                           offset=np.concatenate(([0], np.cumsum(n)[:-1])).astype(np.int64), n=n)
        searched = items["searched"].astype(bool)
        labels = dict(A=np.where(searched, items["A"], 0.).astype(np.float32),
                      SE=np.where(searched, items["SE"], np.inf).astype(np.float32), searched=searched,
                      payable=items["payable_added"].astype(bool), cls=items["cls"].astype(np.int64))
        return root_fields, labels


class AdvantageRows:
    """Roots and their candidates: `roots[name]` [R], `state` [R, d], `cand` [N, d] and
    `labels[name]` [N]; root r's candidates are `[offset[r], offset[r] + n[r])`.
    `held_out` [R] marks the held-out games."""

    def __init__(self, roots, state, cand, labels, held_out=None, kinds=()):
        self.roots, self.state, self.cand, self.labels = roots, state, cand, labels
        self.kinds = tuple(kinds)
        self.held_out = np.zeros(len(state), dtype=bool) if held_out is None else held_out
        n, offset = roots["n"], roots["offset"]
        if len(cand) != int(n.sum()) or not (offset == np.concatenate(([0], np.cumsum(n)[:-1]))).all():
            raise ValueError("Advantage rows: candidate offsets do not tile the candidates")
        if ((roots["own"] < 0) | (roots["own"] >= n)).any():
            raise ValueError("Advantage rows: the bundle's own answer is not on its menu")
        if not labels["searched"][offset + roots["own"]].all():
            raise ValueError("Advantage rows: the bundle's own answer must be the searched baseline")

    @property
    def d_model(self):
        return self.state.shape[1]

    @classmethod
    def load(cls, path):
        data = np.load(path, allow_pickle=False)
        meta = json.loads(str(data["meta"]))
        if meta.get("contract") != ROWS_CONTRACT:
            raise ValueError(f"{path} is not {ROWS_CONTRACT}")
        rows = cls({name: data[f"root_{name}"] for name in ROOT_FIELDS}, data["state"], data["cand"],
                   {name: data[f"cand_{name}"] for name in CAND_FIELDS}, data["held_out"], meta.get("kinds", ()))
        rows.meta = meta
        return rows

    def save(self, path, **meta):
        np.savez(path, meta=json.dumps(dict(contract=ROWS_CONTRACT, kinds=list(self.kinds), **meta)),
                 state=self.state, cand=self.cand, held_out=self.held_out,
                 **{f"root_{name}": self.roots[name] for name in ROOT_FIELDS},
                 **{f"cand_{name}": self.labels[name] for name in CAND_FIELDS})

    @classmethod
    def concat(cls, parts):
        if len({p.kinds for p in parts}) != 1:
            raise ValueError("Advantage rows name their root kinds differently")
        shift = np.cumsum([0] + [len(p.cand) for p in parts[:-1]])
        roots = {name: np.concatenate([p.roots[name] + (s if name == "offset" else 0) for p, s in zip(parts, shift)])
                 for name in ROOT_FIELDS}
        return cls(roots, np.concatenate([p.state for p in parts]), np.concatenate([p.cand for p in parts]),
                   {name: np.concatenate([p.labels[name] for p in parts]) for name in CAND_FIELDS},
                   np.concatenate([p.held_out for p in parts]), parts[0].kinds)

    def tail(self):
        """[N, TAIL]: class one-hot, payable-added, the bundle's own answer."""
        out = np.zeros((len(self.cand), TAIL), dtype=np.float32)
        cls = self.labels["cls"].astype(np.int64)
        known = (cls >= 0) & (cls < len(CLASSES))
        out[np.flatnonzero(known), cls[known]] = 1.
        out[:, len(CLASSES)] = self.labels["payable"]
        out[self.roots["offset"] + self.roots["own"], len(CLASSES) + 1] = 1.
        return out

    def root_of(self):
        return np.repeat(np.arange(len(self.state)), self.roots["n"])


class SearchHead(nn.Module):
    """`linear`: one weight vector over the row (the state part cancels inside a root, so it is
    a linear head on the candidate state, A0); `mlp`: width -> hidden -> hidden -> 1, GELU."""

    def __init__(self, width, kind="mlp", hidden=0):
        super().__init__()
        if kind == "linear":
            self.net = nn.Linear(width, 1)
        else:
            h = hidden if hidden > 0 else min(width, 512)
            self.net = nn.Sequential(nn.Linear(width, h), nn.GELU(), nn.Linear(h, h), nn.GELU(), nn.Linear(h, 1))
        nn.init.zeros_(self.net.weight if kind == "linear" else self.net[-1].weight)

    def forward(self, rows):
        return self.net(rows).squeeze(-1)


class Prepared:
    """The tensors one fit reads: rows [N, width], per candidate its root, weight and target."""

    def __init__(self, rows: AdvantageRows, tau):
        root_of = rows.root_of()
        self.root_of = torch.from_numpy(root_of)
        self.rows = torch.cat((torch.from_numpy(rows.state[root_of].astype(np.float32)),
                               torch.from_numpy(rows.cand.astype(np.float32)),
                               torch.from_numpy(rows.tail())), 1)
        self.own = torch.from_numpy((rows.roots["offset"] + rows.roots["own"]).astype(np.int64))
        searched = rows.labels["searched"].astype(bool)
        is_own = np.zeros(len(rows.cand), dtype=bool)
        is_own[self.own.numpy()] = True
        self.scored = torch.from_numpy(searched & ~is_own)
        self.target = torch.from_numpy(rows.labels["A"].astype(np.float32))
        se = rows.labels["SE"].astype(np.float32)
        self.weight = torch.from_numpy(np.where(searched & ~is_own, 1. / (se ** 2 + tau ** 2), 0.).astype(np.float32))

    def gaps(self, scores):
        """[N] q_i - q_own of each candidate's root."""
        return scores - scores[self.own][self.root_of]


def pairwise_loss(prepared, scores, chosen):
    """Weighted mean of ((q_i - q_own) - A_i)^2 over the searched candidates of `chosen` roots."""
    rows = prepared.scored & chosen[prepared.root_of]
    w = torch.where(rows, prepared.weight, 0.)
    return (w * (prepared.gaps(scores) - prepared.target).square()).sum() / w.sum().clamp(min=1e-12)


def ranks(values):
    """Average ranks (ties share their mean rank)."""
    order = np.argsort(values, kind="stable")
    sorted_values = values[order]
    out = np.empty(len(values))
    start = 0
    while start < len(values):
        stop = start
        while stop + 1 < len(values) and sorted_values[stop + 1] == sorted_values[start]:
            stop += 1
        out[order[start:stop + 1]] = (start + stop) / 2.
        start = stop + 1
    return out


def correlation(x, y, w=None):
    w = np.ones_like(x) if w is None else w
    if len(x) < 2 or w.sum() <= 0:
        return None
    mx, my = np.average(x, weights=w), np.average(y, weights=w)
    sx, sy = np.average((x - mx) ** 2, weights=w), np.average((y - my) ** 2, weights=w)
    return None if sx <= 0 or sy <= 0 else float(np.average((x - mx) * (y - my), weights=w) / np.sqrt(sx * sy))


def report(rows: AdvantageRows, gaps, weight, chosen, margin=None):
    """Held-out metrics of one head's gaps (q_i - q_own) on the `chosen` roots."""
    gaps, weight = np.asarray(gaps, dtype=np.float64), np.asarray(weight, dtype=np.float64)
    A = rows.labels["A"].astype(np.float64)
    searched = rows.labels["searched"].astype(bool)
    payable = rows.labels["payable"].astype(bool)
    cls = rows.labels["cls"].astype(np.int64)
    roots = rows.roots
    per_root, kinds, overrides = [], {}, {}
    search_gain = 0.   # the labels' own 4-world choice, in-sample to its worlds (biased upward)
    sweep = {m: [] for m in MARGINS}   # per root: (game, A of the override or 0)
    pairs = dict(rows=[], weight=[], payable=[])
    for r in np.flatnonzero(chosen):
        lo, n, own = int(roots["offset"][r]), int(roots["n"][r]), int(roots["own"][r])
        items = np.flatnonzero(searched[lo:lo + n])
        g, a = gaps[lo + items], A[lo + items]
        a = np.where(items == own, 0., a)
        if len(items) >= 3 and np.ptp(a) > 0 and np.ptp(g) > 0:
            per_root.append(correlation(ranks(g), ranks(a)))
        others = items[items != own]
        pairs["rows"].extend((lo + others).tolist())
        best = others[np.argmax(gaps[lo + others])] if len(others) else None
        kind = rows.kinds[int(roots["kind"][r])] if 0 <= int(roots["kind"][r]) < len(rows.kinds) else str(int(roots["kind"][r]))
        cell = kinds.setdefault(kind, dict(roots=0, changed=0, student_agrees=0, halves=0, halves_agree=0))
        cell["roots"] += 1
        choice4, half_a, half_b = int(roots["choice4"][r]), int(roots["half_a"][r]), int(roots["half_b"][r])
        top = own if best is None or gaps[lo + best] <= 0 else int(best)
        if choice4 != NONE and choice4 != own:
            cell["changed"] += 1
            cell["student_agrees"] += int(top == choice4)
            search_gain += float(A[lo + choice4])
        if half_a != NONE and half_b != NONE and (half_a != own or half_b != own):
            cell["halves"] += 1
            cell["halves_agree"] += int(half_a == half_b)
        for m in MARGINS:
            take = best is not None and gaps[lo + best] > m
            sweep[m].append((int(roots["game"][r]), A[lo + best] if take else 0., take))
        if margin is not None and best is not None and gaps[lo + best] > margin:
            key = direction(cls[lo + own], cls[lo + best], payable[lo + best])
            overrides[key] = overrides.get(key, 0) + 1
    idx = np.array(pairs["rows"], dtype=np.int64)
    calib = lambda sel: (float((weight[sel] * gaps[sel] * A[sel]).sum() / (weight[sel] * gaps[sel] ** 2).sum())
                         if sel.size and (weight[sel] * gaps[sel] ** 2).sum() > 0 else None)
    split = {}
    for name, sel in (("ordinary", idx[~payable[idx]] if idx.size else idx),
                      ("payable_added", idx[payable[idx]] if idx.size else idx)):
        split[name] = dict(candidates=int(sel.size), pearson=correlation(gaps[sel], A[sel], weight[sel]) if sel.size else None,
                           calibration_slope=calib(sel))
    out = dict(
        roots=int(chosen.sum()), candidates=int(idx.size),
        spearman_per_root=float(np.mean(per_root)) if per_root else None, spearman_roots=len(per_root),
        pearson_pooled=correlation(gaps[idx], A[idx], weight[idx]) if idx.size else None,
        calibration_slope=calib(idx), by_kind={}, margins={}, items=split,
        search_choice4_A_per_100_roots=search_gain / max(int(chosen.sum()), 1) * 100)
    for kind, cell in sorted(kinds.items()):
        out["by_kind"][kind] = dict(cell, top1_vs_choice4=cell["student_agrees"] / cell["changed"] if cell["changed"] else None,
                                    halves_same_answer=cell["halves_agree"] / cell["halves"] if cell["halves"] else None)
    for m, picks in sweep.items():
        games = {}
        for game, a, _ in picks:
            games[game] = games.get(game, 0.) + a
        x = np.array(list(games.values()))
        n = max(len(picks), 1)
        se = float(np.sqrt(len(x) * x.var(ddof=1)) / n * 100) if len(x) > 1 else None
        out["margins"][str(m)] = dict(overrides=int(sum(t for *_, t in picks)), gain_per_100_roots=float(x.sum() / n * 100),
                                      se=se)
    if margin is not None:
        out.update(margin=margin, overrides=overrides)
    return out


def direction(own_cls, chosen_cls, payable):
    """pass->act, act->pass, tap->cast (payable-added or ordinary), land<->cast, other."""
    name = lambda c: CLASSES[c] if 0 <= c < len(CLASSES) else "none"
    own, chosen = name(int(own_cls)), name(int(chosen_cls))
    if own == "pass" and chosen != "pass":
        return "pass->act"
    if own != "pass" and chosen == "pass":
        return "act->pass"
    if own == "tap" and chosen == "cast":
        return "tap->cast (payable-added)" if payable else "tap->cast (ordinary)"
    if {own, chosen} == {"land", "cast"}:
        return "land<->cast"
    return f"{own}->{chosen}"


def held_out_halves(rows):
    """The held-out games split in two, alternating in game id order (ids held out by a stride
    share their parity, so parity would put every game on one side): margin selection, report."""
    games = rows.roots["game"]
    held = np.unique(games[rows.held_out])
    report_games = held[1::2]
    in_report = np.isin(games, report_games)
    return rows.held_out & ~in_report, rows.held_out & in_report


def fit(rows: AdvantageRows, kind="mlp", hidden=0, tau=0.02, lr=1e-3, weight_decay=1e-2, epochs=40, batch=512,
        patience=6, seed=1, log=print, train_fraction=1.0):
    """`train_fraction` < 1 trains on that share of the training games only (a learning curve);
    the held-out games and the report stay the same."""
    torch.manual_seed(seed)
    rng = np.random.default_rng(seed)
    prepared = Prepared(rows, tau)
    head = SearchHead(prepared.rows.shape[1], kind, hidden)
    optimizer = torch.optim.AdamW(head.parameters(), lr=lr, weight_decay=weight_decay)
    train = np.flatnonzero(~rows.held_out)
    if train_fraction < 1:
        games = np.unique(rows.roots["game"][train])
        kept = rng.choice(games, max(1, int(round(len(games) * train_fraction))), replace=False)
        train = train[np.isin(rows.roots["game"][train], kept)]
    held = torch.from_numpy(rows.held_out)
    best, best_state, history, waited = None, None, [], 0
    for epoch in range(1, epochs + 1):
        head.train()
        order = rng.permutation(train)
        for start in range(0, len(order), batch):
            chosen = torch.zeros(len(rows.state), dtype=torch.bool)
            chosen[torch.from_numpy(order[start:start + batch])] = True
            members = chosen[prepared.root_of]
            scores = torch.zeros(len(prepared.rows))
            scores[members] = head(prepared.rows[members])
            loss = pairwise_loss(prepared, scores, chosen)
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            optimizer.step()
        head.eval()
        with torch.no_grad():
            scores = head(prepared.rows)
            train_loss = float(pairwise_loss(prepared, scores, ~held))
            held_loss = float(pairwise_loss(prepared, scores, held))
        history.append(dict(epoch=epoch, train=train_loss, held_out=held_loss))
        log(f"[advantage] epoch={epoch} train={train_loss:.6f} held_out={held_loss:.6f}")
        if best is None or held_loss < best:
            best, best_state, waited = held_loss, {k: v.clone() for k, v in head.state_dict().items()}, 0
        else:
            waited += 1
            if waited >= patience:
                break
    head.load_state_dict(best_state)
    head.eval()
    with torch.no_grad():
        gaps = prepared.gaps(head(prepared.rows)).numpy()
    weight = prepared.weight.numpy()
    select, final = held_out_halves(rows)
    chooser = report(rows, gaps, weight, select)
    best_gain = max(v["gain_per_100_roots"] for v in chooser["margins"].values())
    margin = max(float(m) for m, v in chooser["margins"].items() if v["gain_per_100_roots"] == best_gain)
    zero = torch.zeros(len(prepared.rows))
    return head, dict(history=history, margin=margin, margin_selection=chooser,
                      zero_head_loss=dict(train=float(pairwise_loss(prepared, zero, ~held)),
                                          held_out=float(pairwise_loss(prepared, zero, held))),
                      held_out=report(rows, gaps, weight, final, margin),
                      train=report(rows, gaps, weight, ~rows.held_out, margin))


def export(head, width, path):
    import onnx  # noqa: F401  (the exporter needs it)
    head = head.cpu().eval()
    torch.onnx.export(head, (torch.zeros(3, width),), str(path), input_names=["rows"], output_names=["score"],
                      dynamic_axes={"rows": {0: "N"}, "score": {0: "N"}}, opset_version=18, dynamo=False)


class RootPass:
    """One forward of frozen twin frames from their roots' memory, keeping what the head reads:
    the scalar head's input (`state_emb`) and `cand_scorer[3]`'s (`cand_emb`), as r1's graph
    exports them with `--embeddings`."""

    def __init__(self, net, board_chunk, thinking_steps, device=torch.device("cpu")):
        self.net, self.board_chunk, self.thinking_steps, self.device = net, board_chunk, thinking_steps, device

    @torch.no_grad()
    def __call__(self, frames, hidden):
        import dataset4 as D4
        from .deferred import deferred_readouts
        from .execution import encode_boards, plan_batch, transfer_batch
        items = [{**{name: value[None] for name, value in frame.items()}, "step_valid": np.array([True]),
                  "is_new_traj": np.array(True), "segment_index": np.array(-1, dtype=np.int64)} for frame in frames]
        host = D4.collate_trajectories(items)
        plans = plan_batch(host, 0, self.thinking_steps)
        batch = transfer_batch(host, self.device)
        seen = {"state": [], "cand": []}
        keep = lambda name: (lambda _module, args: seen[name].append(args[0].detach().float().cpu()))
        handles = [self.net.scalar_head.register_forward_pre_hook(keep("state")),
                   self.net.cand_scorer[3].register_forward_pre_hook(keep("cand"))]
        try:
            self.net.clear_structure()
            self.net.prepare_structure((host["entity_rules"], host["cand_rules"]))
            boards = encode_boards(self.net, batch, self.board_chunk, self.device)
            deferred_readouts(self.net, batch, boards, hidden.to(self.device), plans, self.net.decision_chunk,
                              self.device)
        finally:
            for handle in handles:
                handle.remove()
            self.net.clear_structure()
        if len(seen["state"]) != 1 or len(seen["cand"]) != 1:
            raise RuntimeError("A root pass read the heads more than once")
        widths = [int(frame["cand_mask"].shape[0]) for frame in frames]
        return seen["state"][0], [seen["cand"][0][i, :width] for i, width in enumerate(widths)]


def held_out_of(split, games, every=0):
    """Held out: the val split's games, and with `every` > 0 the games whose id is a multiple of it."""
    games = np.asarray(games)
    return np.full(len(games), split == "val") | ((games % every == 0) if every > 0 else False)


def capture(args):
    """Rows from a converted dataset: every twin frame of `search-advantage-menus` joins its sidecar
    row and its root frame (same game, seat, cursor) in train/ or val/; the root's memory is the
    stored window state of the piece holding the root frame (burn-in 0: a piece is a run of
    decision frames, which never move memory), computed once by a state pass (`--device` cpu or cuda, fp32 either way)."""
    import dataset4 as D4
    from torch_io4 import file_digest
    from trajectory_store4 import TrajectoryStore4
    from value_calibration4.checkpoint import load_source
    from .coarse import key
    from .windows import WindowTraining

    torch.set_num_threads(args.threads)
    device = torch.device(args.device)
    path, _, net = load_source(args.model)
    net = net.to(device)   # fp32 throughout, on the CPU or the GPU alike
    net.decision_chunk, net.board_token_budget = args.decision_chunk, args.board_token_budget
    net.prune_unused_policy_heads, net.gpu_prefetch = False, 0
    data = Path(args.data).expanduser()
    meta = json.loads((data / "meta.json").read_text())
    sidecar = SearchAdvantage.load(data / "search-advantage")
    menus = TrajectoryStore4(data, "search-advantage-menus", meta["search_advantage_menus_shards"])
    twin = menus.index
    twin_rows = sidecar.find(twin["game"], twin["seat"], twin["cursor"])
    if (twin_rows < 0).any() or len(np.unique(twin_rows)) != len(sidecar.index) or len(twin_rows) != len(sidecar.index):
        raise ValueError("Search-advantage twins and sidecar rows do not match one to one")
    passer, source = RootPass(net, args.board_chunk, args.thinking_steps, device), file_digest(path)
    parts, counts = [], {}
    for split in args.splits:
        dataset = D4.TrajectoryDataset4(data, split=split, unroll_len=32, encoder_version=args.encoder_version)
        kept_games = None
        if args.max_games:
            # The first games by id: only their trajectories get a state pass and only their twins a row.
            store_games = dataset.store.index["game"]
            game_of = [int(store_games[dataset.segments[t[0]][0]]) for t in dataset.trajectories]
            kept_games = np.array(sorted(set(game_of))[:args.max_games])
            dataset.trajectories = [t for t, g in zip(dataset.trajectories, game_of) if g in set(kept_games.tolist())]
        plan = WindowTraining(dataset, 0, 32, 8, 0, 0)
        began = time.perf_counter()
        origin = plan.refresh(net, device, args.lanes, args.thinking_steps, False, args.workers,
                              args.board_token_budget, cache=Path(args.state_dir).expanduser() / f"{split}.json",
                              source=source, seats=2)
        print(f"[capture] split={split} states={origin} seconds={time.perf_counter() - began:.0f}", flush=True)
        # Each store row's piece (decision frames only; -1 elsewhere).
        piece_of = np.full(len(dataset.store.index), -1, dtype=np.int64)
        for pid, (trajectory, start, length) in enumerate(plan.pieces.tolist()):
            piece_of[plan.rows[trajectory][start:start + length]] = pid
        index = dataset.store.index
        keys = key(index["game"], index["seat"], index["cursor"])
        order = np.argsort(keys, kind="stable")
        wanted = key(twin["game"], twin["seat"], twin["cursor"])
        at = np.minimum(np.searchsorted(keys[order], wanted), len(order) - 1)
        here = np.flatnonzero(keys[order][at] == wanted)
        if kept_games is not None:
            here = here[np.isin(twin["game"][here], kept_games)]
        roots_rows = order[at[here]]
        if (piece_of[roots_rows] < 0).any():
            raise ValueError("A searched root is not a decision frame of its split")
        states, cands = [], []
        for start in range(0, len(here), args.batch):
            chunk = here[start:start + args.batch]
            frames = [dataset._encode_bst4(menus.read_raw(int(row))) for row in chunk]
            for row, frame in zip(chunk, frames):
                side = sidecar.index[twin_rows[row]]
                if int(frame.get("twin_root_cursor", -1)) != int(side["root_cursor"]) or \
                        int(frame["cand_mask"].shape[0]) != int(side["n_cand"]):
                    raise ValueError(f"Twin frame and sidecar row disagree (game {int(side['game'])}, "
                                     f"cursor {int(side['cursor'])})")
            hidden = plan.states.read(piece_of[roots_rows[start:start + args.batch]])
            state, cand = passer(frames, hidden)
            states.append(state.half().numpy())
            cands.extend(c.half().numpy() for c in cand)
            if (start // args.batch) % 10 == 0:
                print(f"[capture] split={split} roots={start + len(chunk)}/{len(here)}", flush=True)
        plan.close()
        roots, labels = sidecar.labels(twin_rows[here])
        parts.append(AdvantageRows(roots, np.concatenate(states) if states else np.zeros((0, net.cfg.d_model), np.float16),
                                   np.concatenate(cands) if cands else np.zeros((0, net.cfg.d_model), np.float16),
                                   labels, held_out_of(split, roots["game"], args.hold_out_every), sidecar.kinds))
        counts[split] = int(len(here))
    rows = AdvantageRows.concat(parts)
    counts["twins"], counts["unplaced"] = int(len(twin_rows)), int(len(twin_rows) - sum(counts[s] for s in args.splits))
    rows.save(args.out, data=str(data), model=str(path), model_sha256=source, splits=list(args.splits), counts=counts,
              device=str(device), runtime=args.runtime, trainer_source=str(Path(__file__).resolve().parents[3]),
              max_games=args.max_games, hold_out_every=args.hold_out_every,
              sidecar=sidecar.header)
    print("[capture] " + json.dumps(counts), flush=True)
    return rows


def smoke(args):
    """A hand-set smoke head for the play path: score = payable-added (0/1), margin 0, so the
    student plays the first payable-added macro item wherever one is offered. Not a fitted head:
    its json says `smoke: true` and it must never be gated."""
    export_meta = json.loads((Path(args.model).expanduser() / "fp16" / "model.onnx.json").read_text())
    d_model, source = int(export_meta["d_model"]), export_meta["source_checkpoint_sha256"]
    out = Path(args.out)
    if out.exists():
        raise FileExistsError(f"{out} exists; a head gets its own directory")
    width = 2 * d_model + TAIL
    head = SearchHead(width, "linear")
    with torch.no_grad():
        head.net.weight.zero_()
        head.net.bias.zero_()
        head.net.weight[0, 2 * d_model + len(CLASSES)] = 1.   # the payable-added flag
    out.mkdir(parents=True)
    export(head, width, out / "advantage.onnx")
    meta = dict(contract=CONTRACT, d_model=d_model, row_tail=TAIL, width=width,
                tail=["class:" + name for name in CLASSES] + ["payable_added", "own"],
                source_checkpoint_sha256=source, margin=0.0, head="linear", smoke=True,
                note="SMOKE HEAD, hand-set: score = payable_added; tests the play path only, never gate it",
                rows_layout="[state_emb (d) | cand_emb (d) | tail], float32; tail: coarse_class_v1 one-hot "
                            "(all zero for an unknown class), payable_added 0/1, own 0/1",
                override="argmax over the scored items of score_i - score_own; play it when it exceeds margin",
                onnx_sha256=hashlib.sha256((out / "advantage.onnx").read_bytes()).hexdigest())
    (out / "advantage.json").write_text(json.dumps(meta, indent=1))
    print("[advantage] smoke head " + json.dumps(dict(out=str(out), d_model=d_model, source=source)), flush=True)
    return meta


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    smoking = commands.add_parser("smoke", help="A hand-set head for testing the play path")
    smoking.add_argument("--model", required=True, help="The net's directory (its fp16/model.onnx.json)")
    smoking.add_argument("--out", required=True)
    grabbing = commands.add_parser("capture")
    grabbing.add_argument("--data", required=True, help="A converted dataset with search-advantage-menus and its sidecar")
    grabbing.add_argument("--model", required=True, help="The frozen net (r1)")
    grabbing.add_argument("--out", required=True, help=f"The {ROWS_CONTRACT} .npz to write")
    grabbing.add_argument("--state-dir", required=True, help="Where the CPU window-state caches go")
    grabbing.add_argument("--splits", default="train,val")
    grabbing.add_argument("--hold-out-every", type=int, default=0,
                          help="Also hold out the games whose id is a multiple of K (0: only the val split)")
    grabbing.add_argument("--max-games", type=int, default=0, help="Only the first N games by id (0: all)")
    grabbing.add_argument("--batch", type=int, default=32)
    grabbing.add_argument("--lanes", type=int, default=16)
    grabbing.add_argument("--workers", type=int, default=2)
    grabbing.add_argument("--board-chunk", type=int, default=32)
    grabbing.add_argument("--decision-chunk", type=int, default=4)
    grabbing.add_argument("--board-token-budget", type=int, default=32768)
    grabbing.add_argument("--thinking-steps", type=int, default=2)
    grabbing.add_argument("--encoder-version", type=int, default=10)
    grabbing.add_argument("--threads", type=int, default=8)
    grabbing.add_argument("--device", default="cpu", help="cpu or cuda (inference only, fp32)")
    grabbing.add_argument("--runtime", default="", help="A sealed runtime directory put first on PATH (trajectory_cache4, train_ops)")
    fitting = commands.add_parser("fit")
    fitting.add_argument("--rows", action="append", required=True, help=f"{ROWS_CONTRACT} .npz files")
    fitting.add_argument("--out", required=True, help="A new directory for the head")
    fitting.add_argument("--head", choices=("linear", "mlp"), default="mlp")
    fitting.add_argument("--hidden", type=int, default=0)
    fitting.add_argument("--tau", type=float, default=0.02, help="The weights' floor: w = 1 / (SE^2 + tau^2)")
    fitting.add_argument("--lr", type=float, default=1e-3)
    fitting.add_argument("--weight-decay", type=float, default=1e-2)
    fitting.add_argument("--epochs", type=int, default=40)
    fitting.add_argument("--batch", type=int, default=512, help="Roots per step")
    fitting.add_argument("--patience", type=int, default=6)
    fitting.add_argument("--seed", type=int, required=True)
    fitting.add_argument("--train-fraction", type=float, default=1.0,
                         help="Train on this share of the training games (a learning curve); held-out unchanged")
    fitting.add_argument("--threads", type=int, default=8)
    args = parser.parse_args(argv)
    if args.command == "smoke":
        return smoke(args)
    if args.command == "capture":
        args.splits = tuple(s for s in args.splits.split(",") if s)
        if args.runtime:
            # The sealed runtime's native tools first, for this process and its loader workers.
            import os
            runtime = Path(args.runtime).expanduser()
            if not (runtime / "trajectory_cache4").is_file() or not (runtime / "train_ops").is_file():
                raise FileNotFoundError(f"{runtime} lacks trajectory_cache4 or train_ops")
            os.environ["PATH"] = f"{runtime}{os.pathsep}{os.environ.get('PATH', '')}"
        return capture(args)
    torch.set_num_threads(args.threads)
    out = Path(args.out)
    if out.exists():
        raise FileExistsError(f"{out} exists; a head gets its own directory")
    parts = [AdvantageRows.load(path) for path in args.rows]
    # The head reads one net's embeddings: every rows file must come from the same checkpoint.
    sources = {p.meta.get("model_sha256") for p in parts}
    if len(sources) != 1:
        raise ValueError(f"The rows come from different nets: {sorted(map(str, sources))}")
    rows = AdvantageRows.concat(parts)
    head, result = fit(rows, args.head, args.hidden, args.tau, args.lr, args.weight_decay, args.epochs, args.batch,
                       args.patience, args.seed, train_fraction=args.train_fraction)
    out.mkdir(parents=True)
    width = 2 * rows.d_model + TAIL
    export(head, width, out / "advantage.onnx")
    torch.save(dict(state_dict=head.state_dict(), width=width, kind=args.head, hidden=args.hidden), out / "head.pt")
    meta = dict(contract=CONTRACT, d_model=rows.d_model, row_tail=TAIL, width=width,
                tail=["class:" + name for name in CLASSES] + ["payable_added", "own"],
                # The play side checks it against the playing graph's model.onnx.json.
                source_checkpoint_sha256=next(iter(sources)),
                rows_layout="[state_emb (d) | cand_emb (d) | tail], float32; tail: coarse_class_v1 one-hot "
                            "(all zero for an unknown class), payable_added 0/1, own 0/1",
                override="argmax over the scored items of score_i - score_own; play it when it exceeds margin",
                head=args.head, hidden=args.hidden, tau=args.tau, seed=args.seed, lr=args.lr,
                train_fraction=args.train_fraction,
                rows=[dict(path=str(p), sha256=hashlib.sha256(Path(p).read_bytes()).hexdigest()) for p in args.rows],
                roots=dict(train=int((~rows.held_out).sum()), held_out=int(rows.held_out.sum())),
                onnx_sha256=hashlib.sha256((out / "advantage.onnx").read_bytes()).hexdigest(), **result)
    (out / "advantage.json").write_text(json.dumps(meta, indent=1))
    print("[advantage] " + json.dumps(dict(margin=result["margin"], held_out=result["held_out"])), flush=True)
    return meta


if __name__ == "__main__":
    main()
