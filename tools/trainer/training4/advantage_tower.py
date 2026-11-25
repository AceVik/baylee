"""Arm B of the search head: a trainable copy of r1's decision readout under the head.

    python -m training4.advantage_tower fit --rows rows-c1.npz --rows rows-c2.npz \
        --state-dir STATES_C1 --state-dir STATES_C2 --model R1_DIR --arm B1 --out DIR \
        --device cuda --seed 1 --minutes 28 --preregistration PREREG.json
    python -m training4.advantage_tower decide --preregistration PREREG.json \
        --arm B1=DIR_B1 --arm B2=DIR_B2 --out decision.json

The frozen fits (`training4.advantage`) showed that r1's embeddings carry little of the
search's advantage although the labels would allow much more. Here the readout that makes
those embeddings trains too. The tower is a copy of r1 in which only the arm's modules train:

- B1: the candidate scorer before its last linear and the menu context (`cand_scorer.0`,
  `cand_scorer.1`, `cand_ctx_proj`);
- B2: B1 plus the thinking block, `h_norm` and the candidate projections (`thinking_block`,
  `h_norm`, `cand_proj`, `dsl_proj`).

The board encoder runs in r1 itself, frozen and without gradients; each root's memory is the
state the capture stored (its cache read only, never computed here). r1 is never changed, so
where the head does not override, the bundle plays exactly as before. The head reads the
tower's `state_emb` (the scalar head's input) and `cand_emb` (`cand_scorer[3]`'s) as the
frozen head read r1's. Loss, weights, the held-out halves, the margin and every report are the
frozen fit's; early stopping and the margin read only the selection half, and the report half
is read once, at the end.

`decide` applies the pre-registered rule: it picks the arm on the selection half and judges
only that arm on the report half.
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

CONTRACT = "search_advantage_tower_v1"
DECISION_CONTRACT = "search_advantage_tower_decision_v1"
PREREGISTRATION_CONTRACT = "searchadv_armb_preregistration_v1"
ARMS = {
    "B1": ("cand_scorer.0", "cand_scorer.1", "cand_ctx_proj"),
    "B2": ("cand_scorer.0", "cand_scorer.1", "cand_ctx_proj", "thinking_block", "h_norm", "cand_proj", "dsl_proj"),
}


def make_tower(net, arm):
    """A copy of `net` in which only `arm`'s modules train; `net` itself is frozen."""
    if arm not in ARMS:
        raise ValueError(f"Unknown arm {arm!r}; arms: {sorted(ARMS)}")
    for parameter in net.parameters():
        parameter.requires_grad_(False)
    tower = copy.deepcopy(net)
    # No recompute in backward: the head's inputs are read by hooks inside the readout.
    tower.cfg.activation_checkpointing = False
    for name in ARMS[arm]:
        for parameter in tower.get_submodule(name).parameters():   # a missing module raises
            parameter.requires_grad_(True)
    return tower


def trainable_state(tower, arm):
    """The arm's weights, by their full names in the tower (a copy on the CPU)."""
    return {f"{name}.{key}": value.detach().cpu().clone()
            for name in ARMS[arm] for key, value in tower.get_submodule(name).state_dict().items()}


def load_trainable(tower, arm, state):
    expected = set(trainable_state(tower, arm))
    if set(state) != expected:
        raise ValueError("Tower weights do not name exactly the arm's modules")
    missing, unexpected = tower.load_state_dict(state, strict=False)
    if unexpected or expected & set(missing):
        raise ValueError("Tower weights do not load into the tower")


