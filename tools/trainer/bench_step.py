"""A stand-in for the play net, timed: training steps and single-decision inference.

Written for the WSL2-against-native-Windows comparison (2026-09-29), and kept
because the same questions come back whenever the model's size or the machine
changes: how many samples a second can the GPU train on, and how long does one
decision take when the net is asked for it.

The model is shaped like the one the brief fixes, not trained: a card embedding
sized for every ledger row, a per-entity feature vector, an entity transformer,
a policy head scoring option entities and a value head over the pooled set.
Shapes are fixed (padded entity sets), as an NPU runtime wants them.

    uv run python bench_step.py            # eager
    uv run python bench_step.py --compile  # torch.compile as well (needs Triton)
"""

from __future__ import annotations

import argparse
import json
import platform
import statistics
import time

import torch
from torch import nn

# Rows in the append-only CardIndex ledger (`baylee_cards_index::ROWS`), plus
# row 0 for padding.
CARDS = 33_694
ENTITIES = 160  # board, hands, stack and graveyard objects, padded
OPTIONS = 32  # the last OPTIONS entities are the options `Pending` enumerated
FEATURES = 64  # per-entity numbers: zone, owner, tapped, counters, P/T, ...


class StandIn(nn.Module):
    def __init__(self, d: int = 384, layers: int = 6, heads: int = 6, ff: int = 1536, card_dim: int = 128):
        super().__init__()
        self.card = nn.Embedding(CARDS + 1, card_dim, padding_idx=0)
        self.card_proj = nn.Linear(card_dim, d)
        self.feat = nn.Linear(FEATURES, d)
        layer = nn.TransformerEncoderLayer(d, heads, ff, dropout=0.0, batch_first=True, norm_first=True)
        self.trunk = nn.TransformerEncoder(layer, layers, enable_nested_tensor=False)
        self.policy = nn.Linear(d, 1)
        self.value = nn.Sequential(nn.Linear(d, d), nn.GELU(), nn.Linear(d, 1))

    def forward(self, cards: torch.Tensor, feats: torch.Tensor, pad: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        x = self.card_proj(self.card(cards)) + self.feat(feats)
        x = self.trunk(x, src_key_padding_mask=pad)
        logits = self.policy(x[:, -OPTIONS:]).squeeze(-1)
        logits = logits.masked_fill(pad[:, -OPTIONS:], -1e4)
        live = (~pad).unsqueeze(-1).to(x.dtype)
        pooled = (x * live).sum(1) / live.sum(1).clamp(min=1.0)
        return logits, self.value(pooled).squeeze(-1)


def batch(n: int, device: str | torch.device, pin: bool = False) -> tuple[torch.Tensor, ...]:
    """A random batch shaped like a real one: about 60 % of the slots filled."""
    g = torch.Generator().manual_seed(n)
    cards = torch.randint(1, CARDS + 1, (n, ENTITIES), generator=g)
    feats = torch.randn(n, ENTITIES, FEATURES, generator=g)
    pad = torch.rand(n, ENTITIES, generator=g) > 0.6
    pad[:, 0] = False
    pad[:, -OPTIONS] = False  # at least one option: passing is always legal
    target = torch.zeros(n, dtype=torch.long)
    result = torch.randint(0, 2, (n,), generator=g).float()
    out = (cards, feats, pad, target, result)
    if pin:
        return tuple(t.pin_memory() for t in out)
    return tuple(t.to(device) for t in out)


def sync() -> None:
    torch.cuda.synchronize()


def train_throughput(model: nn.Module, n: int, steps: int) -> dict:
    opt = torch.optim.AdamW(model.parameters(), lr=3e-4, fused=True)
    cards, feats, pad, target, result = batch(n, "cuda")
    ce = nn.CrossEntropyLoss()
    bce = nn.BCEWithLogitsLoss()

    def step() -> None:
        with torch.autocast("cuda", dtype=torch.bfloat16):
            logits, value = model(cards, feats, pad)
            loss = ce(logits.float(), target) + bce(value.float(), result)
        opt.zero_grad(set_to_none=True)
        loss.backward()
        opt.step()

    torch.cuda.reset_peak_memory_stats()
    for _ in range(10):
        step()
    sync()
    start = time.perf_counter()
    for _ in range(steps):
        step()
    sync()
    elapsed = time.perf_counter() - start
    return {
        "batch": n,
        "steps_per_s": round(steps / elapsed, 2),
        "samples_per_s": round(steps * n / elapsed),
        # Past the card's own memory the driver spills into shared system
        # memory and a step slows by an order of magnitude; this says whether
        # a number was measured on the GPU's memory or on the spill.
        "peak_gb": round(torch.cuda.max_memory_allocated() / 2**30, 2),
    }


def inference_latency(model: nn.Module, n: int, calls: int, graph: bool) -> dict:
    """One decision as self-play would ask it: inputs from host memory to the
    GPU, the forward pass, and the scores back on the host."""
    cards, feats, pad = batch(n, "cpu")[:3]
    host = [t.pin_memory() for t in (cards, feats.half(), pad)]
    dev = [t.to("cuda") for t in host]
    out_host = torch.empty((n, OPTIONS), dtype=torch.float16).pin_memory()
    value_host = torch.empty((n,), dtype=torch.float16).pin_memory()

    def forward() -> tuple[torch.Tensor, torch.Tensor]:
        return model(*dev)

    with torch.inference_mode():
        if graph:
            stream = torch.cuda.Stream()
            stream.wait_stream(torch.cuda.current_stream())
            with torch.cuda.stream(stream):
                for _ in range(3):
                    forward()
            torch.cuda.current_stream().wait_stream(stream)
            cuda_graph = torch.cuda.CUDAGraph()
            with torch.cuda.graph(cuda_graph):
                static_logits, static_value = forward()

            def call() -> None:
                for d, h in zip(dev, host):
                    d.copy_(h, non_blocking=True)
                cuda_graph.replay()
                out_host.copy_(static_logits, non_blocking=True)
                value_host.copy_(static_value, non_blocking=True)
                sync()
        else:

            def call() -> None:
                for d, h in zip(dev, host):
                    d.copy_(h, non_blocking=True)
                logits, value = forward()
                out_host.copy_(logits, non_blocking=True)
                value_host.copy_(value, non_blocking=True)
                sync()

        for _ in range(30):
            call()
        times = []
        for _ in range(calls):
            start = time.perf_counter()
            call()
            times.append(time.perf_counter() - start)
    times.sort()
    median = statistics.median(times)
    return {
        "batch": n,
        "cuda_graph": graph,
        "median_ms": round(median * 1e3, 3),
        "p95_ms": round(times[int(len(times) * 0.95)] * 1e3, 3),
        "decisions_per_s": round(n / median),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--compile", action="store_true", help="also time torch.compile (needs Triton)")
    parser.add_argument("--label", default=platform.system())
    args = parser.parse_args()
    torch.backends.cuda.matmul.allow_tf32 = True
    torch.backends.cudnn.allow_tf32 = True

    model = StandIn().cuda()
    params = sum(p.numel() for p in model.parameters())
    report: dict = {
        "label": args.label,
        "platform": platform.platform(),
        "python": platform.python_version(),
        "torch": torch.__version__,
        "cuda": torch.version.cuda,
        "gpu": torch.cuda.get_device_name(0),
        "params_m": round(params / 1e6, 2),
        "train": [],
        "infer": [],
    }
    for n, steps in ((64, 200), (512, 30)):
        report["train"].append(train_throughput(model, n, steps))
    if args.compile:
        compiled = torch.compile(model)
        for n, steps in ((64, 200), (512, 30)):
            report["train"].append({"compiled": True, **train_throughput(compiled, n, steps)})

    infer = StandIn().cuda().half().eval()
    for graph in (False, True):
        for n in (1, 16, 256):
            report["infer"].append(inference_latency(infer, n, 500 if n < 256 else 200, graph))
    print(json.dumps(report, indent=1))


if __name__ == "__main__":
    main()
