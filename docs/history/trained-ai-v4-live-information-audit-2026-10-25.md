# Live neural information audit — 25 October 2026

Status: source audit, **not a claim that the omissions below are fixed**.
The current contract is encoder 6. Encoder 7 is the proposed next contract;
archived encoder-6 models, exports, datasets and experiments must retain their
original inputs. This audit used source inspection only: no GPU work, live
hidden-state inspection, new rollout or implementation change.

The motivating observation is a neural seat tapping mana without using it.
That observation alone does not establish its cause. The code does establish
that the actor does **not** currently receive all information a player has.
Printed card rules being present is different from a complete current game
state, and neither establishes that the trained weights use the information
correctly.

## Priority and interpretation

- **P0:** no new confidentiality or hidden-information leak was established by
  this audit. Preserving the seat-authorized boundary is mandatory for the fix.
- **P1:** the representation drops or merges facts needed for ordinary planning:
  exact own-deck inventory, current mana capabilities/payment permissions,
  projected characteristics and exact public quantities; zone caps and order
  also prevent a general “same information as a player” claim.
- **P2:** additional public-history and identity detail is omitted or approximate.
  These remain explicit requirements rather than claims that more training
  will recover the missing fields reliably.

“Present” below means the feature/catalog path exists. “Lost” means a source
field is omitted or different values collapse in the actor input. “Learned
unknown” means a faithful input still needs tactical learning; this audit does
not measure that learning. Recurrent memory may help reconstruct some omitted
facts from earlier observations, but is not an exact replacement for current
public state or a deterministic inventory ledger.

## Authorized flow and all twelve entity-zone identifiers

`live4/brain.rs:55` initializes from the seat's submitted `main` and
`commanders`, using `NetPlayer4::init_own_seat`
(`netplay4/live_init.rs:10`). No opponent deck or library order enters that
interface. `NetPlayer4::observe` builds state from `PlayerView`, knowledge and
public teams (`netplay4/mod.rs:248`). Known hand cards are not filtered to the
currently legal cast list: `features4/state.rs:189` visits the entire own hand,
and `push_hand_object` attaches the card's rule references at line 622.

Paths in the remainder of this document are relative to
`crates/baylee-train/src/`, unless stated otherwise. Line numbers identify the
audited source and may move in later revisions.

| ID | Entity zone | Current source/behavior |
| --- | --- | --- |
| 0 | Battlefield | All `view.battlefield` entries, first in traversal. |
| 1 | Stack | All `view.stack` entries, after battlefield. |
| 2 | Own hand | Entire `view.hand`, including cards not currently castable. |
| 3 | Shared hand | All legally supplied `shared_hands` cards. |
| 4 | Controlled hand | All legally supplied `controlled_hands` cards. |
| 5 | Graveyard | All seats' `view.graveyards`, subject to shared entity cap. |
| 6 | Exile | All seats' `view.exile`, subject to shared entity cap. |
| 7 | Command | All seats' `view.command`, subject to shared entity cap. |
| 8 | Looking at | Authorized `view.looking_at` entries, e.g. tutor inspection. |
| 9 | Library top | Authorized `view.library_tops` entries. No hidden order. |
| 10 | Commander designation | Enum declared at `features4/state.rs:25`, but no entity is ever emitted with this zone. |
| 11 | Remembered opponent card | Previously revealed cards from knowledge,
  appended last, unless their object ID already has an entity. |

The actual traversal order is battlefield, stack, own hand, shared/controlled
hands, looking-at, library tops, command, graveyards, exile, remembered cards
(`features4/state.rs:163`). Each push stops at `MAX_ENTITIES = 1024`
(`features4/mod.rs:64`, `state.rs:326`, `state.rs:561`). There is no reservation
for the hand, offered objects, commanders or a tutor's offered cards. A large
battlefield can consume the whole budget. Later objects then lack full state
tokens even if an action still names them. Shared/controlled/looking-at views
are not generally deduplicated against earlier zone rows; these duplicates
also consume the budget. Only the remembered-card append checks existing IDs.
These are **P1 losses**, not a guarantee that all visible zones always fit.

The public-object path selects identity from `obj.rules` or authorized
`obj.card`, otherwise `UNKNOWN_CARD` (`state.rs:397`). References with unknown
identity do not select a card root (`cardstructure/refs.rs:29`). The view's
entitlement determines whether a face-down identity is available; the actor
does not look it up from engine state. Full library contents and unrevealed
opponent hands are intentionally absent. This is correct and must remain so.

