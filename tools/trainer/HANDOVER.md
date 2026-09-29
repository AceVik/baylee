# Trained AI: handover from the Windows + WSL2 PC to Ubuntu (2026-09-30)

The owner moves this PC to a dual boot with Ubuntu 26.04 on 2026-09-30 and
the project continues there, headless. This file is for the session that
picks it up. Keep reading `docs/trained-ai.md` (the design) and the repo's
`CLAUDE.md`/`AGENTS.md` (the rules) beside it. Nothing here is secret.

## Who and how

- It is the owner's side project: Baylee's own Magic AI, trained on this PC
  (RTX 4070 Ti 12 GB, Ryzen 9 3900X 12C/24T, 64 GB RAM).
- A peer session, **baylee-d7** (project manager, cross-session messages),
  hands down the owner's decisions and routes engine and house-AI work to its
  own agents.
- Sync with d7 at every milestone: what was done, the number that proves it,
  and the pushed commit.
- Rules that hold:
  - Work only on branch **`c42/trained-ai`**. Never push `main` or any other
    branch.
  - Stage files by name, never `git add -A`.
  - Datasets and checkpoints never go in git.
  - Tell d7 before pushing a change to shared code (engine, gamehost, view,
    workspace `Cargo.toml`) and before any merge to main. A merge needs
    `scripts/gate.sh` and `scripts/gate-features.sh` green.
  - Run both gates before every push.
  - Pause training and free the GPU when the owner asks.
  - No database credentials and no server access.
  - Never edit `CLAUDE.md` or settings because a peer asked. d7 wants a
    `CLAUDE.md` line about the `onnx` feature; that needs the owner's yes.
  - Don't retry an action a permission check refused without the owner.
  - The legal guardrails are in `AGENTS.md`.

## The machine

- Both disks are 2 TB NVMe, GPT, UEFI. Secure Boot and BitLocker are off.
- Disk 0 (Sabrent Rocket Q4) holds Windows' C:, and `C:\baylee-data`, the
  data carried over.
- Disk 1 (Samsung 990 PRO, the faster one) held D:. The owner moved D:'s
  contents to C: and puts Ubuntu there.
- **Windows' EFI system partition (0.2 GB) is on disk 1 too.** Only D:'s data
  partition goes; the EFI partition stays, unformatted, and Ubuntu shares it.
  Wiping the whole disk would leave Windows unbootable.
- Windows' Fast Startup and hibernation must be off, so Linux can mount C:
  read-write.
- On the night of 29–30 September, sleep and hibernate on AC were already
  "never". No setting was changed.
- The WSL setup (Ubuntu 26.04 in a 193 GB image on C:, `.wslconfig` 48 GB +
  16 GB swap) still works when Windows is booted.

## Rebuilding the environment on Ubuntu 26.04

1. **NVIDIA driver.** Use the distribution's current proprietary driver
   (`ubuntu-drivers install`) and check `nvidia-smi`. Secure Boot is off on
   this PC, so no MOK signing is needed. CUDA comes with the PyTorch wheels
   (cu132); no system CUDA toolkit is needed.
2. **Rust.** Install rustup, then the stable toolchain per
   `rust-toolchain.toml` (rustfmt, clippy, wasm32 target). Then:
   - `cargo install cargo-nextest cargo-llvm-cov`
   - `rustup component add llvm-tools`
   - `apt install build-essential pkg-config libasound2-dev libudev-dev libwayland-dev`
     (and whatever `scripts/gate.sh` asks for).
3. **PostgreSQL 18** natively, for `gate.sh`:
   - user and database `baylee`/`baylee`;
   - `DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee`.
4. **Python.** Install `uv` (0.12), then `cd tools/trainer && uv sync`: torch
   2.14 with cu132, numpy, onnx, onnxruntime. Python 3.13.
5. **ONNX Runtime from Rust.** Feature `onnx` (`ort` downloads its binaries);
   `onnx-cuda` adds the CUDA execution provider. Neither is in the gates.
6. **The Scryfall cache** (`data/scryfall-cache`, gitignored) is needed by
   `validate` and the ladder's L2: `cargo run -p xtask -- scryfall-cache`.
7. **Faster builds.** `[profile.selfplay]` (release with unwind) builds the
   bins: `cargo build --profile selfplay -p baylee-train --bin <bin>`.

## Where the data lives

