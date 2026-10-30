# Prepared host package — no activation

`prepare.rs` is a standalone Rust preparation helper linked against the existing
operations library. It implements only `prepare`, `verify-prepared`, `seal` and
`verify`. Its only systemd command is a read-only `systemctl --user show` with
explicit non-secret identity/state properties. It never reads process
environments or private credentials and contains no service lifecycle operation.

The reviewed package is:

`/home/ace/baylee-data/v5/ops-codex-20261028/refactor-investigations-20261029/native-host-deployment-20261029-01a187a6-9180-7739-a78a-7ecbd0758c01`

It contains the copied helper/source, hashed original unit backups, proposed unit
files, original reviewed profiles, manifest templates, a non-secret unit
inventory and `prepared-artifacts.json`. Verification checks every prepared
artifact and the unchanged installed unit bytes. The three active host units
have no drop-ins. Preparation preserved preview PID 9331, arena PID 95824 and
lane2 PID 95768. These are observations, not future permission to interrupt them.

Release binaries are now sealed; see [SEALED.md](SEALED.md) for the exact build
identity, pins and validation receipts. `release-bin-review.pending.json` remains
an unfilled historical template, not approval. To prepare a future package, the owner creates a
separate review JSON containing `profile: "release"` and literal source paths
and SHA256 values for `serve_local4` and `collect_local4`. The helper preflights
both pins before copying anything, rejects debug paths and missing pins, copies
into this UUID package's `bin/`, makes the copies read-only/executable, seals the
new collection path, and runs native supervisor manifest validation. `verify`
then checks the immutable binary receipt and all runtime profiles without
starting them. The final receipt pins the runtime manifest bytes as well as the
binary bytes. No existing gateway, agent, engine, seat or model is copied,
replaced or promoted.

The proposed units change only `ExecStart` and set `Restart=no`; the latter
prevents systemd from automatically retrying a failed native controller. Their
working directories, logging, kill mode and stop timeout remain unchanged.
The arena profile carries `HOLD_ACTIVE_GAME`; a fresh, explicit drain review is
required. Preview and lane2 likewise need a fresh idle check. Installation,
daemon reload and controlled handover belong to the owner after review and
are deliberately absent from this helper.

The read-only inventory found five inactive/failed Python or shell controller
candidates: `baylee-arena-v5.service`, `baylee-arena-v5-lane2.service`,
`baylee-continuous-v5-training.service`, `baylee-v5-full-coverage-eval.service`
and `baylee-terra-flash-balanced.service`. They are static units with no paired
timer found. Review their completion/hold receipts first; archive original bytes
and disable/mask obsolete controllers rather than leave an old Python wrapper
reactivatable. This is a proposal only; no unit was disabled or masked. Actual
PyTorch worker code and immutable historical experiment evidence remain separate
from these controller candidates.

Validation performed: helper compiled; `prepare` and `verify-prepared` passed;
`seal` with the unfilled review template was refused before any binary copy.
The initial preparation claimed no release pin; the later sealing is recorded
separately. No service activation occurred. Earlier preliminary
packages ending `01a187a2-7d54-70bb-b1e4-3bfa21f0bcbf` and
`01a187a4-8d17-70c7-a99a-f4f483e3514b` are superseded and must not be deployed.