## Own deck and opponent knowledge

**P1 — exact submitted deck inventory does not reach the net.**
`SeatKnowledge4::from_own_cards` stores a sorted `(card, copies)` list
(`features4/knowledge.rs:168`), and `from_preset` reads only the viewer's seat
(`knowledge.rs:188`). However, the only deck input emitted by the state encoder
is the normalized 24-float `own_deck_summary` (`state.rs:892`). The exact
`own_deck_cards` list has no actor input or remaining-card calculation.
The summary covers curve, color/type and broad role shares
(`knowledge.rs:388`); it is static and has no per-card removal/update ledger.
Different exact decklists and multiplicities can have the same summary.

Consequences: the actor cannot directly inspect which specific unobserved
cards remain available for a future tutor, how many copies remain after cards
leave the library, or its exact registered inventory. A current tutor question
can still expose legal cards through `looking_at`/candidates; that is later
information, not prior planning knowledge. Live initialization omits the own
sideboard (`live4/brain.rs:55`), as does `from_preset`'s inventory loop.
Copy counts are `u8` and saturate at 255; live counts are clamped to that width.
The eventual exact ledger must preserve legitimate larger counts and distinguish
known inventory from unknown library order.

Opponent memory is present: authorized shared/controlled hands, inspection and
revealed tops populate `known_cards` (`knowledge.rs:325`). Public movement to
stack/battlefield/graveyard/exile removes remembered entries; an empty hand
clears origin-1 entries (`knowledge.rs:294`, `knowledge.rs:370`). This is a
memory of revealed identity, not omniscient opponent inventory.
**P2:** remembered entity rows retain card identity but discard
`last_seen_turn` and `origin_zone` (`state.rs:632`); a former revealed top and a
former shown hand card have the same remembered-zone encoding. Stored memory
also records no face, and reconstructs face 0. Do not interpret these rows as
proof of current hidden-zone location.

## Printed costs and abilities: present, not proof of understanding

The structural catalog retains every face, effective printed keyword list,
ability list and all faces together (`cardstructure/catalog.rs:36`). It has
separate selected ability, modal branch and alternative-cost roots. The
generated typed visitor includes `FaceDef` fields, except abilities/keywords
which the catalog supplies separately (`crates/baylee-train/build.rs:23`).
Thus printed additional/alternative costs, activation costs and their typed
conditions are not merely absent because a card lacks a current cast candidate.

`ManaCost` is serialized as an ordered symbol list
(`cardstructure/mod.rs:150`). Distinct tags retain colored/colorless/generic,
hybrid, two-or-color, Phyrexian, hybrid-Phyrexian, snow and variable symbols,
including the variable identifier (`mod.rs:160`). Public copies, selected stack
abilities, chosen card names and granted ability provenance have explicit
references (`cardstructure/refs.rs:39`). Rust and Python packing retain all
references or error rather than silently cutting an ability list
(`netplay4/structure.rs:29`, `tools/trainer/structure_data4.py:22`).

The Python `RuleEncoder` learns finite-width vectors from all typed nodes and
ordered children (`tools/trainer/model4/structure.py:66`). Its public lookup
does not execute Magic costs or prove that the policy learned hybrid payment,
X planning, alternative costs, tutor value or when to leave mana untapped.
These are **learned unknowns** to measure with tactical tests. Printed rule
availability must not be conflated with the live modifications below.

## P1: mana planning loses current facts

The wire already contains detailed dynamic mana facts:
`PublicObject::{granted_mana, board_mana, ability_mana_spending, flashback}`
(`crates/baylee-view/src/objects/mod.rs:370`) and the seat's actual pool and
spending matrix (`crates/baylee-view/src/seats.rs:13`). The actor reduces them:

| Visible fact | Current actor encoding | Lost detail |
| --- | --- | --- |
| Granted or board-dependent production | One presence bit, f85 (`state.rs:498`) | Actual allowed colors, granted amount, exact source/ability association from these fields. Printed rules/provenance can help with some sources but do not transmit this projected result. |
| Ability-specific spending permission | Non-default bit, f89 (`state.rs:507`) | Which actual colors may pay which requirements. |
| Seat spending permission | Non-default bit, seat34 (`state.rs:759`) | Complete permission matrix. Different non-default permissions collide. |
| Projected graveyard cast cost | Presence plus capped mana value, f86–87 (`state.rs:501`) | Exact colors/symbols of a granted/changed cost. Printed flashback remains in printed roots where applicable. |
| Floating mana | Six color counts capped at 8, total capped at 12 (`state.rs:750`) | Larger exact quantities. |
| Restricted mana | Six color totals capped at 6 (`state.rs:760`) | Larger exact quantities; the wire itself does not fully state each spending restriction. A full player-information contract must distinguish encoder losses from information missing upstream. |
| Current payment owed | Fixed/any-amount flags and capped total (`state.rs:874`) | Fixed cost's symbol/color composition, already identified as a separate issue. |

Already-known boundary issues remain: priority cast candidates cover only
`legal.castable` (mana already floating); the separate `legal.payable` offers
that allow announcing a cast before its payment window are omitted. Live
encoder-6 compatibility deliberately clears `casting`
and `targeting` before both observations and answers (`live4/mod.rs:19`). This
audit does not change either behavior. The next contract must expose the
current planning/payment/selected-source context faithfully on both live and
offline paths, rather than making a silent encoder-6 input change.

## P1: current projected characteristics and exact quantities

Current card types, five colors, status, controller/owner, P/T, damage and
loyalty have numeric feature paths (`state.rs:339`, `state.rs:429`). They are
not all exact. The helper explicitly clips normalized values
(`features4/mod.rs:99`):

| Quantity | Current range/cap |
| --- | --- |
| Each seat's absolute life | Signed value clipped at ±40 (`state.rs:707`). |
| Life minus initial observed life | Signed difference clipped at ±40; partly supplements absolute life, but is not an unbounded life channel. |
| Current power/toughness and base P/T | Each clipped at ±12 (`state.rs:430`). |
| Marked damage / remaining toughness | Damage capped at 10; remaining toughness clipped at ±12. |
| Loyalty and defense counters | 10. Projected loyalty also has cap 10. |
| Hand / library / graveyard / exile count | 7 / 60 / 20 / 10 (`state.rs:717`). Entities can supplement counts while they fit, but hidden library count above 60 is not recovered by public entities. |
| Board creature/land/untapped-land/other counts | 10 each; aggregate power/toughness 25 (`state.rs:743`). |

The initial-life difference and history channels mean one must not claim that
*every* pair of life values above 40 produces identical full input. For a
Commander seat starting at 40, life 80 versus 100 saturates absolute life,
delta and cumulative life-gained channels alike, with otherwise equal current
state/knowledge. Likewise same-card creatures with current and remaining P/T
12 versus 20 can collide after relevant counter/aggregate channels also
saturate or are kept equal. Tests must compare complete inputs, not one channel.

Current projected keywords are reduced to Flying and “has any keyword”
(`state.rs:525`). Printed keyword trees do not encode the complete **current**
projected bitset. Gained/lost haste, vigilance, trample, hexproof,
indestructibility and other keywords therefore lack faithful direct channels.
`NO_ABILITIES` and summoning-sick flags are present, but do not repair the whole
bitset. Public projected `subtypes` and effective `word_changes` are also not
encoded by the state or reference builders. `chosen_subtype` is present as a
separate scalar; it is not the projected subtype set. Changed protection or
mana-related words cannot be treated as unchanged printed rules.

`prepared_spell` is only a presence bit (`state.rs:413`), without its selected
rules identity/reference. Existing visible exile entities do not supply a
dedicated relation identifying which one this source prepared.

### Every counter kind

The wire supplies exact typed `CounterEntry` records. The actor merges them
into twelve channels (`state.rs:465`), so “all categories covered” does not mean
each type/count remains distinguishable:

| Counter kind(s) | Encoding |
| --- | --- |
| Plus `{power,toughness}` | One capped channel (8), weighted by max component; different plus-counter shapes/counts can merge. |
| Minus `{power,toughness}` | Separate analogous capped channel (8). |
| Loyalty | Own channel, cap 10. |
| Defense | Own channel, cap 10. |
| Lore **or** Level | Shared channel, cap 6. |
| Time | Own channel, cap 6. |
| Charge **or** Bore **or** Energy | Shared channel, cap 8. |
| Stun | Own channel, cap 4. |
| Shield **or** Indestructible | Shared channel, cap 4. |
| Finality | Own channel, cap 2. |
| Flying **or** Lifelink **or** Shadow **or** Hone | Shared channel, cap 4. |
| Poison **or** Rad **or** Custom(any ID) | Shared accumulated channel, cap 6; custom identity is absent. |

