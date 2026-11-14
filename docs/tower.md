# The Verdant Tower

The expedition is an original five-duel weekly campaign. The player plays each
duel and chooses one of three deck improvements between floors. It is not an
autobattler. Central owns the week, progress, three lives, reversible ranking
consent and permanent rewards across every gateway. The full authority contract
and migration are in the sibling Central repository's `docs/tower.md`.

## A complete, certified graph

Three upgrade tracks commute. Four choices therefore produce 35 distinct deck
states across five floors and 81 chronological routes. The publisher uses the
existing synergistic deck builder with a bounded, working-card-verified pool.
Each floor fixes an opponent deck, policy and game seed. Every upgrade state
must independently pass the same certificate: three distinct contributing
solutions, at least two terminal families, fresh execution, exact record replay
and opponent-policy reconstruction. Witnesses are not optimality proofs or a
substitute for human difficulty calibration.

The starting 60-card deck has 24 lands and readable red/green creatures, removal
and combat tricks. The seven supplied opening cards are removed from the library,
not added to it. Upgrades replace exactly two existing copies, stay within copy
limits and have working evidence. The three paths trade early pressure, damage
spells and persistent creatures; their exact mana costs and timing still matter.

Search is bounded to 32 opponent candidates per floor. The first canonical node
must pass before searching neighbouring variants. Each neighbour permits up to
eight deterministic permutations of its human library. This never changes its
card multiset, supplied hand, enemy preset or engine RNG consumption. The accepted
permutation is frozen in the private encounter package: live retries cannot
reroll it. The current strategy list is bounded to 192 human-policy probes per
deal. Any exhausted floor refuses the whole tower.
No partial graph becomes playable and no unsolved fallback is published.

The current solver alternates ordinary house policies, direct-damage pressure,
combat finishing and reserving one damage spell. These use ordinary player views
and legal choices. Contribution signatures ignore harmless action order, copied
objects and overkill. Evidence still requires real outcomes, regardless of the
solver's intended style.

## Publishing and recovery

`challenge tower --at <unix-seconds> --seed <private-seed> --candidates 32 --out <new-private-directory>`
certifies a whole graph and retains every publication and proof offline.
`challenge tower-assess --seed <private-seed> --node <0..34> --candidate <1..32>`
checks one exact node using the same bounded search, without publication.

`challenge stock --activities all` stocks Daily, Weekly and expedition slots;
`calendar` and `tower` limit it explicitly. A complete `Prepared` bundle is
validated before any node is sent. Each node is staged against the immutable
manifest, and Central activates only after all 35 digests agree. Retrying an
accepted upload whose response was lost is safe. Existing published or withdrawn
weeks are not regenerated. Private evidence uses restricted directories/files
and remains outside the web root and ordinary logs.

Every duel uses the existing official-host admission, exact retained executor,
owned seat, monotonic human timer, immutable result and durable outbox. No client
supplies an outcome or chooses an encounter seed. A disconnect removes Sprint
eligibility but permits resumption. A verified loss spends one life; a technical
void does not. `Locate { game }` restores the original owned expedition after
client recovery, including completed games and previous weeks.

Ascent compares cleared floors, then fewer losses, then winning turns. Sprint
requires all five floors without a loss or interrupted timing. Only the best run
per account is listed. Public naming is off until explicitly chosen. Older runs
can continue in practice after UTC rollover without changing their frozen ranked
prefix. Distinct-floor, distinct-summit and flawless-summit badges cannot be
farmed by repeating an easy floor. No daily streak can erase permanent progress.

## Native and website experience

The native Lobby places a compact expedition entrance with Daily and Weekly.
The expedition has a five-floor route, lives, a primary next action, three equal
upgrade previews with explicit confirmation, independent rankings and three
original relic motifs. Each relic has five increasingly ornate shader-rendered
tiers. Motion reduction keeps a static material and no idle CPU animation loop.
Website links use `baylee://tower/<32-lowercase-UUIDv7-hex>` and open details only.
The Central website presents the same public week and separate account progress.

Account switching discards private state and pending responses. Ambiguous
Begin/Start/Choose responses keep the exact request identity. Choosing an upgrade
does not start the next game. Withdrawal, unavailable capacity, pending result
delivery and exhausted lives retain explicit states rather than inventing a win.

## Focused local evidence

The native presentation received five implementation refinements before review:
the playable route and actions; balanced overview panels and centered heading;
three exact replacement previews with selected-state controls; aligned rankings,
life pips and route markers; then earned summit/relic presentation and compact
layout helpers. A sixth correction pass preserved uncertain upgrade retries and
labeled the actual ranking week. These first six passes were implementation and
headless refinements. The owner's subsequent permission to use a passive window
on the main display enabled the rendered acceptance described below.
The preceding Daily/Weekly Lobby redesign had six rendered/control refinements,
including compact gateway presentation, five distinct animated badge motifs and
five inspectable tiers, separate play/ranking/rules views and reduced motion.

The synthetic demo fixture adds three account scenarios when
`BAYLEE_CENTRAL_DEMO=1` and `BAYLEE_CENTRAL_UI_READY` name its readiness file:
`demo_player` has a completed first Tower floor awaiting an upgrade,
`fresh_player` has a completed five-floor expedition and its earned rewards,
and `demo_resume` has a resumable Weekly duel. Supplying the current week's
`BAYLEE_CENTRAL_TOWER_EVIDENCE` enables the Tower scenarios. The bounded hold
ends on its deadline or the readiness path's `.stop` sibling; no production
account, credential or authorization shortcut is involved.

The demo owner generates a fresh password per run and supplies it only through
`BAYLEE_CENTRAL_DEMO_PASSWORD`; it is never printed or written to readiness files.
Ordinary integration accounts use independent operating-system randomness.

