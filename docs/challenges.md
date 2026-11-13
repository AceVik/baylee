# Daily and Weekly challenges

Central owns the UTC calendar, attempt reservations, public rankings and
permanent badges. Every gateway points players at that same authority. Only
the separately provisioned official bridge may publish, claim or score a task;
a registered gateway credential is insufficient.

## Private packages and certificates

`baylee-challenges` carries the bounded package contract without an engine
dependency. Its preset has exactly one human and one frozen, view-only house
opponent. Shuffle and policy use separate seeds; a new game ID, interface
setting or host restart cannot reroll the opponent. The package must match the
executor build, schema and public target turn. A public room cannot set the
private challenge policy flag through the ordinary setup protocol.

`baylee-train::challenges` generates candidates from the tested working-card
pool. Daily candidates currently use the Crossroads tactical template;
Weekly candidates use the existing synergistic deck builder for a complete
duel. This first template is a foundation for difficulty and variety testing,
not a claim that one recurring position provides sufficient long-term variety.

Admission requires at least three distinct contribution signatures and both
combat and direct-damage finishes. The classifier reads historical damage
source snapshots and the actual life-loss defeat. Card-copy identities, mana
order, repeated attacks, overkill and the order of the final spell/attack do
not create another resource signature. Tokens, copied sources and unsupported
victory causes fail closed. This proves several concrete winning executions,
not optimal play, exhaustive solution coverage or human difficulty.

Each witness is checked on a fresh session with a new game ID and a freshly
initialized opponent, through exact record replay, and through policy-checked
record resumption. Answer/turn limits and unsupported-choice fallbacks cannot
produce a certificate. Raw witnesses and future packages remain private.

## Producing and retaining stock

Run the Rust worker with the same tested build as the official executor:

```text
cargo run -p baylee-train --bin challenge -- generate --kind weekly --at 1794528000 --seed 49 --candidates 8 --out target/challenge-evidence
cargo run -p baylee-train --bin challenge -- stock --central-dev http://127.0.0.1:28790 --state-dir target/challenge-stock --days 7
```

The first command is offline. The second requires the synthetic fixture's
bridge capability in `BAYLEE_CHALLENGE_BRIDGE_TOKEN`. Production requires the
explicit `--live` endpoint selection and a separately provisioned credential;
neither credentials nor their values are accepted as CLI arguments.

Schedule `stock` periodically with the deployment's ordinary service manager.
One pass reads Central's occupied slots, keeps withdrawn slots occupied, and
fills only missing current/upcoming periods. It covers the current Weekly even
on Sunday and admits a horizon of at most 24 days, within Central's 32-day
inventory bound. Exhausting the candidate budget is an error; there is no
uncertified fallback. Monitor failures and the remaining published horizon.

Before publication the worker writes and synchronizes every witness and the
immutable publication in its private state directory. A no-overwrite hard link
exposes the completed slot file only afterwards. Restart and ambiguous network
responses reuse this evidence, then re-read Central's inventory. Concurrent
workers cannot replace an already staged slot; Central independently enforces
one immutable publication per period. Unix evidence directories are `0700` and
files `0600`. Keep this directory with the executor's audit data.

A staged task from another executor build is refused. Upgrade planning must
retain the executors needed by already published tasks; silently regenerating
an occupied period would break fairness.

## Official execution and delivery

`POST /challenges/{attempt}/play` requires the official bridge and a verified
Central account session. Only an attempt identity is accepted. The host keeps
the proposed game identity before claiming it and consumes a durable launch
permission before revealing the opening. A retry resumes its existing seat;
an engine lost with a restart is never restarted from the beginning under the
same attempt. Public rematch cannot bypass Central's attempt accounting.

The engine must identify the exact certified build before receiving the
private preset. Challenges disable spectators. Human time uses the process's
monotonic clock and counts waiting for the human, including network latency;
loading, AI work and outgoing batch transmission do not count. A disconnect
permanently excludes that attempt from Sprint but leaves its game resumable.
The ordinary 24-hour disconnected hold ends a challenge as a technical void,
not a defeat. Deliberately leaving the table abandons the attempt.

The authenticated engine sends the outcome, finish timestamp and SHA-256 of
its uncompressed record. The gateway queues that outcome after the contiguous
final record chunk and the recorded ending. A failed write stops the writer;
it cannot acknowledge a successful flush or store a score. PostgreSQL retains
an immutable result until Central confirms delivery. Retries reuse the same
timestamp, measured duration and record hash. Account deletion cascades through
local delivery metadata, and delivered rows are removed.

An owned background worker reconciles at most 32 pending rows every ten seconds.
Confirmed remote abandonment stops its game. A claimed game absent from the
host after a two-minute recovery grace period becomes void without a failure
penalty. This is crash recovery by cancellation; it is not persistent game
checkpoint resumption across a host restart.

## Verification boundaries

The generator tests execute real Daily and Weekly games, fresh opponents and
replays. Calendar tests simulate an accepted publication whose reply is lost,
worker retry, duplicate reconciliation and UTC rollover. Staging tests restart
with a zero generation budget and verify all retained witness hashes.

Central separately tests account binding, authority, idempotent scoring,
leaderboards, assistance, withdrawal and permanent progress on PostgreSQL.
Those checks do not replace the final network/client integration run or blind
human difficulty calibration. Record those results only after they run.
