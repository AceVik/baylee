"""Command line: `extract` a dataset's positions, `fit` a head, `log` it for `phase_a4 values`."""
import argparse
import json
from pathlib import Path

import numpy as np
import orjson
import torch

import model4 as M
from value_calibration4.checkpoint import file_digest, load_source, merged_checkpoint, publish
from .extract import Positions, extract, open_frames
from .fit import Samples, fit, predict, side_logit

FROZEN = "Every tensor except seat_value_head.* bit-identical; trunk, memory and policy untouched"


def parse_sets(values):
    """`DIR[=WEIGHT[:HOLD[:DOMAIN]]]`: Positions, their weight, every HOLD-th game kept out, their domain."""
    sets = []
    for value in values:
        path, _, spec = value.partition("=")
        weight, hold, domain = (spec.split(":") + ["", ""])[:3]
        sets.append((Positions(path), float(weight or 1), int(hold or 0), int(domain or 0)))
    return sets


def held_out(rows, hold):
    """Games a set keeps out of every fit, for an evaluation of its own."""
    return rows["game"] % hold == 0 if hold else np.zeros(len(rows), dtype=bool)


def check_duel_readout(net, positions, device):
    """`side_logit` must be the deployed value, or the head is fit on another function."""
    rows = min(256, len(positions.rows))
    tokens = torch.from_numpy(np.array(positions.features[:rows])).to(device).float()
    seats = torch.zeros(rows, M.MAX_SEATS, M.SEAT_FEAT_WIDTH, device=device)
    seats[:, 0, [M.SEAT_FEAT_PRESENT, M.SEAT_FEAT_SELF]] = 1.0
    seats[:, 1, [M.SEAT_FEAT_PRESENT, M.SEAT_FEAT_OPPONENT]] = 1.0
    spare = torch.zeros(rows, M.MAX_SEATS - 2, tokens.shape[-1], device=device)
    deployed, _ = net.seat_values(torch.cat([tokens[:, 1:3], spare], dim=1), tokens[:, 0], seats)
    torch.testing.assert_close(torch.sigmoid(side_logit(net.seat_value_head, tokens)), deployed, rtol=1e-4, atol=1e-5)


def run_extract(args):
    torch.set_float32_matmul_precision("high" if args.tf32 else "highest")
    device = torch.device(args.device)
    _, saved, net = load_source(args.model)
    net.board_token_budget = args.token_budget
    net.to(device)
    frames = open_frames(args.data, args.split, saved["encoder_version"], args.seats)
    if frames.structure.digest != net.rules.digest:
        raise ValueError("Dataset and model use different rule catalogs")
    print(json.dumps(extract(net, frames, args.out, args.lanes, args.board_chunk, args.workers, args.max_games)), flush=True)


def run_fit(args):
    device = torch.device(args.device)
    source, saved, net = load_source(args.model)
    source_hash = file_digest(source)
    sets = parse_sets(args.train)
    net.to(device)
    check_duel_readout(net, sets[0][0], device)
    validating = lambda rows: rows["game"] % args.val_every == 0
    train = Samples(sets, args.per_turn, args.seed, device, lambda rows, hold: ~held_out(rows, hold) & ~validating(rows))
    validation = Samples(sets, args.per_turn, args.seed + 1, device, lambda rows, hold: ~held_out(rows, hold) & validating(rows))
    head, report = fit(net.seat_value_head, train, validation, args.seed, args.epochs, args.lr,
                       args.weight_decay, args.batch, args.fresh, args.step_weight)
    provenance = dict(kind="value_head_v6", source=str(source.resolve()), source_sha256=source_hash, seed=args.seed,
                      promoted=False, frozen=FROZEN, train=[str(entry[0].path) for entry in sets],
                      weights=[entry[1] for entry in sets], held_out_every=[entry[2] for entry in sets],
                      domains=[entry[3] for entry in sets],
                      settings={k: v for k, v in vars(args).items() if k not in ("run", "handler")})
    report["provenance"] = provenance
    publish(Path(args.out), merged_checkpoint(saved, head.cpu(), provenance), report)
    print(json.dumps({k: report[k] for k in ("selected_epoch", "train_rows", "validation_rows")}), flush=True)


