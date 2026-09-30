# Performance: native Ubuntu against Windows 11 + WSL2 (30 September 2026)

The commands of `perf-windows.md`, run again on the same PC after the switch
to native Ubuntu, on the same code. The outputs are in
`~/baylee-data/perf-linux/`.

## Environment

- The same machine: Ryzen 9 3900X (12C/24T), RTX 4070 Ti 12 GB, 64 GB RAM
  (60 GB visible).
- Ubuntu 26.04.1, kernel 7.0.0, native on ext4 (Samsung 990 PRO). CPU
  governor `schedutil`.
- NVIDIA driver 595.91.07 (CUDA 13.2); torch 2.14.0+cu132, Python 3.13.15,
  onnxruntime 1.30.0.
- rustc 1.98.1, linked by clang with mold 2.40.4 (`~/.cargo/config.toml`).
  The linker WSL used was not recorded.
- The code:
  - the RL stages on `c42/trained-ai` at b40e71d3, the build that played
    iteration 19 on Windows (`0.1.0-beta.3+build.2064`);
  - the gates on fdd18c33 and then deaf2258 (the same Rust code).

## One RL iteration (iteration 19 again)

The league was played again with the same learner (rl-18), the same league,
the same seeds and the same run name (`--name rl-l19`, which seeds its
games). The train stage started from rl-18 on that run's dataset. The arena
played Windows' rl-19 against expert.

| stage | Windows + WSL2 | Ubuntu | faster |
|---|---|---|---|
| league, 2000 games | 1843 s, 1.09 games/s | 1508 s, 1.33 games/s | 1.22× |
| … its 1927 games that played identically on both | 33,119 game-seconds | 25,172 game-seconds | 1.32× (median per game 1.30×) |
| convert | 18.2 s, 106 games/s | 17.2 s, 113 games/s | 1.06× |
| train + export (load, advantages, PPO, export) | 8.0 min | 6.1 min (357.7 + 7.9 s) | 1.31× |
| arena, 1000 games vs expert | 616 s, 16.9 ms per answer | 561 s, 14.1 ms per answer | 1.10× wall, 1.20× per answer |
| **iteration** (sum of the stages) | **49.3 min** | **40.9 min** | **1.21×** |

- **Why the league gains less than a game.** A game still going at the cap
  (`--max-secs 120`) costs 120 s on either system. The cap stopped 73
  games on Windows and 68 on Ubuntu; each of those 68 got further before
  it. The capped games are about a fifth of the league's worker time,
  which makes them the next thing to cut.
- **The arena.** Its wall time gains less than its per-answer time: in its
  115 capped games (60 s) Ubuntu answers more (857k answers against 783k).
- **The PPO step.**
  - The log's minute counter reads 5 min for 2,348 steps; on Windows it
    read 6 min for 2,335.
  - `nvidia-smi` at 1 s: GPU at 92 % over 348 s, which covers the
    advantages, the steps and the held-out pass. That is at least 6.75
    steps/s, or 1,727 samples/s (Windows: ~6.5 and ~1,660).
  - VRAM peak 4.2 GB, 179 W mean.

## Disk I/O

- **A cold read of one league dataset** (d3-rl-l19 with `merged/`, 7.98 GB),
  using `loadtime.py` from `perf-windows.md`:
  - Windows: open 0.2 s, read at 380 MB/s, with a gate running.
  - Ubuntu: open 0.1 s, read in 3.9 s at 2,038 MB/s, on an idle machine.
  - Without root, the pages were evicted per file with
    `posix_fadvise(DONTNEED)` instead of `drop_caches`.
- **Copying the data across.** The 80 GB from Windows' C: (ntfs3, read-only)
  onto ext4 took 89 s, about 960 MB/s. WSL's 9p share managed 15 MB/s.

## Build and gates

- **A cold build.** `cargo build --profile selfplay -p baylee-train
  --features onnx --bin league` in a fresh worktree, crates already fetched,
  took 44.4 s (203 s of CPU). After a touch of `policy.rs`: 22.9 s.
  - This was not timed on Windows. The earlier A/B gave 41 s under WSL and
    68 s natively for an unrecorded build.
- **The gates.** Windows' idle gates at 13:41 and 13:46 ran on a warm target
  right after a commit. A commit rebuilds everything `baylee-build` stamps.
  - Ubuntu's warm run repeats that: first a cold run on fdd18c33, then a
    commit (deaf2258), then the warm run.
  - The gate numbers in `perf-windows.md` were taken under the RL loop's load
    (gate ~16 min, gate-features ~19 min). They compare with nothing
    measured here.

`scripts/gate.sh`, 7,790 tests, in seconds:

| step | Windows, warm, idle | Ubuntu, warm | Ubuntu, cold |
|---|---|---|---|
| fmt | 10 | 7 | 7 |
| clippy | 14 | 7 | 84 |
| test (build + nextest) | 230 (nextest 123) | 104 (nextest 87) | 248 (nextest 89) |
| scryfall-cache + validate | 1 | 1 | 26 |
| **total** | **255** | **120** | **365** |

`scripts/gate-features.sh`: Windows warm 283 s, Ubuntu warm 215 s, Ubuntu
cold 512 s. The largest step is still `dev-table-test`: 180 s on Windows,
140 s on Ubuntu.

On the merge with main 7160e89b (8,002 tests), both gates cold: gate.sh
372 s, gate-features.sh 534 s.

- **umask.** Ubuntu's default umask for a user is 002.
  `baylee-update::apply_tree::macos_in_applications_touches_nothing_but_the_bundle`
  writes a file and expects mode 644, so it fails under 002 and passes
  under 022 (CI). Run the gate under `umask 022` until the test sets the
  mode itself.

## Replaying a run

- **A league replays across machines.** 1,927 of iteration 19's 2,000 games
  played out move for move the same on both systems: outcome, turn,
  answers and record length. The other 73 are all capped games.
- **An arena does not.**
  - Of the 870 games both arenas finished, 557 are identical.
  - 53 changed winner, and the rate against expert moved from 48.6 % to
    47.5 %. The arena's own interval is ±3.3 points.
  - The net answers at temperature 0, on one intra-op thread, on the same
    build and working set.
  - The first difference in a game is almost always the number of net
    answers, even where outcome and turn agree.
  - Until the cause is known, `paired_arena.py` pairs are weaker than "the
    same game twice". A Linux-against-Linux repeat is next.