Some shared channels overwrite rather than sum when multiple corresponding
entries exist. Current P/T can supplement stat-counter effects below its own
cap, but cannot reconstruct all original counter kinds/counts. Every player's
poison/energy/speed/rad also has a seat channel; poison/energy/rad saturate at
10, while speed's 0–4 range is represented (`state.rs:710`).

## P1/P2: order, relations, commanders and timing

Attachments, attack defenders, blocker pairs, bands, stack source/targets,
chosen opponent, battle protector and visible grant source have typed edges
(`state.rs:901`). These are present. The edge builder silently stops at 8,192
edges (`state.rs:910`), so late links can disappear on large boards.

There is no stack-depth or graveyard-position feature and no corresponding
order edge. Entity tokens have no entity-position embedding
(`tools/trainer/model4/__init__.py:169`); seat positions do have one. Merely
iterating a stack/graveyard vector in order does not transmit that order to
this permutation-equivariant entity encoder. Swapping unrelated differently
named stack objects or graveyard cards therefore requires an explicit order
test; targets and same-card edges must not be mistaken for a general order
representation. This affects resolution-order and order-sensitive graveyard
reasoning even below all size caps.

Commander count, summed casts, maximum commander damage and damage aggregated
by owner-seat are present (`state.rs:764`). Per-commander cast count/damage
is not faithfully retained, and public `SeatView.commanders.card` identities
are not emitted when a commander has no other visible entity. The wire
explicitly makes those identities public even in hand/library
(`crates/baylee-view/src/seats.rs:184`); zone-10's unused declaration does not
implement the tracker.

Turn/step, active/awaiting/decision seats, monarch, day/night, sorcery-lock
presence and sorceries-have-flash are present. Turn saturates at 20, the
observation cursor at 200 (`state.rs:822`, `state.rs:891`). There is no faithful
general list of active continuous/replacement effects and their remaining
duration in this actor state. Printed source text plus history may permit
learning some timing, but that is not a direct complete active-effect ledger;
some detail may require an upstream view addition, not just an encoder field.

**P2 history caveats:** `seen_stack_ids` and `seen_bf_ids` use object handle,
not zone-change incarnation (`knowledge.rs:282`). Recasting/blinking the same
stable handle is not counted again by these totals. `lands_played_total`
counts first observed land arrivals rather than only land plays. The
`attacks_declared_total` field is initialized/read but never incremented in
`knowledge.rs`. These statistics are approximate and must not be used as exact
rule counters. They do not provide a complete movement log for deck removal.

## Measurable encoder-7 requirements

1. **Field accounting:** every seat-authorized `PlayerView` field and every own
   submitted-deck field has a documented lossless actor representation, a
   deliberate privacy exclusion, or a named upstream/view limitation. No claim
   of full player information while P1 entries remain. Version this mapping,
   runtime, exports, conversion and training together; preserve archived 6.
2. **Exact public quantities:** compare complete inputs for same-card 12/12
   versus 20/20 and damage/loyalty beyond current caps, and for high-life states
   such as Commander 80 versus 100. Include 40/80 boundary cases while
   accounting for life-delta/history channels. Require distinguishable exact
   numbers, not just different incidental candidate lists. Test every counter
   kind, mixed counter kinds, custom IDs, amounts and plus/minus shapes.
3. **Inventory/tutor planning:** two equal-summary registered decklists with
   different exact card IDs/copies produce different inventory input before any
   tutor is legal. Drawing, casting, discarding, exiling, returning, shuffling
   and known card transfers update the authorized remaining multiset correctly.
   Test duplicate basics and counts above 255. Never expose unknown library
   order or an opponent's unrevealed decklist.
4. **Mana planning:** same printed sources under different board-produced
   colors/amounts and different spending matrices produce different inputs.
   Fixed owed `{W}` versus `{U}`, granted flashback with different symbols,
   X/hybrid/alternative/additional costs and non-payable cards in hand all have
   explicit assertions. Tactical tests must show selecting a purpose for mana,
   paying the right colors and passing without a pointless tap where appropriate.