class TowerPass:
    """Twin frames from their roots' memory: r1's board encoder (frozen, no gradient), then the
    tower's readout, keeping the head's inputs with their graph. Gradients reach the tower only
    while autograd is on."""

    def __init__(self, frozen, tower, board_chunk, thinking_steps, device=torch.device("cpu")):
        self.frozen, self.tower = frozen, tower
        self.board_chunk, self.thinking_steps, self.device = board_chunk, thinking_steps, device

    def __call__(self, frames, hidden):
        import dataset4 as D4
        from .deferred import deferred_readouts
        from .execution import encode_boards, plan_batch, transfer_batch
        items = [{**{name: value[None] for name, value in frame.items()}, "step_valid": np.array([True]),
                  "is_new_traj": np.array(True), "segment_index": np.array(-1, dtype=np.int64)} for frame in frames]
        host = D4.collate_trajectories(items)
        if not bool(np.asarray(host["is_decision"]).all()):
            raise ValueError("A root's twin frame is not a decision frame")
        plans = plan_batch(host, 0, self.thinking_steps)
        batch = transfer_batch(host, self.device)
        seen = {"state": [], "cand": []}
        keep = lambda name: (lambda _module, args: seen[name].append(args[0]))
        handles = [self.tower.scalar_head.register_forward_pre_hook(keep("state")),
                   self.tower.cand_scorer[3].register_forward_pre_hook(keep("cand"))]
        try:
            for net in (self.frozen, self.tower):
                net.clear_structure()
                net.prepare_structure((host["entity_rules"], host["cand_rules"]))
            with torch.no_grad():
                boards = encode_boards(self.frozen, batch, self.board_chunk, self.device)
            deferred_readouts(self.tower, batch, boards, hidden.to(self.device), plans, self.tower.decision_chunk,
                              self.device)
        finally:
            for handle in handles:
                handle.remove()
            self.frozen.clear_structure()
            self.tower.clear_structure()
        if len(seen["state"]) != 1 or len(seen["cand"]) != 1:
            raise RuntimeError("A tower pass read the heads more than once")
        widths = [int(frame["cand_mask"].shape[0]) for frame in frames]
        return seen["state"][0].float(), [seen["cand"][0][i, :width].float() for i, width in enumerate(widths)]


class Roots:
    """Per root of the rows, in their order: its decoded twin frame and its stored memory."""

    def __init__(self, frames, hidden):
        self.frames, self.hidden = frames, hidden

    @classmethod
    def load(cls, parts, state_dirs, net, source, thinking_steps=2, encoder_version=10, log=print, keeps=None):
        """Each rows file's dataset, read as the capture read it; the window-state cache must exist
        (read only) and the roots must come out exactly as the rows hold them. `keeps` (one [R]
        mask per rows file) loads only those roots, for smoke tests."""
        import dataset4 as D4
        from trajectory_store4 import TrajectoryStore4
        from .windows import WindowTraining
        if len(state_dirs) != len(parts):
            raise ValueError("One --state-dir per --rows file, in the same order")
        keeps = [np.ones(len(p.state), dtype=bool) for p in parts] if keeps is None else keeps
        frames, hidden = [], []
        for part, state_dir, keep in zip(parts, state_dirs, keeps):
            if part.meta.get("model_sha256") != source:
                raise ValueError("The rows were captured from another net")
            splits = part.meta.get("splits", ["train"])
            data = Path(part.meta["data"])
            meta = json.loads((data / "meta.json").read_text())
            sidecar = V.SearchAdvantage.load(data / "search-advantage")
            menus = TrajectoryStore4(data, "search-advantage-menus", meta["search_advantage_menus_shards"])
            twin = menus.index
            twin_rows = sidecar.find(twin["game"], twin["seat"], twin["cursor"])
            if (twin_rows < 0).any():
                raise ValueError("Search-advantage twins and sidecar rows do not match")
            pos = 0   # the next root of this rows file
            for split in splits:
                cache = Path(state_dir) / f"{split}.json"
                if not cache.is_file():
                    raise FileNotFoundError(f"{cache}: the capture's window-state cache is missing; the tower never computes it")
                dataset = D4.TrajectoryDataset4(data, split=split, unroll_len=32, encoder_version=encoder_version)
                V.check_catalog(dataset.structure.digest, net, f"The rows' dataset {data}")
                max_games = int(part.meta.get("max_games", 0) or 0)
                kept_games = None
                if max_games:
                    store_games = dataset.store.index["game"]
                    game_of = [int(store_games[dataset.segments[t[0]][0]]) for t in dataset.trajectories]
                    kept_games = np.array(sorted(set(game_of))[:max_games])
                    dataset.trajectories = [t for t, g in zip(dataset.trajectories, game_of)
                                            if g in set(kept_games.tolist())]
                plan = WindowTraining(dataset, 0, 32, 8, 0, 0)
                origin = plan.refresh(net, torch.device("cpu"), 1, thinking_steps, False, 0, 0, cache=cache,
                                      source=source, seats=2)
                if origin != "cache":
                    raise RuntimeError("The window states were not read from the capture's cache")
                here, roots_rows, piece_of = V.locate_roots(dataset, plan, twin, kept_games)
                mine = keep[pos:pos + len(here)]
                if len(mine) != len(here):
                    raise ValueError(f"{data.name}: the dataset gives more roots than the rows hold")
                hidden.append(torch.as_tensor(plan.states.read(piece_of[roots_rows[mine]])).float().cpu().clone())
                plan.close()
                began, first = time.perf_counter(), None
                for row in here[mine]:
                    frame = dataset._encode_bst4(menus.read_raw(int(row)))
                    first = time.perf_counter() - began if first is None else first
                    side = sidecar.index[twin_rows[row]]
                    if int(frame.get("twin_root_cursor", -1)) != int(side["root_cursor"]) or \
                            int(frame["cand_mask"].shape[0]) != int(side["n_cand"]):
                        raise ValueError("Twin frame and sidecar row disagree")
                    frames.append(frame)
                size = sum(np.asarray(v).nbytes for frame in frames[len(frames) - int(mine.sum()):] for v in frame.values())
                log(f"[tower] {data.name}/{split}: {int(mine.sum())} roots decoded in {time.perf_counter() - began:.0f} s "
                    f"(the first {first or 0.:.1f} s), {size / 2**20:.0f} MiB")
                roots, labels = sidecar.labels(twin_rows[here])
                for name in ("game", "seat", "cursor", "own", "n"):
                    if not np.array_equal(roots[name], part.roots[name][pos:pos + len(here)]):
                        raise ValueError(f"{data.name}: the roots' {name} differ from the rows'")
                first = int(part.roots["offset"][pos]) if len(here) else 0
                if not np.array_equal(labels["A"], part.labels["A"][first:first + len(labels["A"])]):
                    raise ValueError(f"{data.name}: the labels differ from the rows'")
                pos += len(here)
            if pos != len(part.state):
                raise ValueError(f"{data.name}: the dataset gives {pos} roots, the rows hold {len(part.state)}")
        hidden = torch.cat(hidden) if hidden else torch.zeros(0)
        if len(frames) != len(hidden) or len(frames) != sum(int(k.sum()) for k in keeps):
            raise ValueError("The datasets give another number of roots than the rows")
        return cls(frames, hidden)