def run_log(args):
    """Writes sets as `--value-log` lines and a summary `phase_a4 values` reads.

    A game's two seats are one table, so its bootstrap keeps them together.
    """
    device = torch.device(args.device)
    _, _, net = load_source(args.model)
    head = net.seat_value_head.float().to(device)
    outcomes, written = {}, 0
    with Path(args.log).open("wb") as log:
        for number, path in enumerate(args.positions):
            positions = Positions(path)
            rows = positions.rows
            values = predict(head, positions, device)
            if args.held_every:
                kept = held_out(rows, args.held_every) ^ args.complement
                rows, values = rows[kept], values[kept]
            for row, value in zip(rows, values.tolist()):
                deal = ((number << 32) + int(row["game"])) * 2 + int(row["seat"])
                outcomes[deal] = float(row["outcome"])
                line = dict(d=deal, focal=int(row["seat"]), turn=int(row["turn"]), active=bool(row["active"]),
                            settled=bool(row["settled"]), kind=int(row["pending"]), src="Net", fb=False,
                            v=value, ov=value, life=int(row["life"]), opp_life=int(row["opp_life"]),
                            hand=int(row["hand"]), opp_hand=int(row["opp_hand"]),
                            board=int(row["board"]), opp_board=int(row["opp_board"]))
                log.write(orjson.dumps(line) + b"\n")
            written += len(rows)
    details = [dict(deal=deal, net_utility=outcome, net_capped=False) for deal, outcome in sorted(outcomes.items())]
    Path(args.summary).write_text(json.dumps(dict(deal_details=details)))
    print(json.dumps(dict(positions=written, trajectories=len(details))), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="run", required=True)
    one = commands.add_parser("extract")
    one.add_argument("--model", required=True)
    one.add_argument("--data", required=True)
    one.add_argument("--split", default="train")
    one.add_argument("--seats", choices=("net", "all"), default="net")
    one.add_argument("--out", required=True)
    one.add_argument("--lanes", type=int, default=32)
    one.add_argument("--board-chunk", type=int, default=16)
    one.add_argument("--workers", type=int, default=6)
    one.add_argument("--token-budget", type=int, default=131072)
    one.add_argument("--max-games", type=int, default=0)
    one.add_argument("--tf32", action="store_true")
    one.add_argument("--device", default="cuda")
    one.set_defaults(handler=run_extract)
    two = commands.add_parser("fit")
    two.add_argument("--model", required=True)
    two.add_argument("--train", nargs="+", required=True, help="DIR or DIR=WEIGHT, from extract")
    two.add_argument("--out", required=True)
    two.add_argument("--seed", type=int, default=31431)
    two.add_argument("--epochs", type=int, default=12)
    two.add_argument("--lr", type=float, default=3e-4)
    two.add_argument("--weight-decay", type=float, default=1e-2)
    two.add_argument("--batch", type=int, default=4096)
    two.add_argument("--per-turn", type=int, default=3)
    two.add_argument("--val-every", type=int, default=20, help="every Nth game validates")
    two.add_argument("--fresh", action="store_true", help="start from random weights, not the source head")
    two.add_argument("--step-weight", type=float, default=0.0, help="weight of the change between turns as a reading of who wins")
    two.add_argument("--device", default="cuda")
    two.set_defaults(handler=run_fit)
    three = commands.add_parser("log")
    three.add_argument("--model", required=True)
    three.add_argument("--positions", nargs="+", required=True)
    three.add_argument("--log", required=True)
    three.add_argument("--summary", required=True)
    three.add_argument("--held-every", type=int, default=0, help="only every Nth game, as a fit's HOLD left it out")
    three.add_argument("--complement", action="store_true", help="with --held-every: the other games")
    three.add_argument("--device", default="cuda")
    three.set_defaults(handler=run_log)
    args = parser.parse_args()
    args.handler(args)