5. **Current characteristics:** distinguish otherwise equal views with
   different projected keyword/subtype sets, word substitutions, prepared spell
   identity, and active effect duration/protection where the player can know it.
   Test actual current P/T separately from printed/base P/T.
6. **Zone completeness and order:** exercise every zone ID, all seats' yards,
   exile/command and public commander designations; include a commander in a
   hidden zone with a still-public designation. Exercise more than 1,024
   visible objects and 8,192 relations without silently losing decision-relevant
   state. Deduplicate the same object appearing in inspection and its zone.
   Reverse stack and graveyard order with distinct cards and assert the actor
   can distinguish it; verify the next-resolving/top/above-card semantics.
7. **Privacy and parity:** changing only unrevealed opponent hand/deck/library
   order leaves actor inputs identical; changing an authorized reveal changes
   them. Hidden face identities select no rule root. Live and replay observations
   must agree on complete authorized context, including casting/targeting,
   at each step. No actor helper may consult hidden `EngineState` to fill gaps.
8. **Behavior, separately measured:** passing information tests is not proof
   of competence. Evaluate view-only tactical cases for mana conservation,
   tutor planning, combat/protection, large creatures, loyalty, graveyard/exile
   recasting and Commander tax/damage; retain exact actions, provenance and
   model/version identifiers. An information omission and a learning failure
   must remain distinguishable findings.

### Authorized performance compression

The owner explicitly permits grouping semantically identical permanents when
the exact multiplicity remains known. Encoder 7 need not spend one transformer
token on every identical vanilla token merely to avoid the existing entity
limit. This permission is conditional on preserving decisions and information:

- Equivalence must use the **complete current authorized semantics**, including
  controller/owner, tapped/phased/summoning-sick status, actual/base P/T,
  complete current keyword/subtype sets, every typed counter and exact count,
  damage, attachments, granted definitions and production, relevant duration,
  attacker/blocker assignment, stack targets/references, commander designation
  and per-object history/target-history relations. Equal printing is insufficient.
- Preserve an exact, unclipped group count and the member-ID mapping. Candidate
  selection, legal subset cardinality, choosing individual members, targets,
  attachment/combat edges and expansion back into engine actions must preserve
  the same meaning and legality as the ungrouped representation. Object handles
  need not be arbitrary neural numeric features, but cannot disappear from the
  action/reference mapping.
- If a relevant semantic difference is unknown to the grouping layer, keep
  the objects separate. Split groups immediately after a tap, damage, mark,
  attachment, target choice, counter change or any other distinguishing event.
  Recombine only after full equivalence is established again.
- Stack and graveyard order must survive. Do not aggregate ordered zones in a
  way that erases positions or which cards are above which others. The same
  requirement applies to any other player-known ordered inspection.
- Grouping uses only information this seat may know. It must neither compare
  secret identities nor reveal a hidden difference through group sizes.
- Compression is not permission to drop everything after 1,024 entries.
  Specify a complete overflow strategy for genuinely distinct visible objects
  and relations, with explicit failure rather than an undocumented truncation
  if a supported bound is exceeded.

Require metamorphic aggregate-versus-ungrouped equivalence tests for state
facts, legal candidate meaning, subset cardinalities and decoded actions.
Include several identical creatures where one is tapped, damaged, marked,
targeted or made a commander; verify exact group splitting and member routing.
Test group counts above every old numeric cap, attachment/combat relations
between groups and individuals, and order-sensitive zones. Performance
measurement follows these semantic checks; a faster but informationally
different input is a contract change, not an equivalent optimization.

## Completed live-game evidence and deployed scope

The first anonymized human game replayed with 417 verified inputs, every
snapshot hash exact (zero public-digest fallbacks). The neural seat answered
226 questions, including 161 priority passes, five intrinsic mana activations,
20 ordinary ability activations and two casts. Many ordinary activations were
mana abilities of nonbasic lands. At four main-phase boundaries, respectively
3, 5, 6 and 4 floating mana disappeared: 18 total. Those pools contained white
and black but no blue. The own hand contained Void Rend, which never entered
`legal.castable`; the model did cast Ephemerate when offered. This demonstrates
actual waste and a failure to assemble suitable colors, not a general inability
to cast instants. Sharp's proposed moves in these states are diagnostic teacher
suggestions, not an oracle or proof of optimal play.