class Batches:
    """The head's rows for a batch of roots, with the tower's embeddings and the labels' tail."""

    def __init__(self, rows, roots: Roots, passer, head, tau, device):
        self.rows, self.roots, self.passer, self.head, self.device = rows, roots, passer, head, device
        target, weight, tail = V.item_labels(rows, tau)
        self.target, self.weight = torch.from_numpy(target), torch.from_numpy(weight)
        self.tail = torch.from_numpy(tail)
        self.offset, self.n, self.own = (rows.roots[k].astype(np.int64) for k in ("offset", "n", "own"))

    def items(self, chosen):
        return np.concatenate([np.arange(self.offset[r], self.offset[r] + self.n[r]) for r in chosen])

    def scores(self, chosen):
        """[items] the head's scores of the chosen roots' candidates, in root order."""
        state, cands = self.passer([self.roots.frames[r] for r in chosen],
                                   self.roots.hidden[torch.from_numpy(np.asarray(chosen, dtype=np.int64))])
        parts = []
        for i, r in enumerate(chosen):
            n = int(self.n[r])
            if cands[i].shape[0] != n:
                raise ValueError("A root's menu has another width than its rows")
            tail = self.tail[self.offset[r]:self.offset[r] + n].to(self.device)
            parts.append(torch.cat((state[i].expand(n, -1), cands[i], tail), 1))
        return self.head(torch.cat(parts))

    def gaps(self, chosen, scores):
        n = torch.from_numpy(self.n[chosen])
        local = torch.repeat_interleave(torch.arange(len(chosen)), n).to(scores.device)
        own = (torch.cumsum(n, 0) - n + torch.from_numpy(self.own[chosen])).to(scores.device)
        return scores - scores[own][local]

    def loss(self, chosen, gaps):
        """The frozen fit's pairwise loss over these roots."""
        items = torch.from_numpy(self.items(chosen))
        w, target = self.weight[items].to(gaps.device), self.target[items].to(gaps.device)
        return (w * (gaps - target).square()).sum() / w.sum().clamp(min=1e-12)


