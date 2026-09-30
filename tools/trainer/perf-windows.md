# Performance baseline: Windows 11 + WSL2 (30 September 2026)

Measured on the PC before the switch to native Ubuntu, so the two can be
compared. **First task on Ubuntu:** re-run these same commands and send
baylee-d7 the difference. Windows stays in the dual boot, so a number
missing here can still be measured there later.

## Environment

- CPU: Ryzen 9 3900X, 12 cores / 24 threads. RAM: 64 GB.
- WSL2 (Ubuntu 26.04, kernel 6.18.33), `.wslconfig`: `memory=48GB`,
  `swap=16GB`, all cores. The disk is a 451 GB `ext4.vhdx` on C:.
- GPU: RTX 4070 Ti, 12 GB; driver 617.14 (Studio), CUDA 13.4.
- torch 2.14 with cu132, Python 3.13 (uv).
- The code: branch `c42/trained-ai` at b40e71d3 (the RL numbers; the engine
  of main 89e5eb13), and fdd18c33 (the gate, the dataset load).

## One RL iteration (iteration 19, 30 September 12:29 → 13:18)

The loop as `rl_loop.sh` runs it: `BASE=~/baylee-data/models/v3-a-dyn
ITERS=20 GAMES=2000 ARENA=1000 TAG=rl TEMP=0.7 nice -n 5
tools/trainer/rl_loop.sh`. The stage times are the log files' timestamps.

| stage | command | time | rate |
|---|---|---|---|
| league | `./target/selfplay/league --learner rl-18/net.onnx --temperature 0.7 --league house:expert,house:sharp,house:steady,net:policy-v1,self,net:rl-16,net:rl-17 --games 2000` | 30.6 min (`summary.json` seconds 1843) | 1.09 games/s, 24 workers (default: one per core), ONNX on CPU |
| convert | `./target/selfplay/convert3 --runs runs/rl-l19 --out datasets/d3-rl-l19 --every 2` | 18.2 s | 106 games/s, 659,207 decisions |
| train | `uv run python train_rl.py --data d3-rl-l19 --init rl-18/net.pt --out rl-19 --resume` (PPO, batch 256, lr 5e-5, 1 epoch, bf16 autocast) | 6 min for 2,335 steps (8.0 min for the stage, with load, advantages and export) | 6.5 steps/s, ~1,660 samples/s |
| export | `uv run python export_onnx3.py --model rl-19 --data d3-rl-l19` | included in the stage above | parity 9.5e-6 |
| arena | `./target/selfplay/arena --model rl-19/net.onnx --against expert --games 1000` | 10.3 min (616 s) | 1.62 games/s, 16.9 ms per net answer (CPU, 24 threads, 783k answers) |
| iteration | | 49.3 min | |

Iteration 18 is the same shape: league 1621 s, arena 568 s at 15.1 ms per
answer.

## Inference

- CPU (ONNX Runtime, feature `onnx`, row buckets 32–192, 24 games at once):
  15–17 ms per answer in the arena (above). The 56 ms figure was v3-a's
  first export, before buckets and the skip of forced answers.
- GPU (`onnx-cuda`): not measured on Windows.

## Training throughput

- RL step: ~1,660 samples/s (above).
- GPU utilisation and VRAM peak were not sampled during the RL steps.
  Earlier, `train3.py` on d3-r003 ran at about 95 % utilisation. On Ubuntu,
  sample `nvidia-smi --query-gpu=utilization.gpu,memory.used --format=csv -l
  1` during a training stage; do the same on Windows if the comparison
  needs it.

## Disk I/O

- One league dataset (d3-rl-l19: 7.3 GB on disk, 7.81 GB of array files
  with `merged/`), with the page cache dropped first (`sync; echo 3 >
  /proc/sys/vm/drop_caches` as root): open and memory-map 0.2 s; every
  array file read once 20.6 s, 380 MB/s. A gate was running at the same
  time. The script is `loadtime.py` below: open with `dataset3.load`, then
  `np.fromfile` on every array file.
- The copy of the new data (about 7 GB) to C:\baylee-data over WSL's `/mnt/c`
  took 484 s, about 15 MB/s. That is WSL's 9p file share, not the disks.

## Build and gates

- `scripts/gate.sh` (`DATABASE_URL` set) under the RL loop's load: fmt 18 s,
  clippy 238 s, nextest 599 s (7,907 tests), scryfall-cache 79 s,
  validate 3 s. That is about 16 min.
- `scripts/gate-features.sh`: 1,118 s of steps (client features 138 + 405
  s, dev-table 77 + 325 s, …). That is about 19 min.
- A cold `cargo build` was not timed on Windows. The earlier A/B (Windows
  native vs WSL) gave 78 s vs 41 s for the same build (68 s natively after the Defender exclusion). On Ubuntu,
  time `cargo clean && cargo build --profile selfplay -p baylee-train
  --features onnx --bin league` (cold), then touch
  `crates/baylee-train/src/policy.rs` and build again (warm).

## `loadtime.py`

```python
import sys, time
from pathlib import Path
import numpy as np
sys.path.insert(0, "tools/trainer")
import dataset3 as D3
path = Path(sys.argv[1])
t0 = time.time(); ds = D3.load(path); t1 = time.time(); total = 0
for f in sorted(path.rglob("*")):
    if f.is_file() and f.suffix in (".i32", ".i16", ".f32", ".u8", ".i64", ".u16", ".i8", ".npy", ".bin"):
        total += np.fromfile(f, dtype=np.uint8).nbytes
t2 = time.time()
print(f"open {t1 - t0:.1f} s; {total / 1e9:.2f} GB in {t2 - t1:.1f} s = {total / 1e6 / (t2 - t1):.0f} MB/s")
```
