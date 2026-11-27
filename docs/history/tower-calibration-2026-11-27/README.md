# Tower Echoes calibration, 27.11.2026

The measurements behind `docs/tower-operations.md` §"Calibration", as the
Tower lab (`baylee-engine-server` example `tower_lab`) reported them. Each
file is a lab report: counts, scores, 95 % Wilson intervals and loadout
shapes, never a house build. The house ladder's builds come from a private
seed and stay off this repository.

| File | What |
| --- | --- |
| `starters-v1.json` | the fifty starters as they were: 49,000 duels, 20 deals per ordered pair |
| `starters-v2.json` | after the first two calibration rounds, on fresh deals |
| `ladder-v2.json` | the house ladder built from those starters, before any cell calibration |

The commands, run on a Linux build of the example (`cargo build --profile
ci-release -p baylee-engine-server --no-default-features --features tower
--example tower_lab`):

```text
tower_lab --threads N [--seed S] starters --seeds 20
tower_lab --threads N variants --starter K --salts 4 --seeds 2 [--commander NAME]...
tower_lab --threads N ladder --seeds 1 --rise 4            < private seed
tower_lab --threads N candidates --sets 3 --trims 3 --field 12 --seeds 1 < private seed
```