`priority_audit4` replays completed records and reports authorized hand instants,
legal casts, omitted payable casts, pool transitions and a view-only Sharp
reference. One second completed record was rejected at input 643 after the
stack-hold correction: do not derive complete-game training or performance
claims from that failed replay. It uses unusual stack holds; a compatible
historical replay is required before conversion. The raw anonymized record is
retained. No unfinished human game was inspected for hidden information.

The three tactical regressions exercise actual Session questions through the
encoder/decoder: Shock on an opponent turn, Counterspell on a stack object,
and an uncounterable target that remains and resolves. They deliberately
select known legal encoded actions, including producing mana first. They test
plumbing, not checkpoint intelligence or the omitted cast-before-payment route.

The local preview adds a verified model-selection manifest checked only before
a game's first observation, explicit critic-based draw-offer replies, and an
anonymized train-only record staging process. The draw rule is a heuristic over
neural expected side utility, recorded separately from learned policy answers;
it is not evidence of calibrated human-opponent win probabilities. These live
changes do not rewrite frozen experiment code, actor weights or PPO data.

Next learning decision: retain the now-completed eight-round experiment and
its comparisons as an encoder-6 baseline; new-seed confirmation did not
establish an improvement (see the
[final report](trained-ai-v4-sharp-intensive-2026-10-25.md)). Do not respond to these findings by
merely increasing model size or appending another broad encoder-6 run. Implement
and verify the encoder-7 information contract, then reconvert replay-compatible
training data and train with the corrected action/payment context. Preserve
encoder-6 evaluation and old checkpoints as controls. Passing is a meaningful
choice; the existing BC converter's five-of-six pass weight 0.2 and BC loss's
3x non-pass boost compound to a 15x per-sample weight difference on those rows
(9x relative to the mean pass weight). Audit/remove that action-dependent bias
in a separately versioned learning objective, then compare learning and actual
mana/interaction behavior on fixed tactical cases and paired full games. The
current heads-only PPO experiment does not use those BC weights; this is a
possible inherited bias, not a measured causal explanation of its outcome.

The audit itself changed no encoder or objective. The following subsequent
implementation is opt-in groundwork, not a deployed encoder-7 checkpoint.

## First implementation milestone after Sharp completion

- `features7::actions::encode_candidates7` preserves the existing priority
  candidates and adds distinct `legal.payable` spells. The engine can then ask
  for modes/targets and open its exact payment window before lands are tapped.
  A priority menu beyond the candidate bound returns an explicit error. This
  does not repair other legacy head/entity/edge limits yet.
- Additive `Session::view_for_with_context` and `observe_view_with_context`
  expose owner-authorized casting/target context from the same builders used
  for sockets. Existing methods and encoder-6 live normalization stay stable.
  Regression coverage checks exact legacy equality after removing only the
  added context, socket parity, and opponent privacy.
- `features7::inventory::RegisteredDeck7` keeps an exact unordered multiset of
  the own submission, with separate sections, counts beyond 255, card identity,
  per-copy deck choices and resolved physical printing rules. Artwork and
  preset-local metadata indices are excluded. The constructor receives one
  seat's submission and only dereferences that seat's rules metadata.
  Changing another deck, RNG seed, submission order, artwork or unrelated
  metadata does not change this inventory.
- The registration inventory is **not** a remaining-library estimator.
  Fixed starting hands are additional physical cards in this engine. An
  absent fixed hand and an explicitly empty fixed hand have the same multiset
  of additional cards but different opening-draw behavior. Setup flow and
  later hidden transfers/copies require their own representations.

Actual Session tests cover announcing a creature before producing mana,
Shock during the opponent's turn, Counterspell targeting a live stack object,
and cancellation during payment. These deliberately select offered actions;
they establish plumbing, not learned planning. `CancelCast` is validated by
the engine before generic `Pending::answer_fault`, using owner-only context.
It remains absent from mode/target heads: cancellation must be represented
as a separate decision, not an extra object in a target subset. That limitation
has its own regression and must be resolved before declaring action coverage.