def pairwise_on(rows, gaps, weight_target, chosen):
    """The pairwise loss of full-size `gaps` over `chosen` roots (numpy)."""
    target, weight = weight_target
    in_root = np.isin(rows.root_of(), np.flatnonzero(chosen))
    w = np.where(in_root, weight, 0.)
    return float((w * (gaps - target) ** 2).sum() / max(w.sum(), 1e-12))


def fit(rows, roots: Roots, frozen, arm, device, kind="mlp", hidden=0, tau=0.02, head_lr=1e-3, tower_lr=3e-5,
        weight_decay=1e-2, epochs=25, batch=32, patience=4, seed=1, minutes=0., thinking_steps=2, board_chunk=32,
        log=print):
    """Train the tower's arm and the head on the training roots; stop early on the selection half's
    loss and within `minutes` (0: no limit); then read the selection half for the margin and the
    report half once."""
    torch.manual_seed(seed)
    rng = np.random.default_rng(seed)
    began = time.perf_counter()
    tower = make_tower(frozen, arm).to(device)
    frozen.eval()
    width = 2 * rows.d_model + V.TAIL
    head = V.SearchHead(width, kind, hidden).to(device)
    passer = TowerPass(frozen, tower, board_chunk, thinking_steps, device)
    batches = Batches(rows, roots, passer, head, tau, device)
    tower_parameters = [p for p in tower.parameters() if p.requires_grad]
    optimizer = torch.optim.AdamW([dict(params=tower_parameters, lr=tower_lr, weight_decay=0.),
                                   dict(params=list(head.parameters()), lr=head_lr, weight_decay=weight_decay)])
    select, final = V.held_out_halves(rows)
    train = V.training_roots(rows, rng)
    labels = V.item_labels(rows, tau)[:2]
    check = reproduce(rows, batches, train[:batch], tower)
    log(f"[tower] arm={arm} trainable={sum(p.numel() for p in tower_parameters)} reproduce={json.dumps(check)}")

    def gaps_of(chosen):
        out = np.zeros(len(rows.cand), dtype=np.float64)
        tower.eval()
        head.eval()
        with torch.no_grad():
            for start in range(0, len(chosen), batch):
                part = chosen[start:start + batch]
                out[batches.items(part)] = batches.gaps(part, batches.scores(part)).double().cpu().numpy()
        return out

    best, best_state, history, waited, stopped = None, None, [], 0, "epochs"
    out_of_time = lambda: minutes > 0 and time.perf_counter() - began > minutes * 60
    selecting = np.flatnonzero(select)
    for epoch in range(1, epochs + 1):
        tower.train()
        head.train()
        order = rng.permutation(train)
        total, steps = 0., 0
        for start in range(0, len(order), batch):
            part = order[start:start + batch]
            loss = batches.loss(part, batches.gaps(part, batches.scores(part)))
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(tower_parameters + list(head.parameters()), 1.0)
            optimizer.step()
            total, steps = total + float(loss.detach()), steps + 1
            if out_of_time():
                break
        select_loss = pairwise_on(rows, gaps_of(selecting), labels, select)
        history.append(dict(epoch=epoch, train=total / max(steps, 1), select=select_loss, steps=steps,
                            minutes=(time.perf_counter() - began) / 60))
        log(f"[tower] arm={arm} epoch={epoch} train={total / max(steps, 1):.6f} select={select_loss:.6f} "
            f"minutes={(time.perf_counter() - began) / 60:.1f}")
        if best is None or select_loss < best:
            best, waited = select_loss, 0
            best_state = dict(tower=trainable_state(tower, arm),
                              head={k: v.detach().cpu().clone() for k, v in head.state_dict().items()})
        else:
            waited += 1
        if waited >= patience:
            stopped = "patience"
            break
        if out_of_time():
            stopped = "minutes"
            break
    load_trainable(tower, arm, best_state["tower"])
    head.load_state_dict(best_state["head"])
    zero = np.zeros(len(rows.cand))
    gaps = gaps_of(selecting)
    chooser = V.report(rows, gaps, labels[1], select)
    margin, _ = V.best_margin(chooser)
    # The report half, read once.
    gaps = gaps + gaps_of(np.flatnonzero(final))
    out = dict(arm=arm, modules=list(ARMS[arm]), trainable_parameters=sum(p.numel() for p in tower_parameters),
               reproduce=check, history=history, stopped=stopped, best_select_loss=best,
               zero_head_select_loss=pairwise_on(rows, zero, labels, select),
               minutes=(time.perf_counter() - began) / 60, train_roots=int(len(train)),
               margin=margin, margin_selection=chooser, held_out=V.report(rows, gaps, labels[1], final, margin))
    if len(np.unique(rows.source)) > 1:
        out["held_out_by_source"] = {str(int(s)): V.report(rows, gaps, labels[1], final & (rows.source == s), margin)
                                     for s in np.unique(rows.source)}
    return tower, head, best_state, out


