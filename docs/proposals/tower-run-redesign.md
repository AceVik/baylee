# Tower run redesign — owner acceptance contract

15 November 2026. This is the implementation plan after the owner's native playtest.
It supersedes the presentation and new-run rules in the earlier Echoes proposals;
saved generations keep their original contracts. Checkboxes below are not evidence.

## Integration baseline

- Baylee `codex/tower-echoes` fast-forwarded from `c83179f3c` to Gemini's
  `agy/tutorial-videos` `7d8878e40119f48b774755c8b9d47817f7560292`.
- Central `agy/tower-achievements` and `codex/tower-achievements` both resolve
  to `bbb45df50eb4ef4cec5aceb494251dce2ccd9d66`; no difference to merge.
- Initial risk-focused review inspected native video buffer ownership/eviction,
  decoder guards, dependency changes, account visibility, tooltip lifetime and
  the touched Tower adapters. This is not a claim of a full independent audit.
- `cargo test --locked -p baylee-avif --lib`: 20 passed; 15.47 s build,
  0.01 s tests. No broad workspace test was run for this integration.

## Owner decisions

- Tower is one self-contained run. New runs reset deck acquisition, items,
  potions and gold. Permanent account rewards are cosmetic/history only.
- The authenticated online gateway lobby keeps the table list on the right.
  Replace the old left play/deck/connection hero with Tower, Daily and Weekly
  tiles. Remove the Tasks/Aufgaben navigation tab. Keep create/join controls
  reachable beside the table list. Offline/guest entry remains functional.
- At least five small, interesting Commander-like starter choices; deck
  selection leads directly to the first shop, before the first duel.
- Six active item slots on the left, potions underneath, the large shop shelf
  on the right, paginated sortable/groupable inventory below. Items are icons
  in rarity frames, not card-shaped panels. Hover/focus opens a reusable detail
  sheet with icon, abilities and exact upgrade preview.
- Buying the third identical family/rarity automatically fuses all three.
  Equipped and stored copies both count. The result replaces the ingredient
  in the lowest occupied active slot, if any; otherwise it stays in inventory.
  Repeat a resulting ordinary fusion if it completes another triple.
  Rare + one Mythic potion -> Mythic Rare remains a deliberate targeted craft.
- Every rarity strengthens the base effect and adds an ability; Mythic adds a
  distinctive legendary ability. Six slots are the build constraint.
- **Owner explicitly chose independent run equipment:** items are not ordinary
  artifacts and normal removal/exile cannot remove them. Costs, priorities,
  targets, triggers, replacement effects and the stack still use MTG machinery.
- The duel has a compact equipment drawer: own items and usable potions,
  opponent public items but no opponent potion inventory. Reuse shop visuals.
- Verified game end -> five random nonbasic card candidates from the frozen
  opponent deck -> select three on victory, one on defeat -> next shop.
- At the next shop all offers reroll, except frozen unsold stock. Purchased
  empty positions refill. A persistent Freeze shop switch preserves the whole
  unsold offer set until unfrozen, including its identities and prices.
  Paid rerolls respect freeze too; a fully frozen/full shelf cannot charge gold.
- Original modular shelf artwork and animated ornament, scalable in width and
  number of rows. Reduced motion/low ambient quality keep an attractive still.

## Concrete defaults and design recommendations

These resolve unspecified details and can be changed without silently rewriting
saved runs. They are design choices, not claims of measured balance.

- Exactly 39 library cards + one commander; at most five sideboard cards.
  Singleton nonbasics across all three lists; normal commander color identity,
  command-zone return and tax. Deck validator and engine preset agree.
  Commander-related combat loss uses the existing engine semantics: 21 combat
  damage from the same commander, tracked separately, including custom-format
  presets with commanders. Command-zone return choices and the additional two
  generic mana per previous command-zone cast remain the ordinary engine rules.
- Infinite free supply of all five standard basics; only run-owned other cards.
  One unified builder uses a restricted Tower pool and Tower save adapter.
  Switching commander requires an owned eligible card and a fully legal edit.
- Initial supported archetypes: Emmara (green/white tokens), Elas il-Kor
  (white/black life and sacrifice), Talrand (blue spells and tempo), Slimefoot
  (black/green Saprolings), Pia and Kiran Nalaar (red artifacts). Each contains
  22 nonbasic spells, 17 basics and its commander. Native support determines
  this opening pool; it is not yet a measured balance conclusion.
- Starting gold 18 and base duel income 12 are calibration starting points.
  Keep 8 wins / 5 defeats as the initial finite run contract. A rules draw
  receives the loss-sized choice, with explicit wording; technical voids grant
  nothing and consume no progression.
