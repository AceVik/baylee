# Windows session, 7 November 2026

For a Claude session on the owner's desktop booted into Windows 11 (Ryzen 9 3900X, RTX 4070 Ti,
two monitors, `docs/perf-client.md` §"Windows"). The window is short: the owner plays on this machine
from about 9:00. Do these in order and stop in time. Close every client you start. Commit to
`c42/shared` by fast-forward, never force; `cargo dev land` stays with the Linux session.

## 1. The key sound

In the shipped `0.1.1-beta.9b` on Windows, every key press plays a Windows system sound. Reproduce it
in the current build and find which keys and which screens. Winit is 0.30; the first suspicion is a key
message nobody handles, so Windows answers with its default beep. That is a suspicion, not a finding.
Fix it, with a test where one is possible.

## 2. PGO for the client

`docs/releasing.md` §"The client's profile" and `crates/baylee-tools/src/pgo/`. Run
`cargo tools pgo client` for `x86_64-pc-windows-msvc`. It needs `llvm-tools`
(`rustup component add llvm-tools`). Note the functions without counts: the limit is 5 %, for the whole
and for `baylee_client` alone. Whether the profile applies from another checkout path is measured on
the Mac; read that first. Keep `merged.profdata`, and say where it is.

Measured on the Mac, 7 November: the profile applies from another checkout path. `cargo tools pgo
client ship` in a second worktree, several commits on from the training, left 6 021 of 253 812 functions
without counts (2.4 %) and 626 of 44 538 in `baylee_client` (1.4 %), against 2.3 % and 1.0 % in the
checkout that trained. So a row on another machine can build from an uploaded profile. Windows paths
were not part of that.

Before the run: `rustup component add llvm-tools`, check `/monitors` and set `BAYLEE_E2E_MONITOR`.
The profile lands in `target/pgo-profiles/client/merged.profdata`. Untested on Windows: the whole run,
leaving through the idle exit and writing the `.profraw`, `dist` with a profile on MSVC, the scenes.
The friends scene clicks the search field at a fixed spot (`window.w - 170`, y 133): check it hits
the field on this layout. The shipped build has no harness, so to compare, build `--release` with
`dev-control` plain and once with the profile applied through `RUSTFLAGS`.

Measure plain against profiled, alternating: frame time in the lobby, a duel, a full board, and an
offline game's CPU time, with the spread.

## 3. If time remains

Deckbuilder on this machine: typing in the search, scrolling, adding cards. On the Mac it does not
stutter, and the owner has seen it stutter somewhere. Report frame times and any spikes over 50 ms.

## Report

What was fixed, the numbers, what is left, and the state of the branch.
