"""What a window run's cross-entropy saw: its targets' classes by subtype, from its sampler order.

    CUDA_VISIBLE_DEVICES= python -m training4.trained_balance --data DATASET \
        --entities /mnt/xdata/.../distill-round0-v3-r1-4eb-b0-s2.json.entities.npy \
        --batches 7468 --mode act --out balance.json

Rebuilds the decision-window plan and the sampler's batches of epoch 1 exactly
as the trainer drew them (seed, lanes, lane length, entity buckets; the
per-piece entity counts come from the run's window-state cache), takes the
first `--batches` (a run's `consumed_batches`), and joins every labelled
priority frame in them with the dataset's coarse-class sidecar by (game, seat,
cursor), which also gives R's and N's candidates (`N != R`). Each frame is routed
as the loss under `--mode` routes it:
- `kl` (arm A): in-scope `N != R` get CE to R; the rest the KL;
- `ce-n` (A'): also every out-of-scope frame, CE to N;
- `act`, `class` (A''): `N != R` frames by group (act/pass or class): groups
  differ, CE to R's group; one group, a cast keeps CE to R, others CE to N.
It counts the CE frames per subtype by their target's class (and act/pass) and
the frames each term saw. No frame is decoded and no network runs.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np

import dataset4 as D4
from .coarse import CLASSES, CoarseClasses, key
from .windows import WindowTraining

PASS = CLASSES.index("pass")


def routes(mode, in_scope, differs, r_class, n_class, cast):
    """Per frame: (term, target class), term '' where only the KL applies."""
    group = (lambda c: c) if mode == "class" else (lambda c: (c != PASS).astype(np.int64))
    term = np.full(len(r_class), "", dtype=object)
    target = np.full(len(r_class), -1, dtype=np.int64)
    if mode in ("kl", "ce-n"):
        ce_r = in_scope & differs
        term[ce_r], target[ce_r] = "ce_r", r_class[ce_r]
        if mode == "ce-n":
            ce_n = ~in_scope
            term[ce_n], target[ce_n] = "ce_n_out_of_scope", n_class[ce_n]
        return term, target
    split = differs & (group(r_class) != group(n_class))
    kept = differs & ~split & cast
    fine = differs & ~split & ~cast
    term[split], target[split] = "ce_coarse_r", r_class[split]
    term[kept], target[kept] = "ce_r", r_class[kept]
    term[fine], target[fine] = "ce_fine_n", n_class[fine]
    return term, target


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--data", required=True)
    parser.add_argument("--entities", required=True, help="The run's window-state cache .entities.npy")
    parser.add_argument("--batches", type=int, default=0, help="Batches the run consumed (0 = the whole epoch)")
    parser.add_argument("--mode", default="kl", help="kl, ce-n, act or class; several comma-separated")
    parser.add_argument("--subtypes", default="cast,pass,instant_their_turn,spend,target")
    parser.add_argument("--seed", type=int, default=73501)
    parser.add_argument("--batch-size", type=int, default=8)
    parser.add_argument("--unroll", type=int, default=32)
    parser.add_argument("--window-length", type=int, default=32)
    parser.add_argument("--window-burn-in", type=int, default=0)
    parser.add_argument("--window-entity-buckets", type=int, default=16)
    parser.add_argument("--encoder-version", type=int, default=10)
    parser.add_argument("--out", default="")
    args = parser.parse_args(argv)
    if not set(args.mode.split(",")) <= {"kl", "ce-n", "act", "class"}:
        parser.error("--mode takes kl, ce-n, act and class")
    data = Path(args.data).expanduser()
    dataset = D4.TrajectoryDataset4(data, split="train", unroll_len=args.unroll, encoder_version=args.encoder_version)
    names = list(json.loads((data / "meta.json").read_text())["bundle_distillation"]["subtypes"])
    sidecar = CoarseClasses.load(data / "bundle-distill")
    plan = WindowTraining(dataset, args.window_burn_in, args.window_length, args.batch_size, args.seed,
                          args.window_entity_buckets)
    plan.entities = np.load(Path(args.entities).expanduser())
    if len(plan.entities) != len(plan.pieces):
        raise ValueError("The entity counts belong to another window plan")
    batches = plan.sampler._batches(0)
    used = batches[:args.batches] if args.batches else batches
    pids = np.fromiter((pid for batch in used for lane in batch for pid in lane), dtype=np.int64)
    rows = np.concatenate([plan.rows[t][s:s + n] for t, s, n in plan.pieces[pids].tolist()])
    index = dataset.store.index
    at = np.searchsorted(sidecar.keys, key(index["game"][rows], index["seat"][rows], index["cursor"][rows]))
    at = np.minimum(at, len(sidecar.keys) - 1)
    found = sidecar.keys[at] == key(index["game"][rows], index["seat"][rows], index["cursor"][rows])
    side = sidecar.index[at[found]]
    genuine = side["n_cand"] > 1
    side, frame_rows = side[genuine], rows[found][genuine]
    subtype = np.where(side["subtype"] == 255, -1, side["subtype"].astype(np.int64))
    scope = {names.index(name) for name in args.subtypes.split(",") if name in names}
    in_scope = np.isin(subtype, list(scope))
    differs = side["r_idx"] != side["n_idx"]
    r_class, n_class = side["r_class"].astype(np.int64), side["n_class"].astype(np.int64)
    cast = subtype == (names.index("cast") if "cast" in names else -2)
    count = lambda chosen: dict(frames=int(chosen.sum()), n_ne_r=int((chosen & differs).sum()))
    report = dict(batches=len(used), of_batches=len(batches), frames_seen=int(len(rows)),
                  labelled_priority_frames=int(len(frame_rows)),
                  subtypes={(names[code] if code >= 0 else "none"): count(subtype == code)
                            for code in sorted(set(subtype.tolist()))}, modes={})
    for mode in args.mode.split(","):
        term, target = routes(mode, in_scope, differs, r_class, n_class, cast)

        def tally(chosen):
            out = {}
            for name_term in sorted(set(term[chosen].tolist()) - {""}):
                rows_term = chosen & (term == name_term)
                classes = np.bincount(target[rows_term], minlength=len(CLASSES))
                out[name_term] = dict(frames=int(rows_term.sum()), act=int(classes.sum() - classes[PASS]),
                                      passes=int(classes[PASS]), by_class=dict(zip(CLASSES, classes.tolist())))
            return out

        report["modes"][mode] = dict(
            terms=tally(np.ones(len(term), dtype=bool)),
            by_subtype={(names[code] if code >= 0 else "none"): tally(subtype == code)
                        for code in sorted(set(subtype.tolist()))})
    text = json.dumps(report, indent=1)
    if args.out:
        Path(args.out).write_text(text + "\n")
    print(text, flush=True)
    return report


if __name__ == "__main__":
    main()