- Rarity colors follow the familiar silver Common, blue Uncommon, gold Rare,
  orange/red Mythic identity. Shape/ornament and labels also distinguish tiers.
- Gold bonuses derive only from replay-verified facts and frozen equipped items.
  A proposed vitality reward refers to the duel's derived *starting life*,
  not an invented MTG maximum life. Cap its gold payout, not life gain or legal
  combos. Infinite combos never mint infinite currency.
- Synergy tags, exact fusion previews, and compare-to-deck on reward choices
  make decisions understandable. No extra resource or account power grind.
- Do not add an always-on audio/video loop to the shop; the frame shader,
  gentle material shimmer and brief purchase/fusion cue provide atmosphere.

## State and authority

Tower service in Baylee remains the sole run authority; gateways host matches.
Central receives only permanent achievement facts and owns Daily/Weekly as before.
Every mutation keeps authentication, account/run ownership, expected revision,
bounded decoding, row lock and durable request idempotency. A request carries
identities/choices, never a price, gold amount, rarity result or claimed victory.

New state machine:

```
Choose starter -> Shop(visit 1) -> Reserved -> Playing -> Verifying
                                   ^                     |
                                   |                  RewardChoice
                                   |                     |
                                   +-- Shop(next visit) <+
                                                         `-> Finished
