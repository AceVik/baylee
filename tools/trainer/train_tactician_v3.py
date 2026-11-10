"""Train, export or re-export the stage-S1 tactician; see tactician_v3.

    train   records of `eval4 --tactician-imitate` -> OUT/{net.pt, report.json,
            tactician.onnx, tactician.json, passes/T<k>/...}
    export  a trained OUT/net.pt at another number of passes
    fixture the tiny untrained net the Rust tests load (d = 4)
"""
from __future__ import annotations

import argparse
from pathlib import Path

import torch


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="command", required=True)

    tr = sub.add_parser("train", help="fit on imitation records and export")
    tr.add_argument("--records", nargs="+", required=True,
                    help="tactician_imitation_v1 directories")
    tr.add_argument("--out", required=True)
    tr.add_argument("--seed", type=int, required=True)
    tr.add_argument("--width", type=int, default=512)
    tr.add_argument("--layers", type=int, default=4)
    tr.add_argument("--heads", type=int, default=8)
    tr.add_argument("--passes", type=int, default=1,
                    help="refinement passes T; 1 unless a measurement shows more help")
    tr.add_argument("--ffn-mult", type=int, default=4)
    tr.add_argument("--dropout", type=float, default=0.0)
    tr.add_argument("--independent", action="store_true",
                    help="score every unit from its own row alone: no attention across units")
    tr.add_argument("--through-passes", action="store_true",
                    help="let gradients run through the previous pass's proposal")
    tr.add_argument("--lr", type=float, default=3e-4)
    tr.add_argument("--weight-decay", type=float, default=0.01)
    tr.add_argument("--batch", type=int, default=256)
    tr.add_argument("--eval-batch", type=int, default=512)
    tr.add_argument("--epochs", type=int, default=30)
    tr.add_argument("--patience", type=int, default=4)
    tr.add_argument("--max-steps", type=int, default=0)
    tr.add_argument("--warmup", type=int, default=500)
    tr.add_argument("--clip", type=float, default=1.0)
    tr.add_argument("--confidence-weight", type=float, default=0.5)
    tr.add_argument("--group-softmax", default="block",
                    help="windows whose groups also get a softmax with none: block, attack")
    tr.add_argument("--held-out-mod", type=int, default=10)
    tr.add_argument("--train-report", type=int, default=20000,
                    help="training decisions read back for the report")
    tr.add_argument("--parity-rows", type=int, default=64,
                    help="held-out decisions every export is checked on")
    tr.add_argument("--export-passes", type=lambda s: [int(x) for x in s.split(",") if x],
                    default=[1], help="also export at these T (passes/T<k>)")
    tr.add_argument("--device", default="cuda" if torch.cuda.is_available() else "cpu")
    tr.add_argument("--bf16", action="store_true", help="autocast the forward to bfloat16 on CUDA")
    tr.add_argument("--data-on-device", action="store_true",
                    help="keep the records on the training device")
    tr.add_argument("--store-fp16", action="store_true", help="keep the records in float16")
    tr.add_argument("--threads", type=int, default=0, help="CPU threads for torch (0: its own)")
    tr.add_argument("--windows", default="",
                    help="fit and report only these windows (comma list; default every one)")
    tr.add_argument("--no-export", action="store_true", help="report only; export nothing")
    tr.add_argument("--zero-parts", default="",
                    help="ablation: row parts read as zero, a comma list of state, unit, look "
                         "(look columns 0-99), prior (look 0-24), facts (look 100-111)")
    tr.add_argument("--stage1-probe", action="store_true",
                    help="the offline probe: fit stage-1 outcome records (tactician_records_v1) "
                         "to their rules answer, its columns zeroed; reports only, no export")

    ex = sub.add_parser("export", help="export a trained net.pt at T passes")
    ex.add_argument("--checkpoint", required=True)
    ex.add_argument("--passes", type=int, required=True)
    ex.add_argument("--out", required=True)
    ex.add_argument("--threshold", type=float, default=None,
                    help="the confidence threshold to write (default: the trained one at T)")

    fx = sub.add_parser("fixture", help="the tiny untrained net for the Rust tests")
    fx.add_argument("--out", required=True)

    args = ap.parse_args(argv)
    if args.command == "train":
        from tactician_v3.train import train
        return train(args)
    if args.command == "export":
        from tactician_v3.export import export_dir, load_checkpoint
        net, meta = load_checkpoint(Path(args.checkpoint))
        if args.threshold is not None:
            meta = dict(meta, thresholds_by_pass=[args.threshold] * args.passes)
        return export_dir(net, meta, args.passes, Path(args.out))
    from tactician_v3.export import export_dir
    from tactician_v3.model import Config, TacticianV3
    from tactician_v3.records import ROW_TAIL
    from tactician_v3.train import CONTRACT
    torch.manual_seed(5)
    cfg = Config(d_model=4, width=16, layers=1, heads=2, passes=2, ffn_mult=2)
    net = TacticianV3(cfg)
    meta = dict(contract=CONTRACT, d_model=4, row_tail=ROW_TAIL, rows_read_rules=False,
                config=cfg.to_dict(), thresholds_by_pass=[0.5, 0.5],
                label="untrained: a test fixture for netplay4::tactician::net")
    return export_dir(net, meta, cfg.passes, Path(args.out))


if __name__ == "__main__":
    main()
