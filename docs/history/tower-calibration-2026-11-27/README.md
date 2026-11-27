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

| Later files | What |
| --- | --- |
| `starters-v3.json` | the final fifty on fresh deals: all inside 40–60 % |
| `candidates-v1.json` | first cell calibration: every cell's six candidates against set 0 |
| `ladder-v3.json` | the ladder after the first pick table, fresh deals |
| `candidates-v2.json` | second round around the published picks |
| `ladder-v4.json` | the published ladder, fresh deals: 505 of 600 cells inside 30–70 % |

The calibrated content's fingerprint is
`6f8b0c7a6cc0babb94a7c6ae3c277b73161e35627086ada97c1b16db57098364`
(`baylee_tower_content::CALIBRATED`), measured at commit `aabb8b499` on the
release `e476745f7`'s card registry.
