"""Offline probe of priority decisions: can a head fit the layer's answer R
where the net's own answer N differs, and from which inputs?

Reads the priority records of an imitating collection (`eval4
--tactician-windows priority --tactician-imitate 0`, `tactician_imitation_v1`)
once, then fits the stage-S1 set transformer (`tactician_v3`, T = 1) on each
feature set, the rows' other parts read as zero:

    logits_facts   the net's logits over the candidates and N (window code)
                   plus the trivial facts (whose turn, stack, untapped mana,
                   hands, step, paying, turn)
    embeddings     + the thought-through state and each candidate's embedding
    look           + the look: the state reads (castable instants under the
                   train-deck prior, mana, lives) and the priority look per
                   candidate (instant on their turn, castable now, the stack)
    look_no_prior  the same without the castable instants under the prior
                   (look columns 0-24), an ablation

Whole tables are held out by seed; one half selects the checkpoint on exact
top-1 agreement with R where N != R, the other is reported: per feature set,
per decision kind (pass, cast, spend, instant on their turn, pay, ...) and per
rule group that changed R there. Writes OUT/<set>/report.json and
OUT/summary.json. The gate the campaign reads is about 80 % at N != R.
"""
from __future__ import annotations

import argparse
import json
from argparse import Namespace
from pathlib import Path

import torch

FEATURE_SETS = {
    "logits_facts": "state,unit,look",
    "embeddings": "look",
    "look": "",
    "look_no_prior": "prior",
}


def summary_of(report: dict) -> dict:
    """What the probe reports of one fit: the reported half, final pass."""
    held = report["held_out"]
    reading = held["passes"][-1]
    window = "priority" if "priority" in reading["by_window_differ"] else "all"
    return dict(
        decisions=held["decisions"], differ=held["differ"],
        net_agreement=held["net_answer"].get(window, {}).get("exact"),
        all_decisions=reading["by_window"].get(window),
        differ_exact=reading["by_window_differ"].get(window),
        by_kind_differ=reading.get("by_kind_differ"),
        by_group_differ=reading.get("by_group_differ"),
        epochs=len(report["history"]),
        select_differ_exact_by_epoch=[row["select_differ_exact"] for row in report["history"]],
    )


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--records", nargs="+", required=True,
                    help="tactician_imitation_v1 directories with priority records")
    ap.add_argument("--out", required=True)
    ap.add_argument("--sets", default=",".join(FEATURE_SETS),
                    help=f"feature sets to fit, of {', '.join(FEATURE_SETS)}")
    ap.add_argument("--seed", type=int, default=76101)
    ap.add_argument("--width", type=int, default=256)
    ap.add_argument("--layers", type=int, default=4)
    ap.add_argument("--heads", type=int, default=8)
    ap.add_argument("--epochs", type=int, default=30)
    ap.add_argument("--patience", type=int, default=5)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--warmup", type=int, default=300)
    ap.add_argument("--held-out-mod", type=int, default=10)
    ap.add_argument("--device", default="cuda" if torch.cuda.is_available() else "cpu")
    ap.add_argument("--threads", type=int, default=0)
    ap.add_argument("--bf16", action="store_true")
    ap.add_argument("--data-on-device", action="store_true")
    ap.add_argument("--store-fp16", action="store_true")
    args = ap.parse_args(argv)
    from tactician_v3.records import read_records
    from tactician_v3.train import train

    sets = [s for s in args.sets.split(",") if s]
    unknown = set(sets) - set(FEATURE_SETS)
    if unknown:
        raise ValueError(f"no feature set {sorted(unknown)}")
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=False)
    store = read_records(args.records)
    summary = dict(records=[str(Path(p).resolve()) for p in args.records],
                   gate="exact top-1 agreement with R where N != R, about 0.80", sets={})
    for name in sets:
        fit = Namespace(
            records=args.records, out=str(out / name), seed=args.seed, width=args.width,
            layers=args.layers, heads=args.heads, passes=1, ffn_mult=4, dropout=0.0,
            through_passes=False, lr=args.lr, weight_decay=0.01, batch=args.batch,
            eval_batch=512, epochs=args.epochs, patience=args.patience, max_steps=0,
            warmup=args.warmup, clip=1.0, confidence_weight=0.5, group_softmax="block",
            held_out_mod=args.held_out_mod, train_report=20000, parity_rows=0,
            export_passes=[1], device=args.device, bf16=args.bf16,
            data_on_device=args.data_on_device, store_fp16=args.store_fp16,
            threads=args.threads, stage1_probe=False, zero_parts=FEATURE_SETS[name],
            windows="priority", no_export=True)
        _, report = train(fit, store=store)
        summary["sets"][name] = dict(zero_parts=FEATURE_SETS[name], **summary_of(report))
        (out / "summary.json").write_text(json.dumps(summary, indent=2))
        line = summary["sets"][name]["differ_exact"]
        print(json.dumps(dict(set=name, differ_exact=line and line["exact"])), flush=True)
    return summary


if __name__ == "__main__":
    main()