The route/deck tests enumerate all 35 states and every upgrade edge, checking
legal working decks and exact replacements. A separate invariant test compares
all eight human deals across every node against the opponent's initial library,
hand and engine RNG position. The latest focused deal invariant passed in 0.89 s.
The previously failing node 16, candidate 9, seed 41 certified four independent
witnesses on turns 17, 15, 17 and 19 after the bounded deal refinement.

`BAYLEE_CENTRAL_TOWER_EVIDENCE=<complete-private-directory>` enables the Tower
extension of `cargo dev central integration --challenges-only`. It uses a current
week's complete certified bundle, simulates a lost staging reply, verifies that
an incomplete graph stays invisible, resumes the actual `challenge stock` CLI,
verifies a second invocation publishes nothing, then plays five actual
networked duels with retained proofs. It checks upgrade retries, owned recovery,
scores, permanent rewards and Sprint exclusion after disconnect/resume. This is
transport verification, not blind human play. Without this opt-in, ordinary
Daily/Weekly integration stays small. The Rust owner removes its own services
and database on completion or failure.

Build the matching `challenge` executable first. The launcher resolves
`target/debug/challenge` (with the platform executable suffix) before starting
PostgreSQL; `BAYLEE_CHALLENGE_BINARY` may explicitly select another matching
build. It passes that absolute path into the fixture. No installed/global
publisher is used implicitly.

The complete seed-41 graph certified all 35 nodes, with at least three contributing
wins in at least two terminal families per node. All retained witnesses passed
fresh execution, exact replay and opponent-policy resumption; observed turn
counts range from 9 to 33. This is not representative human difficulty evidence.
The real network fixture passed in 91.46 seconds: Daily/Weekly plus five Tower
victories, four deck choices, retry recovery, durable rewards, and disconnect /
resume exclusion from Sprint. It exercised the actual stock executable twice
after an interrupted staging response: first completing publication, then doing
no duplicate work. Its first attempt found stale copied runtime grants in the
Baylee fixture; those were aligned with Central migration 000029 before the pass.
The owned services, PostgreSQL container and listener were removed.

Eight final focused native/core regressions passed (0.80/0.01 seconds), including
uncertain upgrade retries after cancelling a local preview and explicit ranking
week labels alongside an older active expedition. Independent generator, host
and client reviewers reported no remaining concrete findings after corrections.
The later native inspection complements these headless checks; neither the
inspection nor the retained witnesses establish representative human difficulty.

The final default gate ran 28,041 tests: 28,035 passed and six Lobby expectations
failed. Four still expected the removed gateway panels; two exposed challenge
refreshes during the Central terms/migration transition. The transition now
blocks both explicit and automatic challenge requests until it completes, and
connection details are tested in their compact popover. All 58 affected Lobby
tests then passed in 5.013 seconds. The independent client reviewer found no
remaining issue in those corrections. The full suite was not repeated.

The same gate passed workspace clippy, bench smoke and API checks. Its printing
validation found one newly refreshed equipment reminder text; `refresh-oracle`
updated only that derived comment, and validation then passed for all 12,568
cards. No card implementation changed.

The final `cargo dev central gate --features` passed: client feature clippy and
2,737 client tests (43 dev-control tests), gateway dev-table clippy/tests, all
applicable desktop feature configurations, shipped/unified hook boundaries and
the exact reviewed Central crypto source/reverse graph. The Linux-only replay
collector check is not applicable to this macOS run. The gate removed its owned
PostgreSQL 18.6 container; its processes exited and listener closure was verified.

### Rendered native acceptance, 2026-11-14

The owner explicitly permitted the main display because no additional monitor
was available. A passive `dev-control` window used only virtual input, synthetic
accounts and isolated local services. Two rendered correction passes brought
the upgrade decision above route/history/consent, reduced the hero's padding,
replaced a missing text-font checkmark with the shipped icon-font glyph, and
moved explicit confirmation above the three cards. The final layout was
inspected at 1440 × 900 and 1280 × 720 logical pixels (2× display scale).

At 1280 × 720, all three equal replacement cards and confirmation fit together
after scrolling to the decision. Selecting Precision, cancelling that preview,
selecting again and confirming through the native UI produced Central's ready
state for floor two, with the committed Precision path and three lives. The
completed synthetic account showed all five checked stages, four chosen paths,
the matching ranking and its three earned Bronze relics. Its five-tier Summit
gallery retained its ornament and labels in reduced-motion mode.

Animation evidence is deliberately bounded. The initial three-second, 8 Hz
recording retained six frames and reported 18 backpressure drops. A subsequent
five-second, 1 Hz recording retained all five samples without drops; checksums
of the medal-only region differ across them. Three paused control frames were
byte-identical. After enabling reduced motion through Settings, three further
frames were byte-identical while virtual time continued, with all five tiers
still visible. Selected raw frames were inspected. These finite samples verify
motion and its suppression, not absence of all possible flicker or performance
under production load. Raw frames, ledgers and previews are retained locally in
the task's `2026/11/14/baylee-tower-native` visualization artifacts.

The final targeted Tower control test passed in 0.544 seconds, including both
client languages, and client all-target `dev-control` clippy passed. Independent
static review found no remaining issue in the layout corrections. CodeQL's
synthetic hard-coded-password finding was corrected using per-process OS
randomness and a per-run, environment-only demo password; the normal Central
integration, including personal re-login, passed in 108.44 seconds. The held
Tower demo also passed on release. Every owned client, backend and PostgreSQL
container exited, and owned listener closure was verified. No full gate or
35-node generation was repeated for these focused corrections.