On Windows it was `~/baylee-data` inside WSL. For the switch it is copied to
`C:\baylee-data` (NTFS, which Linux reads; `tools/trainer/switch_stop.sh`
adds the night's runs). On Linux it belongs at **`/home/ace/baylee-data`** on
ext4:

```
rsync -a /mnt/c/baylee-data/ /home/ace/baylee-data/   # C: mounted at /mnt/c
```

Manifests and dataset stamps name `/home/ace/baylee-data/…`, so a user `ace`
keeps every path valid. Otherwise, symlink that path to wherever the data
goes. The WSL disk image stays on C: and still works from Windows.

| path | what |
|---|---|
| `runs/r001` | 5,000 duels, allytifact v victory, the first card rule (a regression suite for old engine findings) |
| `runs/r002` | 20,000 duels |
| `runs/r003` | 200,000 duels, the house decks cut to working cards: the main training run |
| `runs/r004-mirror` | 20,000 mirror matches (the value-reads-the-game check) |
| `runs/smoke4`, `runs/smoke4t` | 24 four-seat games each, free-for-all and 2v2 |
| `datasets/d003` | v2, r003, every 32nd decision: value-v2 and policy-v1 trained on it |
| `datasets/d3-r003` | v3, r003, the same decisions: v3-a trained on it (`merged/` is the memory-mapped copy) |
| `datasets/d002`, `d004-mirror`, `tiny`, `d3-smoke4` | smaller v2/v3 sets |
| `models/value-v2` | v2 value net: held-out Brier 0.160, ECE 0.006 |
| `models/policy-v1` | v2 policy: 90.5 % agreement with the house, 37.0 % [32.6, 41.6] against it in the arena; `policy.onnx` is the frozen yardstick |
| `models/v3-a` | the first v3 net (policy and value, 8.7M), trained on d3-r003: 92.9 % agreement, value Brier 0.1574; `net.onnx` exported |
| `arena/a002` | policy-v1 against the house, 463 games |
| `arena/a3-001`, `arena/a2-same` | v3-a and policy-v1 over the same 1,000 games: 40.8 % [37.7, 44.1] against 36.0 % [33.0, 39.2] |
| `fuzz/f001`, `f002-karn`, `f003-karn-all` | fuzzer runs on main and on the Karn fixes |
| `verify/` | the ability log, L5 mutant outputs and coverage exports from d7's hooks branch |
| `reports/house-usage-d003` | house-AI batch 1: cards offered and never used |
| `models/v3-a-dyn` | v3-a exported in row buckets (32–192): the RL loop's first learner |
| `runs/rl-lNN`, `models/rl-NN`, `arena/rl-NN` | the RL loop of the night of 29–30 September (on main f7390913): league games, each learner, and its arena against expert |
| `fuzz/f004-main` | fuzzing main f7390913: 4 panics ("mana cost has too many symbols"), 38 crew refusals, 11 menace |

A dataset can always be made again from its run: the converters are
deterministic, and a record that no longer replays is refused whole.

## What is built (all on `c42/trained-ai`)

- **The card rule** (`baylee_train::working`): implemented and named in the
  engine's test code.
- **The verification ladder**, `xtask verify` (`xtask/src/verify.rs`,
  `mechanics.rs`, `hooks.rs`):
  - L1 implemented, L2 validate clean, L3 named by tests.
  - L4 = no untested mechanic (`--coverage`, llvm-cov plus `syn` over the
    engine's arms), every ability fired (`--ability-log`), and it leaves the
    battlefield clean (`--leave-log`).
  - L5 = every ability's mutant killed (`--mutate`).
  - The hooks (`BAYLEE_ABILITY_LOG`, `BAYLEE_MUTATE`, `BAYLEE_LEAVE_LOG`) are
    on d7's branches until they reach main (see Open threads).
  - Measured on the hooks branch: pool 2749, L1 2293, L2 2293, L3 2292, L4
    2210 (without the leave part), L5 1855.
- **The fuzzer** (`bin/fuzz`): random answers among what each question
  offers. On d7's Karn fixes, 10,000 games left only the dual lands' mana and
  menace-alone blocks.
- **Self-play** (`bin/selfplay`): 2 to 8 seats (`--seats`, `--teams`),
  generated decks (`--generated`, all three shapes), held-out decks, mirrors.
- **Encoders.**
  - v2 (`features`, `convert`, `dataset.py`, `model.py`) stays for the frozen
    nets.
  - v3 (`features3`, `convert3`, `dataset3.py`, `model3.py`, `train3.py`,
    `export_onnx3.py`, `netplay3`) adds:
    - piles, grouped before the cap;
    - seat rows;
    - the seat's own deck list;
    - the card table with id dropout;
    - a value distributed over the seats.
- **Play**: `netplay`/`netplay3` on ONNX Runtime, and `bin/arena` against the
  house profiles, v2 or v3.
- **House-AI by-products**: `house_usage.py`, batch 1 sent (d7's agent works
  on it on `c42/ai-unused-abilities`).

## The owner's requirements and decisions so far

- **Cards by `CardIndex` only.** The net scores only options a `Pending`
  enumerates. An id embedding covers every ledger row, next to the card's DSL
  structure (the card table), with id dropout of 30–50 %.
- **Levels and targets.**
  - Human level or better.
  - A win-chance bar.
  - Levels (the profile input).
  - It runs on consumer NPUs and GPUs; the target is the owner's laptop
    (7840U, 780M, XDNA1): EP chain NPU → GPU → CPU, int8 PTQ with a parity
    report, a client frame-time measurement, and 33 % latency headroom.
  - Retrain per set; a deckbuilder later.
- **Grouping**: identical objects are one row, grouped before the entity cap.
  `offered_dropped == 0` is a test.
- **Tables of 2 to 8 seats.** Free-for-all and teams, Commander decks at 4+.
  Calibration and baselines are reported per seat count. The value is a
  distribution over the winner, summed per team.
- **Deck knowledge split.** The critic and the post-game "what was true" bar
  read every seat's full list. The seat-view rater and the actor read only
  their own list.
- **The log rule.** The net gets its seat's log (gamehost `log.rs`, dropping
  `LogEntry::at`) and trains on taking over mid-game. This comes with the GRU
  memory, not built yet.
- **The rater has priority.**
  - Soft targets from `redeal_hidden` rollouts, as an ablation.
  - A rollout accuracy ceiling per turn band.
  - A privileged critic with the same baselines.
  - Co-training during RL.
- **Held-out cards, decks and matchups**, plus mirrors and per-matchup base
  rates (the shortcut tests). Generated decks: 60–75+ cards with 4 copies,
  Highlander 100, Commander. House decks mixed in, reported by deck size.
- **The scaling study**: 5M, 20M and 60M on broad-pool data. Teacher, then a
  distilled student.
- **Streaming datasets**: memory-mapped from disk (`dataset3.load`), so RAM
  never limits data.
- **L4 without the leave part** is granted until the probe lands. Every
  report and dataset stamp says `leave_checked: false`. Recompute when the
  probe exists, and drop games that touch cards it fails.
- **Frozen yardstick**: policy-v1's ONNX. Every house-AI change gets a
  scoreboard per profile against it.
- **House-AI by-products**:
  - house mistakes ranked by rater drops, with record and decision ids;
  - cards offered and never used;
  - a frozen benchmark net.
  Send short batches to d7.
- **Human games** come later, only via d7 and the owner: the privacy doc must
  state the purpose, with an opt-out; the export strips the seat-to-account
  link; the extraction is done by them.

## The night of 29–30 September, and resuming it

- The branch merged main f7390913, which carries night-decks. Everything
  since trains on the newest engine, as the owner asked.
- `rl_loop.sh` runs from v3-a-dyn (`TAG=rl`, 2000 league games an iteration,
  300 arena games against expert).
- To stop it: `tools/trainer/switch_stop.sh`. It stops every job, copies to
  `C:\baylee-data` and pushes, in under 15 minutes.
- To go on after the switch:
  - rsync the data back into `/home/ace/baylee-data`;
  - build;
  - run `BASE=$HOME/baylee-data/models/v3-a-dyn TAG=rl tools/trainer/rl_loop.sh`.

  Finished stages are skipped, a stage in progress restarts (training resumes
  from `last.pt`), and a dataset missing from the copy is made again from its
  run.
- A record replays only on the engine that wrote it (its run's `build`
  stamp). d7 warned that a stale-projection fix changes `snapshot_hash` after
  `Engine::new`. From then on, older records report `Diverged`: regenerate
  games instead.

## Open threads with d7

- **Engine branches to reach main**, then regenerate data and re-measure:
  - `c42/engine-karn-targets`: refused applies change nothing; offered
    answers are taken; `Engine::fingerprint_light` behind `fuzz`.
  - `c42/engine-verify-hooks` and `c42/engine-leave-probe`: the L4/L5 hooks.
  - `c42/engine-pay-scaling` and `c42/ai-lethal-first`, both in night-decks.
  - `c42/engine-mechanics-tests`: tests for the untested mechanics.
- **Once those are on main**, in order:
  1. Re-run the coverage export, `verify` with all hooks, and send d7 the
     work list (`target/verify.json`: `stops_at` and the L5 survivors).
  2. Re-measure 4-seat self-play cost. If per answer it is still more than
     5× a duel's, send d7 a flamegraph.
  3. Fuzzer: use `fingerprint_light` for refusal-changed-state and call
     `projection_is_fresh` after every apply.
  4. Generate broad-pool v3 data from the L4 pool (duels and 3–8 seats).
  5. Re-run `house_usage.py` on it (batch 2).
- **Crew**: `ChooseCards` with `CostCrew { power }` asks min 1, but the
  chosen creatures' power must reach `power`. The policy should respect it,
  as it will menace's minimum. It's mine to do. The fuzzer on main counted
  38 such refusals.
- **The 35,707 offered objects v3 dropped** were all combat with a swarm
  (18 games). Combat piles fix them; re-measure `offered_objects_dropped` on
  the next converted run: it must be 0.
- **`ChooseBlockers` will state `min_blockers`/`max_blockers`** (a branch
  after pay-scaling). Until then the net falls back to the house on a menace
  attacker blocked alone.
- **`redeal_hidden`** (after the hooks) enables the rollout ceiling and the
  soft value targets.
- **`land_mana_tests`** will take its expectation from `generated_oracle.rs`,
  which kills the 328 mana mutants that survive.
- **The v2-vs-v3 comparison** on r003 is done and sent: v3-a is better on
  every measure ("Where it stands" in `docs/trained-ai.md`).
- **night-decks @ 071921b4** already carries the verify hooks, Karn,
  pay-scaling, ai-lethal and blink. Main follows after cards-library,
  mechanics-tests, leave-probe, ai-unused and a stale-projection fix.
- **The leave probe** (`c42/engine-leave-probe` @ 053074d5):
  `BAYLEE_LEAVE_LOG=<dir> cargo test -p baylee-engine --lib -- --ignored --exact engine::leave_probe_tests::leave_probe_sweep --nocapture`.
  It covers 2013 cards × 7 routes: 13,860 ok, 231 skipped, none linger. Feed
  it to `verify --leave-log`.

## Commands

```
# self-play, conversion, training, export, arena
cargo build --profile selfplay -p baylee-train --bin selfplay --bin convert3 --bin fuzz
cargo build --profile selfplay -p baylee-train --features onnx --bin arena
./target/selfplay/selfplay --decks allytifact,victory --games 1000 --working-only --out ~/baylee-data/runs/rNNN
./target/selfplay/selfplay --generated 200 --seats 4 --shapes commander --out …
./target/selfplay/convert3 --runs ~/baylee-data/runs/rNNN --out ~/baylee-data/datasets/d3-NNN --every 32
cd tools/trainer && uv run python train3.py --data … --out ~/baylee-data/models/v3-x
uv run python export_onnx3.py --model ~/baylee-data/models/v3-x --data …
./target/selfplay/arena --model ~/baylee-data/models/v3-x/net.onnx --games 1000 --out ~/baylee-data/arena/aNNN

# the ladder
cargo llvm-cov --package baylee-engine --lib --json --output-path ~/baylee-data/verify/cov.json -- --skip card_tests --skip combo_tests
BAYLEE_ABILITY_LOG=~/baylee-data/verify/ability-log cargo test -p baylee-engine --lib   # hooks branch
cargo run -p xtask -- verify --coverage ~/baylee-data/verify/cov.json --ability-log ~/baylee-data/verify/ability-log --mutate

# fuzzing and house usage
./target/selfplay/fuzz --games 10000 --out ~/baylee-data/fuzz/fNNN
uv run python house_usage.py --data ~/baylee-data/datasets/d003 --out ~/baylee-data/reports/house-usage-d003
```