```

Rewards are created once from the actual frozen opponent deck at verified
settlement, with a private deterministic stream distinct from the shop stream.
Sample five distinct eligible identities from the main deck and commander without replacement, excluding
basics, sideboard, conjured/copy objects and unrelated cards. If fewer than five
exist, offer only those available and require min(choice count, candidate count).
The UI labels color-incompatible/already-owned cards; it does not forge a useful
replacement. An already-owned singleton choice grants one gold, accounted for
in the same atomic claim and ledger. Repeated requests/reconnects cannot resample or claim twice.
On the terminal duel the choice can complete into the run summary; there is no
fake playable next stage. A pending choice can be left and resumed; shop actions
and abandonment remain unavailable until the claim is committed.

Do not generate the next shop in the current `Run::settled` branch immediately.
Persist the pending reward first. Claiming it and opening the next visit are one
transaction; preserve the previous frozen offers through the duel and choice.
Render technical verification/recovery separately from defeat.

## Inventory and economy correctness

- Eliminate the six-reserve-slot gameplay limit. Store monotonically allocated
  item instance IDs, six optional equipped references, and an ordered inventory;
  display only one page. Do not allocate a vector of empty slots.
- Pagination uses revision-bound cursors and a stable tie-breaker. Sorting or
  grouping never changes equip position, fusion identity or server order.
  A finite run, request/byte limits and affordable acquisition bound actual
  resource use without imposing a visible slot purchase mechanic.
- Server computes automatic fusion from the purchased definition and current
  inventory. Canonical ingredient order is equipped slot first, then instance
  ID. Preserve summed actual paid basis and deterministic output ID allocation.
  Ordinary fuse tiers strictly increase, bounding cascade work.
- Purchase/debit/fusion/equip/receipt commit atomically. No temporary full-bag
  rejection, duplicate ingredient consumption or half-fused state.
- Combined attribute limits must be reconsidered for six slots. An automatic
  upgrade must not unexpectedly fail because the old three-slot cap assumed
  smaller hand/mulligan bonuses; no silent effect truncation.
- Drag/drop onto occupied equipment swaps atomically. Dropping into inventory
  unequips. Right-click equips to the first free active slot, or unequips if
  already active. Full active slots give a clear refusal, not an arbitrary swap.
  Escape cancels drag; keyboard/focus alternatives perform identical commands.
- Freeze is stored in the run, never in local preferences. On automatic next
  visit, keep each frozen unsold slot exactly and fill bought slots. Unfrozen
  visits refresh all slots. A second entry into the same visit does nothing.
  Define offer replacement by slot/stage/reroll counter, not mutable iteration.
  Duplicate item offers are allowed and have distinct offer identities.

## Engine, privacy and AI

Use the existing compile-time native-pack boundary and introduce a new saved
contract. Do not redefine old IDs/revisions as new behavior. A normal engine must
still reject Tower setup; Tower stays loop-detection-off. No Lua, network callback,
wall-clock scheduler, remote arbitrary effect command or rule rewrite.

Independent item sources need typed native identity, owner, slot, per-duel
counters/exhaustion and compiled ability lookup. Reuse normal trigger/activation
execution while excluding item sources from ordinary permanent target selection,
artifact counts, ETB/death triggers, sacrifice, copy and zone movement. A generic
emblem substitution alone is insufficient for existing tap costs and artifact
selectors: define the source semantics and reset/exhaustion rules explicitly.
Only the six equipped items enter engine state; reserve inventory never does.

Potion inventory is seat-private in every initial/reconnect view, legal action,
replay projection, log and spectator path. Activated effects become public as
normal. Merely hiding public command-zone objects in the renderer is not enough.
The opponent AI receives its own potions and the human's public items, never the
human's hidden inventory. Any public source IDs/counts must not leak that inventory.

Active item/potion requests go through the legal-action path and ordinary costs.
One activation consumes exactly one charge even after retries. Public trigger
and stack entries identify the source item and ability. Client, Expert, LLM and
trained seats must handle the same new requests without guessing. Expert uses
bounded deterministic planning and may stop optional combos; engine legality is
not capped by the AI's strategic stopping rule.

## UI composition and performance

One shared item presentation module serves shop, equipment, inventory, reward
explanations and duel drawer. Typed models distinguish owned item, offer and
public opponent equipment. Effects/rarity labels come from a generation-bound
catalog, not independently drifting UI prose.

Use a 9-slice outer frame, independent 3-slice shelf beams and a quiet separate
backing. Corners/endcaps retain proportions; rows are layout-derived, never baked
into one full-screen image. Image generation supplies original textures only.
Texture atlas/UV sampling can share source images; cache handles/materials, not
one texture per rarity. Rarity/fusion ornament uses GPU time, no per-frame ECS
material writes. Do not render off-page items or animate hidden screens.

Layout starts with left equipment/potions, dominant right shelf, full-width
inventory below; stats and deck/start actions remain reachable at 1280x720.
Smaller widths reflow the same components. Use at least five measured passes:
(1) navigation/flow, (2) shelf/equipment proportions, (3) hover/drag/fusion,
(4) duel/reward/reconnect, (5) readability/motion/performance. Keep screenshots,
control/layout evidence and issue corrections, not just an iteration counter.

## Implementation order and targeted acceptance

1. [x] Lobby entry: remove Tasks nav, three tiles beside tables, gated availability.
2. [x] New generation: Commander collection/preset, five starters, reward phase.
3. [x] Economy: six slots, paginated reserves, auto-fusion, freeze/refill.
4. [x] Engine: independent sources, hidden potions, active effects and all seats.
5. [x] Item catalog: every tier adds abilities; expand families and bounded prices/income. Human balance calibration remains a release task.
6. [x] Shared icon/detail/frame renderer, modular shelf and restricted builder.
7. [ ] Native full run, rendered refinements, focused gates, PR/review.

| Area | Essential regression / adversarial checks |
| --- | --- |
| Lobby | Central+gateway sees three tiles with table controls; guest/offline still usable; no Tasks tab or duplicate hidden route |
| Run isolation | new run cannot inherit items/cards/gold; account switch discards stale replies |
| Commander | 39+1, singleton, identity, <=5 sideboard, free basics, unowned card denial; frozen preset matches |
| Rewards | exact opponent source; 3/1 choice; small pool; no basics/sideboard; retry/reconnect cannot reroll or duplicate |
| Freeze | all/partial/empty shelf, sold slots, repeated visit, paid reroll with no replaceable slots, affordability |
| Fusion | active+reserve permutations, lowest active slot, cascade, exact basis, duplicate request, insufficient gold rollback |
| Inventory | stable pages across sort/group, stale page mutation, full active slots, drag swap and right-click equivalence |
| Privacy | serialized owner/opponent/spectator/reconnect/replay views do not disclose unused enemy potions |
| Rules | ordinary removal cannot target items; ordinary cards still can be removed; priority/cost/targets/charges/replay |
| AI | legal activations, own consumables only, deterministic decisions, finite voluntary combo choice |
| Income | verified facts only, overflow and bonus cap, no void reward or duplicate settlement |
| Rendering | 1280x720 and 1440x900, focus details, long German text, reduced motion, unchanged idle components |
| Performance | visible-page-only entity count; bounded atlas/material cache; no Tower hot-path tax in normal engine |

Run pure domain/property cases first, then affected crate tests together to reuse
builds, one real DB transaction suite after state changes, and a small native
story after integrating UI. Repeat only after relevant changes or failures.
No unchanged workspace-wide gates, no CI wait, no synthetic whole-catalog solve.
Always stop owned test processes and verify listeners close.

## Evidence ledger

- Preparation: source-map inspection established three starters, 3+6 item slots,
  40–60-card non-Commander collection, explicit manual fusion, two frozen offer
  pins, immediate post-settlement shop and public command-zone supplies.
- Original cabinet concept visually inspected; modular frame and shelf generated
  using it as the sole style reference. Native integration/animation still pending.
- Lobby milestone: Tasks navigation removed; authenticated gateway sessions show
  Tower/Daily/Weekly beside tables, while table creation moves to that panel.
  Central login opens gateway selection first; deep-linked account-global task
  details remain readable before joining. Tower Back returns to the lobby.
  Guest/offline controls retain their existing mode. Three stable illustrations
  share cached materials and controls participate in the keyboard order.
- Targeted native headless checks: 107 selected, 97 passed initially; the ten
  remaining fixtures still tried removed controls. Corrected fixtures passed
  in the focused 11-test rerun (1.093 s; 34.73 s incremental compilation).
  The initial build after Gemini's dependency/profile changes took 3m26s.
  Logs: `/tmp/baylee-tower-lobby-redesign-20261115{,-r2}.log`.
  Rendered visual acceptance is still pending; these tests do not establish it.
- Client/client-core library and test Clippy with `dev-control` and warnings
  denied passed; `git diff --check` passed. No service or GUI process was started.
- Inventory foundation: explicit legacy/run layout preserves old serialized
  commitments. Six active positions and dense unnumbered storage use a slot
  bitmask and sorted instance IDs. Automatic purchase/fusion is atomic, prefers
  equipped ingredients in slot order, cascades Common → Uncommon → Rare and
  never consumes a Mythic material. Right-click toggles without displacing an
  equipped item; explicit moves swap. This foundation is not yet enabled for a
  published generation. Fourteen focused economy tests passed, including rollback,
  legacy byte identity, 30 stored items, slot/ID aliases and fusion placement;
  tower library/test strict Clippy passed (7.38 s test build, <0.01 s tests).
- Shop foundation: the separately registered, not-yet-default generation four
  sells six equipment and two supply offers; duplicate offers are intentional.
  Relic purchases always use automatic fusion. A persistent shelf switch retains
  unsold identities and refills purchased slots; a fully frozen shelf cannot
  charge a reroll. New rerolls cost 2/3/4/5/6 gold, then stay at 6. The first
  fourteen scoped shop tests passed (0.16 s), including exact byte digests for
  both older offer streams, persistent freeze, sold-slot refill and atomic gold.
  Tower library/test strict Clippy passed. Native generation-five equipment and
  Commander admission must be integrated before making generation four default.
- Commander foundation: frozen card metadata, 39+1 and <=5 sideboard validation,
  singleton across zones, color identity, owned commander and five free basics.
  The authority admits five to eight distinct commander starters for the new
  generation, preserves the three-choice legacy publication encoding and fences
  requested starter indices against the exact publication. Native packages carry
  all three zones and bind the metadata in their catalog digest. Migration 013
  widens house fallback archetypes to eight; application admission still uses the
  saved publication's exact count. Native default remains generation three.
- Domain validation: 79 tower tests passed in 0.16 s, followed by the added
  five-distinct-commanders case. Tower/API/store/client-core all-target checks and
  strict library/test Clippy passed. The first Clippy pass found an unnecessary
  `expect` during sideboard sorting; sorting before construction removed it.
  Logs: `/tmp/baylee-tower-commander-{domain,starters,clippy-20261115-r2}.log`
  (the first two filenames also carry `-20261115`).
- Reward milestone: four focused domain cases cover immutable opponent sampling,
  exact choice cardinality, duplicate compensation, terminal wins/defeats and
  package mismatch. Twenty-seven native client/core/API cases passed, including
  five-starter enrollment and strict request shapes. Strict Clippy passed for
  the affected library/test targets. The reward screen uses card thumbnails,
  inspection, selection count and duplicate labels; rendered acceptance is pending.
- Isolated PostgreSQL exercised migration 013, the fifth starter, verified
  settlement retries, concurrent identical claims, persistent frozen offers,
  cross-account refusal and a fresh empty run. All three selected execution
  cases passed. The owned PostgreSQL process exited and its listener and
  temporary cluster were removed. Logs:
  `/tmp/baylee-tower-run-postgres-20261115.log`,
  `/tmp/baylee-tower-reward-client-tests-20261115.log`,
  `/tmp/baylee-tower-reward-client-clippy-20261115-r3.log`.
- Native opening/privacy milestone: fourteen focused core/engine/host cases passed
  (0.14 s execution). Bounds remain unchanged without a compiled pack; native
  London redraws cover 19-card hands and 25 free mulligans. Private consumables
  have fixed source identities, owner-only wire projections and sanitized search
  copies. Removing the simulation redaction deliberately made the new regression
  fail; restoring it and adding the delayed-log/owner-concession case passed both
  targeted cases (0.06 s). Strict Clippy passed for core/engine/host/Tower libraries
  and tests. Logs: `/tmp/baylee-tower-native-inventory-20261115.log`,
  `/tmp/baylee-tower-native-inventory-clippy-20261115.log`,
  `/tmp/baylee-tower-private-negative-20261115.log`,
  `/tmp/baylee-tower-private-positive-20261115.log`.
- Independent native review found that an eliminated seat's last-known source
  and retained journal could preserve private charge counts in a simulation.
  Concealed stock is now a two-seat contract; every fair fork explicitly refuses
  a departed seat, including games prolonged by win-prevention effects. The
  independent re-review found the issue closed. Three targeted privacy cases
  passed in `/tmp/baylee-tower-review-privacy-20261115.log`; this is not a full
  repository audit.
- A separate `baylee-tower-content` adapter derives five legal starters and 106
  card records from the compiled registry, without linking that pool into the
  authority. Three focused tests passed, including forged commander metadata,
  off-color cards, singleton across zones, sideboard bounds and undeclared
  printing/construction rules. Its offline `--check` command passed and reports
  balance as unmeasured. Strict all-target Clippy passed for the content adapter
  and the reviewed core/engine/host changes. Native publication remains pending.
- First workshop implementation: modular original frame/shelves, six equipment
  positions, potion row, paginated sortable/grouped bag, revision-bound direct
  purchases and drag/right-click commands, shared keyboard/hover details and
  cached rarity/fusion shader. Thirteen focused client tests and two domain-facing
  inventory tests passed; strict client/core Clippy passed. Evidence:
  `/tmp/baylee-tower-workshop-tests-20261116.log`,
  `/tmp/baylee-tower-workshop-core-tests-20261116.log`,
  `/tmp/baylee-tower-workshop-clippy-20261116-r5.log`.
  These are headless correctness checks, not rendered refinement passes. Native
  layout, motion, drag feedback and tooltip edge placement still need inspection.
- Current executor integration: new enrollment defaults to generation 4, native
  rules 5 and Expert 22. The gateway advertises this exact identity; the durable
  executor and verifier use RunPack. All five actual published starters pass
  native admission, while retained setup is refused by this artifact. Historical
  domain/test fixtures explicitly keep generation 3 and frozen commitments.
  The changed native tests passed (50 in the first pass, then the one corrected
  retained-registry replay test); 58 affected domain cases passed. Strict Clippy
  passed for native all-targets and the affected host/service/store/core targets.
  Logs: `/tmp/baylee-tower-activation-tests-20261116.log`,
  `/tmp/baylee-tower-activation-tests-20261116-r2.log`,
  `/tmp/baylee-tower-activation-native-clippy-20261116.log`,
  `/tmp/baylee-tower-activation-host-clippy-20261116-r4.log`.
  Real gateway settlement/frozen-stock/refill story is updated but not yet run
  against the newly built executables. Publication balance remains unmeasured.
- The native gateway story now passes against the current Tower executable:
  purchase, equip, frozen offers, reservation, crash recovery, verified loss,
  five-card reward, exact one-card claim, next shop and idempotent retries.
  One settlement, one recovery, zero quarantines; 32.05 s execution. The fixture
  was corrected to buy into Inventory before the explicit move (generation four
  intentionally refuses the legacy Delivery::Slot purchase). Log:
  `/tmp/baylee-tower-run-gateway-story-20261116-r2.log`.
- Rendered refinement 1 (native 1440×900 and 1280×720, left external monitor):
  purchase, right-click equip and drag between equipment slots worked. The
  measured rich tooltip fits above a bottom-row item. Found excessive backdrop
  contrast, wrapping shop toolbar, primary actions below the viewport and an
  oversized shelf pushing inventory down. These are failures to correct, not
  visual acceptance. Screens: `/tmp/baylee-tower-pass1-1280.png`,
  `/tmp/baylee-tower-pass1-tooltip.png`, `/tmp/baylee-tower-pass1-dropped.png`.
  Client PID 46775, backend 46746 and PostgreSQL 46731 exited; their loopback
  listeners closed. This is the first actual rendered pass, not the first five.
- Workshop refinement implementation: quiet backdrop, shorter shelf and toolbar,
  early Deck/Play actions, five Commander profiles, closed-run summary and all
  zones of the run-owned deck editor. Accessible explicit equipment destinations
  supplement drag/right-click. Thirteen affected UI cases, three tooltip cases,
  the rich-detail idle/cleanup case and two formation cases passed. Strict
  client/core Clippy passed. Evidence:
  `/tmp/baylee-tower-workshop-pass2-{tests,tooltips,details,formation}-20261116.log`,
  `/tmp/baylee-tower-workshop-pass2-clippy-20261116-r3.log`.
  The second rendered inspection is still pending at this commit.
- Rendered refinement 2 (native 1440×900 and 1280×720): the quieter cabinet,
  visible Play/Deck actions and six equipment slots improve the hierarchy.
  Purchase and the run-owned deck editor were exercised, including main-to-side
  movement and a replacement basic land. The small viewport still pushes the
  bag too low; the next pass combines the status/action band and moves repeated
  help into tooltips. Screens: `/tmp/baylee-tower-pass2-owned-glow.png`,
  `/tmp/baylee-tower-pass2-deck.png`,
  `/tmp/baylee-tower-pass2-1280-settled.png`. Client 48888, backend 48868 and
  PostgreSQL 48837 exited; ports 65509 and 65457 closed. English ability copy
  and insufficiently distinct duplicate highlighting remain open findings.
- Table equipment drawer: shared item portraits/details, public opponent
  equipment, owner-only potions and ordinary ability selection. Independent
  run sources no longer cover the Commander's card pile. Three targeted tests
  cover malicious opponent potion projections, foreign/stale activation,
  spectator access and unchanged Commander/legacy presentation. Identity tests
  and strict client/core Clippy passed. Logs:
  `/tmp/baylee-tower-table-tests-20261116.log`,
  `/tmp/baylee-tower-table-identity-20261116.log`,
  `/tmp/baylee-tower-table-clippy-20261116-r5.log`.
  The third rendered inspection and real table interaction are pending.
- Rendered refinement 3 (1280×720): bought two items and a mana potion,
  equipped both by right-click and entered the real hosted game. Heartwood
  changed the opening from 20 to 24 life; the own potion appeared with one
  charge and opposing potion sources were absent. The Commanders retained
  their separate piles. Opening a potion exposed a real UI defect: the normal
  ability chooser had no card visual to anchor to and remained hidden. This
  pass is not acceptance of potion interaction. A generic UI ability anchor
  and its running-system regression address that finding next. Screens:
  `/tmp/baylee-tower-pass3-equipped.png`,
  `/tmp/baylee-tower-pass3-table-drawer.png`,
  `/tmp/baylee-tower-pass3-potion-chooser.png`.
  Client 51187, backend 51169 and PostgreSQL 51137 exited; ports 53010 and
  52958 closed. All nine existing Command-pile cases passed independently.
- Original item copy now has German rules for every family/tier, rich mana
  symbols and shared wording in the tooltip, ability chooser and stack.
  The 72-variant comparison checks every English sentence against the native
  ability description (only number spelling/plural editorial differences are
  normalized) and complete German coverage. It and the rich tooltip lifecycle
  test passed in `/tmp/baylee-tower-localized-rules-tests-20261116.log`.
  Seven sheet-placement cases, including the new independent-item anchor,
  passed in `/tmp/baylee-tower-ability-anchor-tests-20261116.log`.
- The table drawer is keyboard-accessible through rebindable Shift+I and a
  modal focus order. A running-system test delivers keyboard activation before
  the game's keyboard handler and proves Enter/Space only open the chooser,
  never consume an item or answer the game underneath. Four focused drawer
  tests passed in `/tmp/baylee-tower-pass4-input-tests-20261116.log`; strict
  client/core Clippy passed in `/tmp/baylee-tower-pass4-clippy-20261116-r4.log`.
  The fourth native visual pass remains pending.
- Rendered refinement 4 (1280×720): German item rules fit the shared tooltip.
  Shift+I opened the table drawer; Tab/arrow focus and Enter opened the mana
  chooser without spending a charge or advancing view sequence 5. Choosing
  green visibly changed stock 1→0 and the pool 0→1 (sequence 7). Concede/leave
  produced five distinct opponent cards, an exact one-card choice and 12 gold;
  claiming returned to stage 2 with new offer IDs 9–16. Found raw reward count
  placeholders, a claim action below the viewport and a retained reward scroll
  offset in the new shop. Corrections are part of the next pass. Evidence:
  `/tmp/baylee-tower-pass4-{german-tooltip,table-drawer,mana-chooser,reward,return-shop}.png`.
  Client 53276, backend 53255 and PostgreSQL 53224 exited; ports 56649 and
  56595 closed. This validates interaction, not broad balance.
- The opt-in `BAYLEE_TOWER_UI_SHOWCASE=1` native demo fixture supplies a valid
  isolated run with six active items, all rarities, more than two inventory
  pages and a Common→Uncommon→Rare purchase cascade. It exists only in the
  gateway test binary and still uses ordinary authenticated APIs. No production
  hook or normal starting resources change. Its domain validation/cascade test
  and strict gateway test-target Clippy passed:
  `/tmp/baylee-tower-showcase-test-20261116-r2.log`,
  `/tmp/baylee-tower-showcase-clippy-20261116.log`.
- Rendered refinement 5 used that synthetic high-rarity fixture at 1280×720.
  The 4-gold Grove purchase cascaded Common→Uncommon→Rare into active slot 0
  (instance 47, wallet 10→6); the fully frozen reroll before purchase charged
  nothing. A confirmed Rare Heartwood + one Mythic potion became Mythic in the
  same slot (instance 48, starting life 31→36, material stock 1→0). Dragging
  occupied slots exchanged instances 47/5; right-clicking a reserve while all
  six slots were occupied left them unchanged. Page 2 rendered only its 16 bag
  items plus six equipped items (22 item controls, no off-page item controls).
  German Mythic text fit at both M and XL. Matching/fusing offers now have
  visibly different cyan/silver/gold/orange frames. The original modular frame
  has a shared GPU-clock inlay, with static reduced-motion appearance.
  Screens: `/tmp/baylee-tower-pass5-{showcase,fused,mythic-detail,xl-mythic,mythic-confirmed,page2,lobby}.png`.
  Two further findings require a sixth pass: a newly opened upgrade confirmation
  must scroll into view, and the 720-pixel shop should expose more of the bag.
  Client 55057, backend 55018 and PostgreSQL 54987 exited; ports 60021 and
  59968 closed. The mixed, unoptimized 356-second session averaged 53.24 fps
  (p50 17.015 ms, p95 19.810 ms, p99 39.648 ms); it includes startup, compilation,
  settings/layout changes and captures, so is not an isolated shader benchmark.
- The reward placeholder regression passed; all 35 preference cases and 13
  Echoes UI cases passed. Existing preference tests caught a missing removal of
  the new action from the historical keymap and a stale action-count expectation;
  both were corrected. Strict client/core dev-control Clippy passed. Logs:
  `/tmp/baylee-tower-pass5-copy-tests-20261116.log`,
  `/tmp/baylee-tower-pass5-preferences-tests-20261116-r2.log`,
  `/tmp/baylee-tower-pass5-ui-tests-20261116.log`,
  `/tmp/baylee-tower-pass5-clippy-20261116-r2.log`.
- Rendered refinement 6: all five equal starter choices fit side by side at
  1280×720; the compact workshop exposes the first inventory row. An upgrade
  initiated below the fold now brings its confirmation to the top. Real
  concede/leave shows the translated `0 von 1` count and reachable claim;
  choosing one duplicate adds one gold and opens stage 2 with fresh stock.
  This revealed that the scroll-memory system could overwrite the phase reset;
  the reply handler now follows that system. Reward tiles also use tighter
  padding/gaps; these last two corrections require the next native pass.
  Screens: `/tmp/baylee-tower-pass6-{1280,confirm-visible,five-starters,reward,return-shop}.png`.
  A fixed shelf crop changed 12,136/228,000 pixels with animation enabled.
  The previous reduced-motion pair changed 0/290,700 pixels. These are motion
  observations, not performance measurements. Client 57317, backend 57297 and
  PostgreSQL 57266 exited; ports 64655 and 64603 closed.
- Four Tower UI tests, thirteen Echoes UI tests and five item-art tests passed.
  A follow-up focus test proves mouse focus does not pin an enabled item tooltip;
  the existing disabled-control tooltip regression also passes. Strict client
  and client-core dev-control Clippy passed. Evidence uses
  `/tmp/baylee-tower-pass6-{tests,echoes-tests,clippy}-20261116-r2.log`,
  `/tmp/baylee-tower-pass6-item-art-tests-20261116.log`,
  `/tmp/baylee-tower-pass6-{focus,disabled-tooltip}-tests-20261116.log`.
- Permanent native collection: Central's existing authenticated progress reader
  supplies Duel master, Crown conqueror and Immaculate. No client-owned counters
  establish an earned tier. One account-scoped mailbox and one in-flight read
  bound work; a visible collection refreshes at most every 30 seconds unless the
  player explicitly refreshes. Failed/corrupt replies preserve confirmed totals,
  and account replacement discards both the cache and departed mailbox. Three
  new original procedural motifs extend the existing cache to 66 fixed slots.
  All five cosmetic tiers and exact goals have DE/EN previews. Sixteen Echoes
  tests and three medal/shader tests passed; strict client/core dev-control
  Clippy passed after the compact layout refinement. Logs:
  `/tmp/baylee-tower-collection-tests-20261116.log`,
  `/tmp/baylee-tower-collection-medal-tests-20261116.log`,
  `/tmp/baylee-tower-collection-clippy-20261116-r3.log`.
- Rendered refinement 7 verified the real Central collection at DE/M and EN/XL,
  dismissed mouse-only item detail after moving away, and replayed a native
  duel/reward/shop cycle. All five reward buttons fit at the 720-pixel floor;
  claiming from a scrolled reward view restored the shop title to y=94 and its
  Play control to y=180. The final compact collection required another pass.
  Screens: `/tmp/baylee-tower-pass7-{collection-de,collection-xl-en,unbroken-de,mouse-tooltip-dismissed,reward,return-shop}.png`.
  Client 59176, backend 59151 and PostgreSQL 59127 exited; ports 52246/52193 closed.
- Rendered refinement 8 fits the three tracks and all five labelled goals at
  1280×720. Choosing Crown conqueror changes its motif/requirements. A further
  real duel and claim returned to stage 2, and changing text size/language kept
  the choices usable. Screens: `/tmp/baylee-tower-pass8-{collection-de,ascents-de,return-a,return-b,shop-a,shop-b}.png`.
  A presentation observation remains for final review: some earlier captures
  lacked the header title despite valid visible node geometry. Seven later
  native captures over 24 seconds, spanning rebuild counts 154–162, each
  contained exactly 1,137 bright title pixels; this does not establish the
  earlier cause. Evidence: `/tmp/baylee-tower-title-probe-20261116.log` and
  `/tmp/baylee-tower-title-probe-{0..6}.png`.
  Client 59953, backend 59923 and PostgreSQL 59907 exited; ports 54384/54341 closed.
- Verified leaderboards: API 6 and migration 014 retain one completed personal
  best per account/cohort/lane. Eight wins or five losses/draws establish a best
  inside independent verified settlement; any house opponent puts the run on
  the separate house board. Equal scores share competition ranks, and the top
  25 plus the requesting owner's best are bounded projections without account
  IDs. Publication defaults off, requires a live Central public identity and a
  revisioned explicit choice, and may be withdrawn without losing progress.
  The client binds replies to the requested cohort/lane, preserves ambiguous
  mutations for exact retry, and offers the board from the run overview.
- Scoped final gate, first pass: 11,299 tests across the 27 changed packages ran
  in 605 seconds after compilation; 10,902 passed and 397 failed. Most failures
  shared the headless Lobby fixture's missing drag/drop message registration.
  Remaining failures exposed stale navigation/action counts, shader-path audit
  assumptions, old loop-policy replay fixtures, API version headers and migration
  rollback fixtures. These were corrected without changing gameplay to satisfy
  obsolete assertions. The targeted follow-up ran 505 tests in 40.742 seconds:
  504 passed, including all new ranking/HTTP/privacy/storage tests. One legacy
  wire-policy assertion required a second correction: explicit wire 0 remains
  RunOnceThenBreak, 2 is Off, and absent historical wire rules retain the legacy
  semantics. New table creation explicitly selects Off. Its final rerun and
  the strict lint/feature checks remain pending at this entry. Evidence:
  `/tmp/baylee-tower-push-gate-20261116/{tests,regressions}.log`.
  The regression fixture's PostgreSQL PID 91198 was reaped and port 61692 closed.
- Disk exhaustion interrupted two supplemental compile attempts before their
  tests ran. Only this checkout's regenerable debug/gate incremental caches
  were removed; sources, logs and preserved native executables were retained.
  The subsequent regression run above compiled successfully. Interrupted
  compilations are not counted as successful tests.
- The final legacy-wire assertion passed in isolation (0.026 seconds); all
  originally failed test names now have a successful targeted rerun. Strict
  all-target Clippy across the 27 changed packages and scoped formatting passed
  (`/tmp/baylee-tower-final-lint-20261116-r2.log`). The client dev-control/dev-reload
  Clippy pass also succeeded; its feature tests and the remaining matrix are
  still running at this entry. The AVIF example's duration display now uses
  `Duration::as_secs_f64`, removing its lossy integer cast without touching decoding.
- Rendered refinement 9 verified both leaderboard lanes, shared ranks 1/1/3,
  private personal bests, explicit publication confirmation and removal of the
  public identity after hiding. It found missing Fluent argument declarations
  for date/score; these are fixed, with shorter player-facing copy and 17 passing
  localization/ranking tests (`/tmp/baylee-tower-ranking-copy-tests-20261116.log`).
  Corrected copy still needs the next native capture. Shop captures at 1440×900
  and 1280×720 showed the title; a real conceded duel offered five cards, accepted
  one, returned to stage 2 with four run lives/26 gold and fresh shelf stock.
  The first return screenshot lacked the crown icon; the later capture included
  it. Both contained the heading. Derived camera/clipping inspection found no
  missing target or clipped text center in the observed settled state; this does
  not resolve the earlier intermittent capture observation.
  Screens: `/tmp/baylee-tower-pass9-{shop-1440,shop-1280,ranking-echoes,ranking-public,return-shop,return-shop-b}.png`.
  Client 94312, backend 94116 and PostgreSQL 94085 exited; ports 64074/64018 closed.