@torch.no_grad()
def reproduce(rows, batches, chosen, tower):
    """The untrained tower must give the captured embeddings back (fp16 rows): the smallest cosine
    and the largest relative error of the state and candidate rows of `chosen` roots."""
    tower.eval()
    state, cands = batches.passer([batches.roots.frames[r] for r in chosen],
                                  batches.roots.hidden[torch.from_numpy(np.asarray(chosen, dtype=np.int64))])
    want_state = torch.from_numpy(rows.state[chosen].astype(np.float32)).to(state.device)
    got_cand = torch.cat(cands)
    want_cand = torch.from_numpy(rows.cand[batches.items(chosen)].astype(np.float32)).to(got_cand.device)
    cosine = lambda a, b: float(torch.nn.functional.cosine_similarity(a, b, dim=1).min())
    relative = lambda a, b: float(((a - b).norm(dim=1) / b.norm(dim=1).clamp(min=1e-6)).max())
    out = dict(roots=int(len(chosen)), state_cosine=cosine(state, want_state), cand_cosine=cosine(got_cand, want_cand),
               state_relative=relative(state, want_state), cand_relative=relative(got_cand, want_cand))
    if min(out["state_cosine"], out["cand_cosine"]) < 0.999 or max(out["state_relative"], out["cand_relative"]) > 0.02:
        raise ValueError(f"The untrained tower does not reproduce the captured embeddings: {out}")
    return out


def verdict(result, rule):
    """The pre-registered rule on one arm's report half: pooled Pearson, the gain per 100 roots at
    the selection half's margin, and z = gain / SE (SE by game; z = 0 without an SE)."""
    held = result["held_out"]
    cell = held["margins"][margin_key(result["margin"])]
    gain, se, pearson = cell["gain_per_100_roots"], cell["se"], held["pearson_pooled"]
    z = gain / se if se else 0.
    checks = dict(pearson=pearson is not None and pearson >= rule["pearson_pooled_min"],
                  gain=gain >= rule["gain_per_100_roots_min"], z=z >= rule["z_min"])
    return dict(pearson_pooled=pearson, gain_per_100_roots=gain, se=se, z=z, margin=result["margin"],
                overrides=cell["overrides"], checks=checks, passed=all(checks.values()))


def margin_key(margin):
    keys = {float(m): str(m) for m in V.MARGINS}
    return keys[float(margin)]


def read_preregistration(path):
    data = json.loads(Path(path).read_text())
    if data.get("contract") != PREREGISTRATION_CONTRACT:
        raise ValueError(f"{path} is not a {PREREGISTRATION_CONTRACT}")
    return data, hashlib.sha256(Path(path).read_bytes()).hexdigest()