Before new full-network training: complete exact numeric/typed state facts,
ordered zones and all visible entities; bind the inventory, owed symbols and
choice context to actual neural tensors; represent sideband cancellation;
version model/export/conversion together; test runtime/replay parity and hidden
information invariance. Then reconvert compatible complete records, use an
explicitly versioned pass-neutral BC objective, and evaluate mana/interaction
tactics plus paired games. Archived encoder-6 exports remain the controls.
No old dataset, checkpoint or frozen completed run is rewritten by this work.

## Second implementation milestone: Encoder 7, Loss v3, and runtime/trainer integration

- **`features7::state::encode_state7` (`ENCODER_VERSION7 = 7`, `128` entity / `64` seat / `80` global / `48` candidate features):**
  - Replaces hard clipping on life, power/toughness, damage, loyalty, counters, mana pools, and zone sizes with paired normalized linear and `log1p` magnitude channels (`feat_exact_pos`, `feat_exact_signed`).
  - Expands projected keywords to 24 explicit channels (including haste, vigilance, trample, hexproof, indestructible, deathtouch, lifelink, reach, menace, flash, protection, ward, first/double strike, prowess, defender), adds projected subtype presence/count and active `word_changes` indicators, and separates counter kinds (`Plus`, `Minus`, `Loyalty`, `Defense`, `Lore`, `Level`, `Time`, `Charge`, `Energy`, `Stun`, `Shield`, `Indestructible`, `Finality`, `Poison`, `Rad`, `Custom`) plus signed net P/T from counters.
  - Encodes dynamic mana production (`granted_mana`, `board_mana`), per-ability and per-seat spending permission summaries, granted flashback cost composition, and exact per-color owed payment requirements (`payment_owed_colors`, `payment_owed_generic`, `payment_owed_hybrid`, `payment_owed_phyrexian`, `payment_owed_x`, plus casting/targeting stage flags in `global_feat[64..80]`).
  - Attaches `obj.prepared_spell` (`role: 6`) to `cardstructure::refs::public_refs`.
  - Preserves stack and graveyard order via normalized position channels (`f[36..38]`) and explicit `EDGE_ORDERED_NEXT` (`edge_type = 10`) chain edges.
  - Emits public commander designations (`EntityZone4::CommanderDesignation = 10`) even when a commander is in a hidden zone, and emits remaining own-library and own-sideboard inventory groups (`DeckInventory = 12`, `SideboardInventory = 13`) from `InventoryLedger7` (with `remaining_copies`, `log1p(remaining_copies)`, and `draw_probability` in `f[120..123]`).
  - Retains `last_seen_turn` age and `origin_zone` on remembered opponent cards (`f[125..128]`), tracks permanent age and `entered_battlefield_this_turn` in `SeatKnowledge4`, and deduplicates objects across shared/controlled/inspection views.
- **`features7::grouping::PermanentGroup7` and `encode_candidates7_grouped`:**
  - Losslessly groups battlefield permanents with identical full authorized semantics into single entity tokens carrying multiplicity (`f[96..99]`) and `entity_group_members`, while deduplicating equivalent priority/target/subset candidates (`group_object_ids`) and canonicalizing teacher targets (`encode_teacher_action4`).
- **Runtime, conversion, and trainer integration (`LOSS_VERSION = 3`):**
  - `NetPlayer4` dynamically dispatches between archived Encoder 5/6 (`(96, 64)`, `encode_state4` + `encode_candidates4`) and Encoder 7 (`(128, 80)`, `encode_state7` + `encode_candidates7_grouped`) based on model input dimensions.
  - `convert4` emits Encoder 7 trajectories with `step_loss_weight = 1.0` for all multi-option decisions (removing the 0.2 pass downweighting) and reuses the deciding seat's `PlayerView` via `Cow::Borrowed` instead of building it twice.
  - `SeatKnowledge4` uses zero-allocation streaming FNV-1a hashing (`Fnv1aWriter` + `ObservableView`) for `board_observation_hash` and `view_only_state_hash`.
  - `PlayerNet4` adds `cand_ctx_proj` (zero-initialized residual projection over the masked mean and max of legal candidate embeddings so each candidate sees competing menu options) and `upgrade_state_dict_to_v7` to warm-start Encoder 7 models from archived Encoder 6 checkpoints.
  - `LOSS_VERSION = 3` removes the 3x non-pass multiplier and the uncalibrated `pos_weight = 2.0` on `SubsetPick` / `PairPick` in both `training4/loss.py` and `training4/device_loss.py`.
