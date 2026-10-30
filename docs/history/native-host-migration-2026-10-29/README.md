# Native local-host migration review — 2026-10-29

Current status: all three hosts now run the sealed native controller. Five
obsolete inactive controller units were archived and masked under explicit
owner authorization. See [OPERATIONS.md](OPERATIONS.md) for the handover,
orphan-session ownership evidence and final verification receipts. The review
below describes the earlier preparation stage.

These three sealed manifests are review artifacts. No service, gateway, seat or
collector was started, stopped, replaced or reconfigured while preparing them.
They preserve the literal environment, loopback ports, database names, capacity,
CPU settings and model/seat arguments in the existing local profiles. The
`serve.py` source bytes are sealed as provenance; they are not executed.

- `arena.json`: existing private arena host on port 28776.
- `arena-lane2.json`: existing private arena host on port 28777.
- `preview.json`: existing fixed preview on port 28766, K2/raw policy and the
  existing encoder7 relational-v2 model. All three fixed model artifact hashes
  were compared with the original supervisor's literal pins. No model promotion
  or reload is part of this replacement.

`serve_local4 --manifest PATH --check` verifies inputs without starting anything.
It does not acquire the live controller lock or write runtime state. A normal
invocation requires the existing controller lock and an unoccupied loopback
listener; it never stops the existing owner to obtain either. The native process
uses one isolated process group per direct child, pins its leader by retaining
its wait relationship, handles SIGTERM/SIGINT, and empties owned groups before
reaping their leaders. Child failure does not trigger a restart. Continuous
health checks retain the preview's two-second/five-failure policy; arena hosts
retain their one-second child-liveness policy.

The agent token is generated from the operating system RNG in memory and passed
only to the gateway and agent environments. Receipts contain identities and
input seals, never the token. Seat and collection children do not receive it.
The preview's collection-only stage uses `collect_local4`: the existing official
anonymized export, a 120-second export bound, native dedup import into the same
train-only corpus, and a 300-second interval. It performs no preview selection,
model publication or training. The debug executable is sealed at its review
location; a deployment must copy a reviewed immutable executable and reseal that
literal destination. A changed executable causes verification to fail.

The active user gateway (including PID 9331/port 28766) remains untouched.
Installing or changing units is a separate root-owned step after reviewing the
current service identities and these concrete manifests. Historical files and
completed artifacts stay available.

## Collector lifecycle interface

`collector_ops4 check --root ROOT` replaces the quiet collector monitor. It
preserves the `monitor/state.json` delivered/retry ledger, existing event IDs,
attempt floors and sealed false-quota corrections. The pinned historical wake
helper is checked but never imported or executed. Native notification delivery
uses the same bounded `codex queue` command. No check was invoked on a deployed
root during implementation.

`collector_ops4 verify-probe --config PATH` performs strict one-game Flash probe
admission without starting anything: normal duel setup, exact model/input pins,
closed room, full positive LLM transcripts without fallback, official record,
executed provenance and exact replay through the final input are all required.
A running/missing-status probe returns pending; a missing probe root is refused.

`collector_ops4 probe-preflight PLAN` preserves the reset plan's exact floating
point `not_before_unix`, fresh namespace checks and pinned input checks. The
launch command also applies this gate when its plan contains a quota deadline,
so omitting an external preflight does not bypass the hold.

`collector_ops4 finish --ops OPS` and `resume-lane9 --ops OPS` preserve the old
terminal receipts. Existing `done`/`failed` receipts return without reading a
config or querying/starting a unit. A new nonterminal native handover additionally
needs an immutable `collector-activation-plan.json` with `schema_version: 1`, the
exact `unit`, the raw config file's `config_sha256` and an explicit
`not_before_unix` at least as late as the original hold. Strict probe admission
is required for both handovers. No active activation plan was written. In
particular, old October 28 windows cannot implicitly authorize the current
November 1 reset plan. Durable intent precedes DRAIN removal/start; recovery
binds to the exact config and observed new PID birth identity, and never starts a
second controller when one is already present. Failed activation restores DRAIN
and records a terminal failure rather than retrying.

`collector_ops4 launch PLAN` is the memory-only credential bridge for the primary
collector. It verifies input seals, the designated active private gateway unit,
its child UID/parent/executable/birth identity and endpoint configuration before
reading the private token into memory and replacing itself with the collector.
Errors remain opaque. It does not start or mutate the gateway.

Historical timestamp integrity is covered by the `serde_json/float_roundtrip`
feature and repeated read/write regression cases taken from the deployed
ledgers. `collector_ops4 repair-ledger --state PATH --backup PATH --lock PATH
--out RECEIPT` audits a comparison under the existing monitor lock; `--apply`
restores only timestamp fields proven to differ by one floating-point ULP.
Missing delivery IDs, changed receipt payloads or larger differences are refused.
New delivery entries and all other current state are preserved. This command
never queues notifications or starts a controller.
