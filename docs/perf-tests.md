# Beta.8: less setup work in two tests

Local validation, 14 October 2026. Branch `codex/b8-tests`; tested source commit
`9aa40144050ee458f4521d2a8b5ac1682dbb7487`. This identifies the code;
the report is committed separately. No post-push CI result is claimed.

## Scope and coverage

Only test code changes in `crates/baylee-feedback/src/alive.rs` and
`crates/baylee-client-core/src/dial.rs`; production prefixes before
`#[cfg(test)]` are byte-identical to baseline.

- The presence-cap test builds the first 19,999 entries directly, avoiding
  quadratic setup through repeated sweeps of a growing map. Actual `beat` calls test the last legal slot,
  refusal at capacity, refresh while full, and admission exactly at expiry.
  It deliberately stops retesting each intermediate size. Existing small-map
  and expiration tests remain; full-cap refresh/expiry assertions are stronger.
- The tearing-dial test visits only `Spotlight` and `FocusRing`, the only two
  arrangements that previously reached its assertion. All rosters, 2–8 seats,
  five aspects, interests, parked predicates and scale tolerance remain.
  Both arrangements must reach an assertion: **635 each** in both green fault
  controls. The general clearance test still visits all eight arrangements.

## A/B/A measurements

Apple M1 Max, 64 GiB, Rust 1.99.0, default Nextest profile,
`--workspace --all-targets --no-fail-fast`. All heavy work is serialized by the
global Cargo lock. A1 is the parent baseline on beta.7
`7e4aa634ee630b50af552de2550ac8d520b15119`; P4 starts at
`78164a73e4ffd6089a73a2c64d7ba00b3c749da2` (two additional docs commits).

| Measurement | A1 baseline | B candidate | A2 return |
|---|---:|---:|---:|
| Suite, excluding build | 335.556 s | 325.665 s | 331.917 s |
| Passed / skipped | 14,344 / 21 | 14,344 / 21 | 14,344 / 21 |
| Presence cap | 35.789 s | 0.046 s | 35.259 s |
| Tearing dial | 25.199 s | 0.564 s | 25.281 s |

All 14,344 test names match across the three runs. [Before data](perf-data/beta8-tests-before.json)
and [after data](perf-data/beta8-tests-after.json) retain all three top-50 lists,
source/log hashes, gate outcomes and raw fault-environment evidence.

B and A2 use identical corrected V2 isolation, compiler, target and profile;
A1 did not establish matching isolation. A2 temporarily restores **only the two
test files from HEAD**, verifies their production prefixes match B, and restores
B in `finally`. Both restored SHA-256 hashes equal the frozen gated candidate.
A2 takes 444.968 s including compilation.

The suite differences are descriptive: B is 9.891 s shorter than A1;
A2 − B is 6.252 s and A2 − A1 is
-3.639 s. One cycle, changed initial isolation and
parallel test scheduling do **not** establish a causal 3% suite improvement.
Per-test reductions and fault coverage are the direct evidence for these changes;
summed test-case seconds are not job wall-time savings.

## Fault proof

One outer lock ran nine commands: two green controls, five independently
compiled faults, then both restored controls. A red counted only when the
named test reached its assertion, never for compilation failure.

| Temporary fault | Observed assertion failure |
|---|---|
| Remove cap guard | 20,001 ≠ 20,000 |
| Reject at `MAX_GAMES - 1` | last slot refused: 19,999 ≠ 20,000 |
| Reject all beats while full | timestamp was not refreshed by 60 s |
| Omit sweep before checking cap | newcomer refused: 1 ≠ 2 survivors |
| Tearing scale `MIN_SCALE + 0.1` | Spotlight, 3 seats, aspect 2.8: scale 1.1 |

**5/5 assertion faults detected; 4/4 controls green.** All source bytes restored.
The expiry fault cannot hide behind `count()`'s sweep: the refused newcomer is
still absent. [Fault data](perf-data/beta8-tests-faults.json) contains exact
mutations, messages and checksums. Isolated control times were cap 0.03/0.03 s,
dial 0.53/0.52 s; nine commands totaled 91.028 s including compilation. These
isolated times are separate from the full-suite measurements.

## Gates and repaired local isolation

| Check | Result | Command wall time |
|---|---|---:|
| Six previously failing tests | 6/6 passed; 7.104 s execution | 70.106 s |
| `scripts/gate.sh` | exit 0, V2 | 364.956 s |
| `scripts/gate-features.sh` | exit 0, V2 | 411.692 s |
| `scripts/gate-wasm.sh` | exit 0, initial run reused with identical sources | 16.358 s |

Main gate: fmt, Clippy, full Nextest, cache and card validation passed. Features:
all steps passed, including 1,991 client tests (25 `devctl::`, floor 15) and
`dev-table` gateway tests. WASM checks the eight browser crates. Sources stayed
frozen throughout the gates. A2 followed in a separate global-lock acquisition.

The initial main/feature attempt is retained as failure evidence, **excluded
from performance evidence**. Four gateway tests lost agent profiles because the
original profile allowed loopback IP but denied test Unix sockets; the direct
Unix connection reported `EPERM`. Two `cli_mind` process-death assertions failed
because spawning setuid `/bin/ps` returned `EPERM`; their `exited()` predicate
also returns false on spawn failure. That did not prove the child was alive.

V2 permits Unix sockets only beneath its test temporary directory. A private
copy of Apple's actual `ps` precedes PATH, mode `04755` → `0755`, ad-hoc signed.
Copy/chmod preserve the original bytes; signing changes the whole-file hash.
Independent reconstruction reproduces the signed hash; every file-backed
Mach-O section matches (7 x86_64, 9 arm64e). The same owned PID under the exact
sandbox returns live/nonempty status then dead/empty status after terminate/wait;
the unchanged predicate is false then true. Provenance and raw probe results
are in the after JSON. This proves the limited owned-child macOS setuid/sandbox
case, not general equivalence to privileged `ps`. **No Apple binary is committed.**
No production code, assertion, complete-suite filter or skip was changed for V2.

Runner: clean environment whitelist, isolated XDG, unchanged HOME, test TMPDIR,
`CARGO_INCREMENTAL=0`, `CARGO_NET_OFFLINE=true`, shared target
`$HOME/Projects/baylee/target`, local PostgreSQL, external network denied.
PostgreSQL remains running. No interactive client, production service or real
model/API call. Card validation uses cached payloads (oldest 23 days), not live
Scryfall. The nonfatal `__eh_frame section too large` warning also existed in A1.

## Historical CI estimate

Historical [run 38484576125](https://github.com/AceVik/baylee/actions/runs/38484576125)
on beta.7 put both tests in `test (1/2)`: cap 85.133 s, dial 89.484 s; job wall
1,730 s, Nextest 1,094.176 s. The after JSON retains three historical run inputs.

Scaling those two costs by local B/A1 ratios estimates **172.50
saved test-case seconds**; using B/A2 ratios instead gives
**172.51 s** (172.50–172.51 s sensitivity).
Formula per test: `historical_seconds * (1 - B / A)`.
This transfers macOS debug ratios to Linux. It predicts neither parallel job
wall time nor release-test speedup; no measured CI saving or post-push CI result.