def decide(prereg_path, arm_dirs, out_path):
    """Pick the arm on the selection half (its gain at its own margin; ties: the first listed),
    judge only that arm on the report half by the pre-registered rule."""
    prereg, digest = read_preregistration(prereg_path)
    results = {}
    for name, directory in arm_dirs:
        result = json.loads((Path(directory) / "advantage_tower.json").read_text())
        if result.get("preregistration_sha256") != digest:
            raise ValueError(f"{directory} was not fit under this pre-registration")
        results[name] = result
    gains = {name: V.best_margin(r["margin_selection"])[1] for name, r in results.items()}
    chosen = max(results, key=lambda name: (gains[name], -list(results).index(name)))
    out = dict(contract=DECISION_CONTRACT, preregistration=str(prereg_path), preregistration_sha256=digest,
               rule=prereg["rule"], selection_gain_per_100_roots=gains, chosen_arm=chosen,
               verdict=verdict(results[chosen], prereg["rule"]),
               by_source={name: {s: dict(pearson_pooled=c["pearson_pooled"],
                                         gain_per_100_roots=c["margins"][margin_key(r["margin"])]["gain_per_100_roots"],
                                         se=c["margins"][margin_key(r["margin"])]["se"],
                                         search_halves=c["search_halves"])
                                     for s, c in r.get("held_out_by_source", {}).items()}
                          for name, r in results.items()},
               informational={name: verdict(r, prereg["rule"]) for name, r in results.items()})
    Path(out_path).write_text(json.dumps(out, indent=1))
    print("[tower] decision " + json.dumps(dict(chosen_arm=chosen, verdict=out["verdict"])), flush=True)
    return out


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    fitting = commands.add_parser("fit")
    fitting.add_argument("--rows", action="append", required=True, help=f"{V.ROWS_CONTRACT} files (sources, in order)")
    fitting.add_argument("--state-dir", action="append", required=True,
                         help="Each rows file's capture state cache directory (read only), in the same order")
    fitting.add_argument("--model", required=True, help="The frozen net the rows were captured from (r1)")
    fitting.add_argument("--arm", choices=sorted(ARMS), required=True)
    fitting.add_argument("--out", required=True, help="A new directory")
    fitting.add_argument("--preregistration", default="", help="The pre-registration file the fit runs under")
    fitting.add_argument("--device", default="cpu")
    fitting.add_argument("--head", choices=("linear", "mlp"), default="mlp")
    fitting.add_argument("--hidden", type=int, default=0)
    fitting.add_argument("--tau", type=float, default=0.02)
    fitting.add_argument("--head-lr", type=float, default=1e-3)
    fitting.add_argument("--tower-lr", type=float, default=3e-5)
    fitting.add_argument("--weight-decay", type=float, default=1e-2)
    fitting.add_argument("--epochs", type=int, default=25)
    fitting.add_argument("--batch", type=int, default=32, help="Roots per step")
    fitting.add_argument("--patience", type=int, default=4)
    fitting.add_argument("--minutes", type=float, default=0., help="Stop training after this long (0: no limit)")
    fitting.add_argument("--seed", type=int, required=True)
    fitting.add_argument("--load-only", action="store_true",
                         help="Load every root and check the untrained tower on training roots, then stop (sizing)")
    fitting.add_argument("--smoke-games", type=int, default=0,
                         help="Smoke tests only: the first N training games of each rows file (never a held-out one)")
    fitting.add_argument("--hold-out-every", type=int, default=0,
                         help="Smoke tests only: hold out games whose id is a multiple of K instead of the rows' own")
    fitting.add_argument("--threads", type=int, default=8)
    fitting.add_argument("--runtime", default="", help="A sealed runtime directory put first on PATH (trajectory_cache4)")
    fitting.add_argument("--no-halves-judged", dest="halves_judged", action="store_false")
    deciding = commands.add_parser("decide")
    deciding.add_argument("--preregistration", required=True)
    deciding.add_argument("--arm", action="append", required=True, help="NAME=DIR, in tie-break order")
    deciding.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    if args.command == "decide":
        return decide(args.preregistration, [tuple(a.split("=", 1)) for a in args.arm], args.out)
    if args.runtime:
        import os
        runtime = Path(args.runtime).expanduser()
        if not (runtime / "trajectory_cache4").is_file():
            raise FileNotFoundError(f"{runtime} lacks trajectory_cache4")
        os.environ["PATH"] = f"{runtime}{os.pathsep}{os.environ.get('PATH', '')}"
    from torch_io4 import file_digest
    from value_calibration4.checkpoint import load_source
    torch.set_num_threads(args.threads)
    out = Path(args.out)
    if out.exists():
        raise FileExistsError(f"{out} exists; a tower gets its own directory")
    digest = read_preregistration(args.preregistration)[1] if args.preregistration else None
    if digest and (args.smoke_games or args.hold_out_every):
        raise ValueError("A fit under a pre-registration uses the rows' own held-out games, never a smoke subset")
    rows, parts, sources, judged_from = V.load_rows(args.rows, args.halves_judged)
    keeps = None
    if args.smoke_games:
        # Smoke tests: the first training games of each file only, never a held-out one, with a
        # held-out split of their own.
        if not args.hold_out_every:
            raise ValueError("--smoke-games needs --hold-out-every for its own held-out split")
        keeps = [np.isin(p.roots["game"], np.unique(p.roots["game"][~p.held_out])[:args.smoke_games]) for p in parts]
        parts = [p.subset(k) for p, k in zip(parts, keeps)]
        rows = V.AdvantageRows.concat(parts)
    if args.hold_out_every:
        rows.held_out = rows.roots["game"] % args.hold_out_every == 0
    device = torch.device(args.device)
    path, _, net = load_source(args.model)
    source = file_digest(path)
    if sources != {source}:
        raise ValueError("The rows were captured from another net than --model")
    net.decision_chunk, net.board_token_budget = 4, 32768
    net.prune_unused_policy_heads, net.gpu_prefetch = False, 0
    full = [V.AdvantageRows.load(p) for p in args.rows] if keeps is not None else parts
    roots = Roots.load(full, args.state_dir, net, source, keeps=keeps)
    net = net.to(device)
    if args.load_only:
        # Sizing only: every root loaded, the untrained tower checked on training roots; no step,
        # no held-out root read, nothing written.
        tower = make_tower(net, args.arm).to(device)
        head = V.SearchHead(2 * rows.d_model + V.TAIL, args.head, args.hidden).to(device)
        batches = Batches(rows, roots, TowerPass(net, tower, 32, 2, device), head, args.tau, device)
        check = reproduce(rows, batches, np.flatnonzero(~rows.held_out)[:args.batch], tower)
        size = sum(np.asarray(v).nbytes for frame in roots.frames for v in frame.values()) / 2**20
        out_line = dict(roots=len(roots.frames), frames_mib=round(size), hidden_mib=round(roots.hidden.numel() * 4 / 2**20),
                        reproduce=check)
        print("[tower] load-only " + json.dumps(out_line), flush=True)
        return out_line
    tower, head, state, result = fit(rows, roots, net, args.arm, device, args.head, args.hidden, args.tau, args.head_lr,
                                     args.tower_lr, args.weight_decay, args.epochs, args.batch, args.patience,
                                     args.seed, args.minutes)
    out.mkdir(parents=True)
    width = 2 * rows.d_model + V.TAIL
    torch.save(dict(contract=CONTRACT, arm=args.arm, modules=list(ARMS[args.arm]), tower=state["tower"],
                    head=state["head"], width=width, kind=args.head, hidden=args.hidden, margin=result["margin"],
                    source_checkpoint=str(path), source_checkpoint_sha256=source), out / "tower.pt")
    meta = dict(contract=CONTRACT, d_model=rows.d_model, row_tail=V.TAIL, width=width,
                tail=["class:" + name for name in V.CLASSES] + ["payable_added", "own"],
                source_checkpoint=str(path), source_checkpoint_sha256=source,
                preregistration=args.preregistration or None, preregistration_sha256=digest,
                head=args.head, hidden=args.hidden, tau=args.tau, head_lr=args.head_lr, tower_lr=args.tower_lr,
                seed=args.seed, batch=args.batch, epochs=args.epochs, patience=args.patience, minutes_cap=args.minutes,
                device=str(device), halves_judged=judged_from, runtime=args.runtime or None, smoke_games=args.smoke_games,
                hold_out_every=args.hold_out_every or None, trainer_source=str(Path(__file__).resolve().parents[3]),
                rows=[dict(source=i, path=str(p), sha256=hashlib.sha256(Path(p).read_bytes()).hexdigest())
                      for i, p in enumerate(args.rows)],
                state_dirs=list(args.state_dir),
                tower_sha256=hashlib.sha256((out / "tower.pt").read_bytes()).hexdigest(), **result)
    (out / "advantage_tower.json").write_text(json.dumps(meta, indent=1))
    print("[tower] " + json.dumps(dict(arm=args.arm, margin=result["margin"], stopped=result["stopped"],
                                       held_out=result["held_out"])), flush=True)
    return meta


if __name__ == "__main__":
    main()
