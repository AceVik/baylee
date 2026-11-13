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
pool. Daily candidates rotate four Crossroads resource positions: different
life totals, blockers, attackers and available spells change the necessary
damage/removal combinations. The calendar rotates the template while 62 seed
bits stay private and random. Weekly candidates use the existing synergistic
deck builder for a complete duel, anchored around a direct-damage tool.
Four tactical templates are the initial content set, not unlimited variety.
Weekly decks exclude explosive mana staples (including the original Moxen,
Black Lotus, Mana Crypt, Sol Ring and Mana Vault) on both sides. A candidate
with a probed victory before game turn nine is rejected. The probe tries both
ordinary house play and reserving Lightning Bolt for lethal; these are visible
seat strategies, not access to the opponent's hand or library. The lower bound
is on observed proofs, not a claim that no stronger human can win earlier.

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
cargo run -p baylee-train --bin challenge -- generate --kind weekly --at 1794528000 --seed 63 --candidates 8 --out target/challenge-evidence
cargo run -p baylee-train --bin challenge -- stock --central-dev http://127.0.0.1:28790 --state-dir target/challenge-stock --days 7
```

Both commands accept `--source-dir` for the matching source checkout containing
the working-card tests. A stock pass reads that evidence once and reuses it for
all candidates. Live publication requires this option explicitly, a clean release
binary with a known commit, and a clean checkout at that same commit. Keep this
checkout read-only in the worker deployment; it is not an incidental build path.

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

`scripts/server/baylee-challenges.service` and `.timer` run the Rust worker
hourly with a seven-day horizon and 32 candidates per missing slot. Provision
the dedicated `baylee` account, install the clean release `challenge` executable
and its matching read-only source at `/opt/baylee/challenges/current/`, and
supply `BAYLEE_CHALLENGE_BRIDGE_TOKEN` to the service manager through the
operator's secret manager. The unit contains no credential. Its persistent
state is owner-only `/var/lib/baylee-challenges`; the unit is read-only elsewhere.
Build the executable with `cargo build --release --locked -p baylee-train --bin challenge`.
The units are templates for operator installation; they are not enabled by a
development test or automatically deployed by this change.

`challenge assess --kind weekly --seed 1000 --count 32` measures admission
without publishing or printing proof records. The first improvement pass on
13 November 2026 admitted 4/32 candidates in 291.7 seconds, with certified
winning turns 11–19. This finite sample is not an availability guarantee or
human difficulty calibration. A real publisher run before the anchor change
exhausted 16 random candidates and correctly published no uncertified Weekly.
Keep advance stock and alert on a failed timer or a shrinking horizon.

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

Run a dedicated agent for each retained engine binary, setting `BAYLEE_ENGINE_BIN`
to its fixed operator-installed path and `BAYLEE_ENGINE_BUILD` to its exact build
identity. The gateway obtains the required identity from Central before claiming
an attempt and selects only an authenticated matching agent. Missing capacity
leaves the attempt reserved with no start counted. The attached engine is checked
again before receiving the private preset. No player selects a binary or path.
Ordinary games use current-build or legacy agents, never retained builds.
This supports retained executors with the same wire and preset schema; a protocol
upgrade needs a compatible retained execution host before those tasks can run.

## Official execution and delivery

`POST /challenges/{attempt}/play` requires the official bridge and a verified
Central account session. Only an attempt identity is accepted. The host keeps
the proposed game identity before claiming it and consumes a durable launch
permission before revealing the opening. A retry resumes its existing seat;
an engine lost with a restart is never restarted from the beginning under the
same attempt. Public rematch cannot bypass Central's attempt accounting.
Delivery reconciliation takes the same admission lock per row, reloads its durable
binding, and keeps the lock through the remote status decision. A delayed Claim
cannot race with recovery deleting or voiding its original game identity.

The engine must identify the exact certified build before receiving the
private preset. Challenges disable spectators. Human time uses the process's
monotonic clock and counts waiting for the human, including network latency;
loading, AI work and outgoing batch transmission do not count. A disconnect
permanently excludes that attempt from Sprint but leaves its game resumable.
The ordinary 24-hour disconnected hold abandons the attempt, as does deliberately
leaving the table, even after its socket has already disconnected. Both count as
failed attempts; a player cannot erase a losing Weekly by waiting out the hold.
Confirmed technical host failures remain void without a failure penalty.

The authenticated engine sends the outcome, finish timestamp and SHA-256 of
its uncompressed record. The gateway queues that outcome after the contiguous
final record chunk and the recorded ending. A failed write stops the writer;
it cannot acknowledge a successful flush or store a score. PostgreSQL retains
an immutable result until Central confirms delivery. Retries reuse the same
timestamp, measured duration and record hash. Account deletion cascades through
local delivery metadata, and delivered rows are removed.

An owned background worker reconciles at most 32 pending rows every ten seconds.
It holds a PostgreSQL session advisory lock on its own connection, outside the
request pool, scoped to database and schema. Only its owner admits or reconciles
challenges; a second gateway
fails closed instead of voiding games absent from its own memory. A lost lock
connection closes admission and stops owned games. Queue rotation is separate
from reservation age, so a batch of young reservations cannot starve results.
Confirmed remote abandonment stops its game. A claimed game absent from the
host after a two-minute recovery grace period becomes void without a failure
penalty. This is crash recovery by cancellation; it is not persistent game
checkpoint resumption across a host restart.

Ownership checks run every ten seconds independently of slow result delivery,
with a three-second database deadline; acquisition has a five-second deadline.
Losing that connection cancels the delivery pass before releasing ownership.

## Verification boundaries

The generator tests execute real Daily and Weekly games, fresh opponents and
replays. Calendar tests simulate an accepted publication whose reply is lost,
worker retry, duplicate reconciliation and UTC rollover. Staging tests restart
with a zero generation budget and verify all retained witness hashes.

Central separately tests account binding, authority, idempotent scoring,
leaderboards, assistance, withdrawal and permanent progress on PostgreSQL.
On 13 November 2026, `cargo dev central integration` also passed the complete
Daily/Weekly path over real Central HTTP, authenticated gateway requests and
engine/seat WebSockets. It checked publication retry, reservation idempotency,
both real victories, a Weekly disconnect/resume in the same attempt, exclusion
of that interrupted result from Sprint, all three ranking responses and permanent
badge progress. Its isolated PostgreSQL 18.6 container and listener were removed.
The scripted seat replayed a retained witness; this is not blind human difficulty
calibration or a rendered-client check.

The first improvement pass also passed all five generator tests (including
all four Daily positions), the publisher tests and all 47 real PostgreSQL
schema tests, including exclusive ownership and release. Clippy passed for
the changed train, database, gateway and development-tool libraries/binaries.

## Second improvement pass: blind play and presentation

The isolated publisher was run twice through real Central HTTP after the first
pass. It generated fresh Daily and Weekly tasks; the repeated pass published
nothing twice. The native client could not be photographed: its prescribed
monitor 1 was absent. It exited, and the owned PostgreSQL container and listener
were removed. Do not substitute the owner's primary monitor.

`challenge play --kind weekly` provides an offline blind check when a test
display is unavailable. It generates a fresh certified candidate internally,
discards its witnesses without displaying them, and prints only the supplied
public deck and the normal human `PlayerView`/pending question. It accepts
`PlayerAction` JSON on standard input; an empty line passes priority, and
`quit` exits. It does not publish or submit a score.

The implementing agent played one such fresh Weekly without reading a proof.
After a free mulligan, Mana Crypt enabled an early Guttersnipe and two Students
of Ojutai. The opponent removed the Crypt with Archdruid's Charm. Lightning
Bolt, Flame Jet, correctly ordered triggers and combat won at game turn seven.
The process exited successfully. This was a meaningful difficulty finding:
the Weekly was too explosive. It led to the mana exclusions and observed-turn
floor above. One agent game is not a representative human study.

Native calendar cards now put the current Daily and Weekly before badge
progress and the archive; old tasks cannot push the current Weekly down the
list. Replay/settings test helpers follow the new navigation. All three native
challenge control tests passed. Central's website refreshes scores every minute,
retains the chosen archive page, pauses refresh during hint confirmation, and
clears hint state when the account session is lost. Its ten desktop/phone tests
passed with both one and five browser workers after rebuilding matching SSR
and island assets. The five generator tests passed with the new fixed Weekly
fixture (seed 63); fresh execution, exact replay and policy reconstruction remain
mandatory. Native screenshots and broader human calibration remain unverified.

## Third improvement pass: compatibility and operational failures

The clean second-pass build `0a5b8605a` admitted 17 of 32 Weekly candidates
starting at seed 1000 in 261.2 seconds. The 53 retained witnesses finished on
game turns 9–49. Every admitted candidate met the independent route/family,
fresh-execution, replay and resume checks; the other 15 were refused. These
are sampled solver results, not an assertion of optimality or human difficulty.

Changes in this pass: exact retained-executor routing, capacity refusal before
claim, independent bounded ownership heartbeats, and continued queue rotation
without renewing the crash-recovery grace. The real HTTP integration caught a
bridge capability incorrectly attached to a public metadata read; that read now
uses anonymous public access, while status/claim keep host authority.

Validation on 13 November: all 49 PostgreSQL schema tests passed, including
termination of the isolated ownership connection and rotation past 32 waiting
reservations; all 19 agent tests, six engine-routing tests and three publisher
tests passed. Publisher tests reject shared/symlink evidence roots and overwrites.
Clippy passed for all targets of the agent, gateway, challenge contract, trainer,
database and development tools. The real Central/gateway/engine Daily and Weekly
integration passed in 110.71 seconds, including unavailable-capacity refusal with
zero consumed starts, resumed Weekly play, verified scores and permanent awards.
The owned database and listener were removed. The isolated full-gate wrapper's
deliberate failure also removed its owned container. The three improvement rounds
are complete; required full gates and independent PR review are recorded separately.

## Final verification and second blind game

The clean third-pass publisher was exercised against real Central HTTP again:
fresh Daily/Weekly publication and a repeated pass with zero duplicate publications
passed in 128.30 seconds. The fixture removed its owned database and listener.

The implementing agent also played a second freshly generated Weekly without
reading its private proofs. After a free mulligan it kept two lands, developed
Guttersnipe and Sawblade Scamp, and saved Flame Jet for their triggers. It avoided
trading into Mai's first strike. When the opponent kept two blockers back, an oil
counter activation followed by Fever Charm and Guttersnipe's trigger won on turn
eleven. Damage, trigger order and the terminal victory were observed in ordinary
player views, and the CLI exited. This exercises the slower pool and a different
decision sequence; two agent games still do not establish population difficulty.

The first complete gate exposed pool starvation in two existing sign-in tests:
the ownership transaction occupied one of the fixture's two request connections.
Ownership now uses a separate connection with the same TLS and schema options.
A new regression requires ordinary queries to work with a one-connection request
pool while ownership remains exclusive. All 50 PostgreSQL schema tests and all
three Central-barrier tests passed after the fix, as did clippy for all targets
of the database, gateway and development-tool crates. A further full run exposed
an unrelated feedback test upload race: the server could reject Content-Length
before its test client finished writing. The extracted test fixture now waits for
HTTP 100-continue or the early refusal. Server bounds and assertions are unchanged.

The final full gate passed all 27,802 executed tests (61 intentionally skipped),
workspace clippy, formatting, bench smoke, cached-card validation and API checks.
The test phase took 899 seconds including its build; execution took 884.98 seconds
on the shared workstation. Its owned PostgreSQL container/listener were removed.
The development-feature matrix passed its lint, gateway `dev-table`, support,
simulation, fork, mutate and other applicable feature steps, but its aggregate
client test invocation failed. The runner retained only the final build-output
tail, so the failing test names were unavailable. This result remains unresolved;
it is not reported as a green matrix. The separately reported SDK revision guard
failure was an obsolete expected pin: after an independent source review,
updating that exact pin passed all five guard tests and the shipped/unified-hook
and complete dependency-graph checks (see [Central login](central-login.md)).

Following the owner's request to test only changed areas, no further broad run
was started. Focused client checks with `dev-control,dev-reload` passed all 538
executed lobby tests (two intentionally ignored) and all 43 Dev-Control tests.
These validate the affected client paths but do not retroactively clear the
unexplained aggregate failure. All owned PostgreSQL fixtures were removed.
Independent PR review and its targeted follow-up checks are recorded below.

A fresh Daily was also played blind with the ordinary player view. With two
2/2 Bears against one blocker and five opposing life, both attacked. After the
opponent blocked one, Giant Growth on the unblocked Bear dealt lethal damage
on turn one. Lightning Bolt, Shock and Fling were unused. The CLI reported victory
and exited; this confirms an accessible combat solution alongside the certified
direct-damage routes. Four initial resource positions are a bounded starter set,
not a claim of inexhaustible Daily content.

## Independent review corrections

After all three improvement rounds, independent reviewers inspected the generator,
host/delivery path and native client. Seven actionable findings were corrected:

- Frozen opponent decisions now use only accepted opponent answers as their noise
  sequence, so manual versus automatic human priority passing cannot reroll them.
  Human views keep their advancing sequence for solver loop detection. Live play,
  record replay and process resumption reconstruct the same opponent counter.
- Crash reconciliation shares admission's lock and rereads the queued binding
  under it; a slow in-flight claim cannot be removed from an earlier snapshot.
- Explicit abandonment is honoured even before attaching or after a disconnect.
  Expiry of the 24-hour player absence is abandonment too; genuine host failures
  remain void. Waiting out a Weekly cannot restore first-try eligibility.
- Native challenge controls have semantic keyboard stops and a separate
  confirmation focus order. Website links wait for refreshes and confirmations.
- The ordinary authenticated seat response carries the optional public challenge
  identity, restoring the result navigation on client recovery. Stale gateway
  replies cannot replace the current account's context.

All three reviewers inspected the corrections and reported no remaining concrete
findings in those scopes. Their reviews were static; execution evidence is recorded
separately. Central's independent authority/web reviews and the two corrected web
findings are recorded in its companion PR.

Following the owner's test-scope constraint, the corrections were checked without
another full gate: all six native challenge tests, three engine lifecycle tests
and five gamehost challenge tests passed. The settings regression's old assertion
on the human sequence was corrected to compare the opponent sequence; its isolated
rerun passed. The separate automatic/manual-pass regression exercises real F6
inputs and record resumption after every accepted answer. Clippy passed all targets
of the seven changed crates, with the native development features enabled.

`cargo dev central integration --challenges-only` passed in 55.20 seconds against
real PostgreSQL 18.6, Central HTTP and gateway/engine sockets. It freshly certified
both kinds, published twice without duplicates, exercised the slow-claim recovery
race, checked challenge context through the ordinary seat route, won both games,
resumed the Weekly within the same attempt, and checked turns, failures, Sprint
exclusion, all three leaderboards and permanent badges. The owned database,
listener and services were removed. This targeted result does not change the
earlier aggregate feature-matrix limitation above.

Formatting of the changed crates, the affected gamehost library's `simulation`
feature lint and the regenerated API-document check also passed. No rendered
native client was started for these final checks.
