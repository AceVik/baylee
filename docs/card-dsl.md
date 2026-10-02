# Card DSL Cookbook (frozen at M2.S8)

**This is the authoring contract for card implementations — humans and LLMs
alike.** If a mechanic is not expressible with the vocabulary here, the card
gets `Coverage::Partial("exact reason")` and a `// NOT SUPPORTED:` comment.
Never hack around the DSL; extend the DSL instead (in a new milestone).

## File standard (one file per card)

Location: under `crates/baylee-cards/src/cards/`, in the branch the card's own
type line puts it in — `instants/mv_1/swords_to_plowshares.rs`. The next section is
the whole rule; `cargo xtask codegen` computes it, so never place or move a
card file by hand. Header is mandatory and must be kept truthful (it's the
human-verification surface):

```rust
//! Lightning Bolt — {R} — Instant
//! Oracle: Lightning Bolt deals 3 damage to any target.
//! Set: M11 #149 — Magic 2011 | Scryfall ID: <uuid> | Oracle ID: <uuid>
// IMPLEMENTED — one-line summary of the implementation.

use baylee_cards_dsl::prelude::*;
```

The first line's three segments are all checked, so none of them is prose.
The name and the cost must be some face's, and the type segment is
`pool::type_line` for each face the *file* has, joined with `" // "` — the
card's spelling and not the constants' (`Land — Swamp Mountain`, never
`Land — SWAMP MOUNTAIN`), supertypes included (`Legendary Land`). `xtask
refresh-oracle` writes the `//! Oracle:` block and the `//! Set:` line and
never this one, so it is the one line of the header a person keeps true.

One import, and it is the only one most cards need — the prelude carries the
whole vocabulary plus the macros below. Add a second `use` line only for
something outside it: a subtype module
(`use baylee_core::generated::subtypes::creature;`), a token
(`use crate::tokens::…`), or a shared filter (`use crate::filters::…`).

A *generated* card reaches a token through `use crate::generated_tokens;`
instead, which is the whole ledger under one name — the hand-written half is
re-exported from there. Either spelling reaches the same `TokenDef`, and a
hand-written card keeps naming `crate::tokens` because that is where it can
read the comment saying which printing lent the token its art.

A token the pool lacks is written into `crate::tokens` (with the Scryfall id
of a printed token card for its art) and filed in the ledger by `cargo run -p
xtask -- codegen --tables`: that half of the ledger reads only `tokens.rs`
and appends, so it needs no corpus. Never add the row to
`generated_tokens.rs` by hand (Fable of the Mirror-Breaker's Goblin Shaman
was the first filed this way).

Do **not** add `#![allow(unused_imports, missing_docs)]`. It used to be on
every card file because the generated import list was identical for every
card whether the card used it or not; it is gone, and with it the two dozen
genuinely dead imports it had been hiding.

Rules:

1. **Every oracle sentence maps to an ability/effect** or to an explicit
   `// NOT SUPPORTED: <reason>` comment near the affected ability.
2. Keep `index`/`oracle_id`/`scryfall_id`/`faces` data from the generated
   stub untouched. Only edit `coverage`, `keywords`, `abilities`. State only
   what the card prints — see *Only state what the card prints* below.
3. Declare layers + durations explicitly for continuous effects. The engine
   deregisters effects structurally — never hand-roll removal.
4. Card behaviour is tested in `crates/baylee-engine/src/engine/*_tests.rs`,
   where a card can be *played*. A card file carries no `#[cfg(test)]` module
   and none in the pool has ever had one — a test beside a `CardDef` literal
   can only re-read the literal.

## Where a card's file lives

`cards/` is a taxonomy a person can browse, not a flat list of slugs, and
every part of it is computed from what the card prints:

```text
<card type>/<second type or defining subtype>/mv_<mana value>/<slug>.rs
```

```text
creatures/artifacts/mv_2/baleful_strix.rs      Artifact Creature, {U}{B}
creatures/mv_2/orcish_bowmasters.rs            a tribe is not a kind of card
artifacts/equipment/mv_2/dowsing_dagger.rs     no second type, so the subtype
enchantments/sagas/mv_3/welcome_to.rs
instants/kindred/mv_3/crib_swap.rs             Kindred Instant
sorceries/mv_2/curse_of_the_swine.rs           {X}{U}{U} — X counts 0
planeswalkers/mv_4/jace_the_mind_sculptor.rs
lands/fetch/arid_mesa.rs                       from data/land-cycles.tsv
lands/check/glacial_fortress.rs                read off "unless you control a Plains"
lands/manlands/celestial_colonnade.rs          "becomes a … creature until end of turn"
lands/utility/bojuka_bog.rs                    does something other than make mana
lands/dual/taiga.rs                            two basic land types, and no text
lands/enchantments/urza_s_saga.rs              Enchantment Land
lands/wasteland.rs                             nothing to say about it yet
```

Four rules, all of them in `baylee-cards-codegen/src/layout.rs`:

- **One card, one home.** A total order over card types picks the door —
  Land > Planeswalker > Creature > Artifact > Enchantment > Battle > Instant
  > Sorcery > Kindred — and the next type the card has is the room inside, so
  an Artifact Creature is a creature that happens to be an artifact and is
  never in two places. Land leads because a land prints no mana cost, which
  keeps every mana-less card in the one branch that has no `mv_` level;
  Kindred trails so Crib Swap is an instant. CR 205.2a lists the types and
  states no order — this one is ours.
- **The front face decides**, whatever the layout. A transforming back is not
  a card anyone holds and a modal back is the same card from the other side
  (CR 712.2), so filing by either would give Westvale Abbey two homes.
- **A `mv_` level ends every branch but lands**, `{X}` counting 0 (CR 202.3).
- **Lands take a semantic level instead**, because a land's type line says
  almost nothing — 888 of the 1124 in the pool print no subtype at all. Six
  sources are asked in order and the first that answers wins: the `Basic`
  supertype, a second card type, the hand-kept cycle map, a printed nonbasic
  land subtype (CR 305.6), what the printed text *does*, and finally how many
  basic land types it prints.

  The two middle sources are the two halves of "semantic", and they are
  opposites. `data/land-cycles.tsv` is an **assertion no card prints** —
  nothing about Scalding Tarn's text says "fetchland" — so it is kept by hand
  and is **additive**: a land missing from it is filed one level shallower,
  never misfiled, so a stale map costs browsing and never correctness. Add to
  it freely. `layout::land_role` is the opposite: it **reads the card**,
  because "enters tapped unless you control two or fewer other lands" is a
  fastland whoever printed it. It knows about thirty cycles by their printed
  sentence — fast, slow, check, crowd, unlucky, legendary, saddle, battle,
  reveal, scry, surveil, gain, refuge, filter, pain, shock, bounce, storage,
  horizon, cycling, fetch, manlands, no_untap, restricted — and ends in the
  two shapes that are left: `utility` for a land that does something other
  than make mana, `tapland` for one whose only text is that it comes in
  tapped. A land whose whole text is a mana ability stays flat in `lands/`,
  which is the honest place for it. That reader is what took `lands/` from
  872 files in one directory to 73.

  The map's own failure mode is silent, so it is made loud: an entry naming a
  card the pool does not have is a **bail**, because the land it meant to file
  would otherwise just sit one level shallower with nobody the wiser — ten
  pathways did exactly that for a commit, the map holding their front-face
  names while Scryfall hands over `A // B`.

Two things keep the tree honest, and both are `codegen`'s:

- **The file moves; the module path never does.** `cards/mod.rs` declares
  every card with `#[path = …]`, so `cards::lightning_bolt` resolves exactly
  as it did when the directory was flat. `generated.rs`, the ledger and every
  path in the workspace are untouched by a re-filing, which costs one
  `git mv` and one generated line.
- **Placement is a reconciliation, not a write.** `codegen` finds every card
  file wherever it is, **fails on any `.rs` under `cards/` that no card in the
  registry claims**, and only then moves the misplaced ones. The refusal comes
  first because the slug a card claims is known without fetching anything, and
  a bail halfway through a re-filing would leave every card moved and `mod.rs`
  still naming the old paths — a tree that does not build, over a stray file
  someone could have deleted in a second. It is checked again after the moves,
  against the slugs the reader actually produced. An orphan left
  behind by a move would still compile, be declared by nothing and be read by
  nobody — which is exactly how an empty `lightning_bolt.rs` sat in the tree
  unnoticed. Two files claiming one slug fails for the same reason: `mod.rs`
  could only declare one of them, and which one would depend on directory
  order.

## Only state what the card prints

`CardDef` and `FaceDef` both carry a `DEFAULT` associated const, and every
card file ends its literals with a struct-update tail:

```rust
card!(
    index = index::ONDU_CLERIC,
    oracle_id = "f4232466-dd6a-49bf-be6c-95905c3ded17",
    scryfall_id = "ced43447-fefc-482a-b8fa-33b9616aa532",
    faces = &[face!(
        name = "Ondu Cleric",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[creature::KOR, creature::CLERIC, creature::ALLY],
        power = Some(1),
        toughness = Some(1),
    )],
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[/* … */],
);
```

`card!` and `face!` *are* those literals — they expand to `CardDef { … ,
..CardDef::DEFAULT }` and `FaceDef { … , ..FaceDef::DEFAULT }`. Two things
come with them: the tail can no longer be forgotten, and `card!` writes the
doc comment on the `pub static CARD` it defines. The three identity fields
are mandatory and come first, in the order codegen writes them.

**Parentheses and `=`, never braces and `:`** — in every macro in this
document, and it is rustfmt's rule rather than a house style.
rustfmt leaves a macro invoked with braces alone entirely, and `field: value`
is not an expression, so it could not format the body even if it entered it.
One knob written the old way therefore switches formatting off for the whole
call: `coverage: Coverage::Implemented` in a generator emit is what left 384
card files unformatted while `cargo fmt --check` stayed green. Written
`card!(field = value)`, every card in the pool is ordinary rustfmt output.

Never write a field back just to restate its default (`loyalty = None`,
`delve = false`, `partner = PartnerKind::None`, …) — a reviewer should be able
to read the literal as the card's printed face. Adding a field to `FaceDef`
then costs one line in `baylee-cards-dsl` instead of one line in ~200 card
files.

Two defaults are the *pessimistic* value rather than the common one, and
both are load-bearing:

- `CardDef::DEFAULT.index` is `0`, which collides with card 0. The
  `every_card_sits_at_the_index_it_claims` test in `baylee-cards` turns a
  forgotten index into a build failure instead of a card that silently
  resolves as another one.

  Where the number comes from, who may write it, and what it survives:
  `docs/card-identity.md` is normative on all of it. The short of it is that
  a `CardIndex` is an identity and not a position, assigned over every card
  there is rather than over this pool, so implementing an old card inserts
  nothing and a number is never handed to a second card. A card *names* that
  identity — `index = index::MOX_OPAL`, never `index = 11391` — and the
  macro's fragment specifier is `path`, so a bare number is refused by the
  matcher before the type system is reached (`no rules expected 240`). That
  is the same argument as `every_card_sits_at_the_index_it_claims` made one
  step earlier: a digit typed wrong used to be a card that compiled.
- `CardDef::DEFAULT.coverage` is `Coverage::Unimplemented`, so a stub that
  was never finished cannot reach the deckbuilder as playable just because
  a line went missing. An implemented card writes
  `coverage = Coverage::Implemented` by hand.

`FaceDef::DEFAULT.castable_from_hand` is `true`; disturb backs, adventure
backs and the back face of a **transforming** double-faced card opt out. The
last of those is the one that bites, because nothing in a `CardDef` says
which layout a card was printed in: an MDFC's back is a face a player may
cast, a werewolf's back is only ever reached by turning the card over (CR
712.2), and this flag is the whole of the difference. Leave it `true` on a
transformed back and the cast wizard offers that face as a *mode* at the
cost it prints — which is nothing, so Tavern Smasher was a 6/5 for {0} and
Ormendahl, Profane Prince a 9/7.

A **stub writes the line itself**, so this is only ever hand-written on a
card a reader could not finish: `stubgen::render_face` emits
`castable_from_hand = false` for any back face that prints no mana cost and
is not a land, that being what separates the two layouts — an MDFC's back, a
disturb back and an adventure all print one.
`a_back_face_with_no_printed_cost_is_never_castable_from_the_hand` in
`baylee-cards` reads the rule back out of the compiled pool and turns a
missing line into a build failure. Reading *nightbound* instead, which is
where that test started, guards the werewolves and nothing else.

### Two faces that disagree

Three `FaceDef` fields exist for cards whose faces are not the same card,
and all three are hand-written — the Scryfall payload codegen reads carries
none of them per face:

- `keywords`. `CardDef::keywords_for_face` gives face 0 the card-level set
  when the face states none of its own, and gives a back face **only** what
  it prints. So an ordinary card still writes one `keywords =` line, and a
  transforming card writes one per face — including the keyword both faces
  share, which is written twice on purpose. Daybound is printed on a front
  face and nightbound on a back one (CR 702.145a), and a card that stated
  either for the whole card would be a permanent that turns over at night
  and turns back in the same breath.
- `color_indicator` (CR 202.2e). A face with no mana cost has nothing else
  to say what colour it is; Dire-Strain Brawler is green only because of the
  dot printed on it.
- `castable_from_hand`, above.

### Rooms: two doors, one card

A Room (CR 709.5) is a split card with one shared type line: each half is a
face, and each face writes its own `abilities`. The card-level `abilities`
is both halves' lists end to end, the left first, which is what the
permanent has with both doors unlocked; `CardDef::door_abilities(unlocked)`
reads the right list for each door state, and
`lints::a_rooms_card_list_is_its_doors_lists_end_to_end` holds the union
equal to the halves. `CardDef::has_shared_type_line` is how the engine knows
a card is a Room: two faces, both with the `Room` subtype.

"When you unlock this door" is `Trigger::UnlockThisDoor(n)`, written on the
half that prints it with that half's number (0 the left, 1 the right). It
hears the half being unlocked however that happens: the Room entering cast
as that half (CR 709.5d, 709.5h), or its controller paying the half's mana
cost later (CR 709.5e). Nothing else is written for the door mechanic; the
engine does the rest from the faces.

`Modifier::ExileInsteadOfYourGraveyard` is Forgotten Cellar's "if a card
would be put into your graveyard from anywhere this turn, exile it
instead", made by the trigger as
`Effect::continuous(&Filter::Any, Modifier::ExileInsteadOfYourGraveyard,
Duration::UntilEndOfTurn)`. It is its controller's, cards only (a token or a
spell copy dies as usual), and it lasts its duration, not as long as a
source. The same trigger makes the Cellar's first clause, "you may cast
spells from your graveyard this turn", the same way and before it:
`Modifier::CastSpellsFromGraveyard`, one of the graveyard permissions
`casting::graveyard_cast_permission` reads.

## Generated cards, and why they may say `Implemented`

Most card files are hand-written. Two of them are not, and the distinction
matters when you open one:

- **Lands** are read from their printed text by
  `crates/baylee-cards-codegen/src/landgen.rs`.
- **Everything else with a local card-script reference script** is read by
  `crates/baylee-cards-codegen/src/scriptgen.rs` (the checkout is an
  automated lookup, never copied and never part of the build).

Both write the *same* file standard as this document describes: the macros,
one `use baylee_cards_dsl::prelude::*;`, the `//!` header `xtask validate`
checks, and `Coverage::Implemented` only when the reader consumed **every**
clause of the card. An unknown effect, a parameter no rule claimed, a
computed amount, a keyword that is data rather than a bit — any one and the
whole card is refused and generated as an ordinary stub instead. There is
deliberately no partial path: a card that claims `Implemented` while quietly
dropping "and they can't be regenerated" is worse than no card, because the
deckbuilder offers it as playable.

### Who owns a card file

Two markers, and the file itself says which it is:

- `// GENERATED STUB` — nobody has finished it. **Machine-owned.**
- `// IMPLEMENTED — generated by xtask codegen: …` — a reader wrote it in
  full. **Machine-owned.**
- neither — a person wrote it, or adopted it. **Hand-owned.**

`codegen` rewrites every machine-owned file on every run and never touches a
hand-owned one. So **a generated card that is wrong is fixed in the reader,
never in the file**: `landgen` and `scriptgen` each wrote hundreds of cards, so
a rule that got one wrong got every card that rule reached wrong, and patching
the one file in front of you leaves the other ninety-nine broken *and* the
patch is reverted on the next run. Fixing the reader corrects them all at once
and is the only edit that survives.

The way out is `cargo run -p xtask -- adopt --name "<card>"`. It strips the
ownership marker and the file is yours from then on, like any card written
from a stub. Reach for it when the card genuinely needs something the reader
cannot say — not to get past a transcoding bug, which belongs in the reader
where it also fixes the cards you have not looked at.

For an unfinished generated stub, use
`cargo run -p xtask -- codegen --adopt-stub "<card>"` before editing it.
This transfers ownership only: the card stays unimplemented until its rules
and tests are written. It preserves the generated identity and printing data
and refuses finished or already hand-owned cards. Do not remove a generated
marker by hand.

`xtask validate` reports the split, which is the number to watch: **1365
cards, 590 finished — 207 hand-owned, 383 machine-owned — and 775 stubs.**

It also holds a card against its **printing** — Scryfall's own payload in
`data/scryfall-cache`, which is tracked, so a fresh checkout checks exactly
what CI does. Four comparisons, and each is a mistake a header cannot catch
because a header is the other thing a person wrote: the front face's cost,
P/T and starting loyalty; the card's colour identity (CR 903.4, which counts a
mana symbol in the rules text — a `{4}` artifact that taps for `{U}` is blue);
every keyword **bit** the card claims, which must appear as its own word in
the printed text; and the colours its mana abilities offer to make, which must
appear in an "Add" clause. The last two are read out of the oracle text rather
than out of Scryfall's `keywords` and `produced_mana` arrays, because the
cache holds a trimmed `ScryfallCard` that has neither.

Each comparison skips quietly when the card has nothing to compare, so the
command ends by printing how many it actually made and failing if that falls
below a floor. A checker that has silently stopped checking reports a clean
pool, which is the one failure a card gate must not have.

The reach today is **all 1365**. It was 1263, and the gap was a slug rather
than missing data: a double-faced card is cached under both of its face names
(`agadeem_s_awakening_agadeem_the_undercrypt.json`) while the pool names its
file after the front face alone, so three separate copies of the same four
lines built `{slug}.json` and found nothing for every double-faced card in
the pool. `cached_printing` is the one lookup they now share.

What the other 102 brought with them is the reason to care: five cards where
a Town or a Land is printed in front of a spell, reported as costing nothing
in the printing and `{3}{W}{W}` in the code. The cards were right. A costless
face writes no `mana_cost` line at all — never restate a default — so reading
the file's mana costs as a list of the lines it *writes* handed the front face
the back face's cost, and the list is read per `face!` block now.

If you want more cards generated, the lever is usually **this document's
vocabulary**, not the readers. `cargo run -p xtask -- transcode-report` ranks
what the corpus is waiting on, and the top entries are effects the DSL cannot
express at all yet.

## The vocabulary

### Card faces & costs

- `mana_cost = mana!("{2}{W/U}{W/P}")` — compile-time parsed (generic,
  color, hybrid, 2-or, Phyrexian, hybrid-Phyrexian, snow, X/Y/Z).
- `FaceDef.alternative_costs: &[AlternativeCost { cost, condition }]` —
  pitch/evoke/conditional-free (conditions: `Always`, `NotYourTurn`,
  `CommanderControlled`). Its `parts` pay `PayLife` and `ExileFromHand`, and
  both are asked about before the card is offered: a pitch with nothing in
  hand to exile is not a cast the engine lists.
- `FaceDef.additional_costs: &[Cost]` — kicker (optional, yes/no at cast).
  Only its `mana` is read; `parts` is paid by nothing and is held empty.
- `waterbend = true` — "you may waterbend {N}": the additional cost above is
  a waterbend cost, so once it is paid, artifacts and creatures may be tapped
  for its generic mana and nothing else (CR 701.67b). It is not
  `convoke = true`, which taps creatures only, for the whole cost, and
  whether or not anything was kicked. `lints::waterbend_fault` holds the
  shape: `{N}` alone, no convoke beside it.
- `replicate = Some(mana!("{U}"))` — "Replicate {U}" (CR 702.56a), Lose
  Focus: an additional cost the caster may pay any number of times. The cast
  asks how many after the kicker and before the targets, and the spell's cast
  trigger copies it once per payment, each copy with the chance of new
  targets. Nothing else is written for it: the trigger is the engine's, not
  an ability on the face. A mana cost only, as every printed replicate cost
  is.
- `FaceDef.mandatory_additional_costs: &[CostPart]` — e.g. `PayLifeX`. Pays
  `PayLifeX` and `PayLife`, and is the one cost list nothing gates at all.
  `PayLifeX` is bounded where it is asked instead — the wizard offers X up to
  the caster's life total (CR 119.4) and never up to a constant. A `PayLife(n)`
  written here is bounded by nothing and would be paid past zero.
  `Sacrifice(filter)` here is "as an additional cost to cast this spell,
  sacrifice a …" (Crop Rotation, Natural Order): the cast wizard's
  `Sacrifice` stage asks which one with `ChoicePrompt::CostSacrifice`, the
  spell is not castable while nothing can pay it, and the sacrificed
  permanent's mana value is written on the spell for
  `Amount::SacrificedManaValue`.
- `FaceDef.flashback: Option<ManaCost>` — a printed "Flashback {…}" (CR
  702.34a), mana only. From the owner's graveyard the cast is offered as
  `CastModeKind::Flashback` at that price (beside a grant's `Normal` at the
  mana cost, when there is one), and the spell is exiled afterwards.
  `validate` holds it against the printing. What the cast paid is
  `Amount::ManaSpentToCast` (Memory Deluge).
- `Amount::TargetsPutIntoGraveyard` — "the number of Mountains put into a
  graveyard this way" (Volcanic Eruption): the resolving ability's targets
  that a graveyard holds as new objects since the resolution began, so a
  regenerated target, one exiled instead, or one dropped as illegal is not
  counted. Written after the effect that moves them.
- `Amount::CreaturesDiedThisTurn` — "for each creature that died this turn"
  (Scavenging Ghoul): every player's creatures put into a graveyard from the
  battlefield this turn (CR 700.4), each counted if it was a creature as it
  left, read as the effect applies (CR 608.2h). The reader writes it for
  `Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature` wherever it
  reads an amount, an `etbCounter` included.
- `Amount::UntappedLandsAtTurnStart` — the untapped lands controlled by the
  current active player at the turn boundary, before untapping or phasing.
  It reads a saved count, so later taps, arrivals and control changes do not
  change it. The engine records it even without a source in play and when
  the untap step is skipped. Power Surge needs this vocabulary; its card
  implementation and reader mapping remain pending.
- `Amount::DamageDealtToYouThisTurn` — "the damage dealt to you this turn"
  (Simulacrum): every point dealt to the ability's controller since the turn
  began, combat or not, counted where the damage is dealt
  (`GameState::damage_player`). Damage, not life lost: a payment is not in
  it, a gain does not take it back, and a player whose life can't change is
  still dealt it; prevented damage never was. The reader writes it for
  `PlayerCountPropertyYou$DamageThisTurn`.
- `cost!("{1}{G}", TapSelf, SacrificeSelf)` — a cost, read left to right the
  way the card prints it: the mana string first (omitted when there is none),
  then the parts. A part is named without its `CostPart::` prefix, which on a
  fetchland was the same word three times.

  The parts: `TapSelf`, `UntapSelf`, `SacrificeSelf`, `Sacrifice(filter)`,
  `Discard(filter)`, `DiscardSelf` (cycling), `PayLife(n)`, `PayLifeX`,
  `ExileSelf`, `ExileFromHand(filter)`, `ReturnSelfToHand`,
  `TapOther(filter)` (the convoke lands, Earthcraft),
  `Crew(n)` (written only by `crew!(n)`: one question answered with any
  number of other untapped creatures the payer controls, refused when their
  total power is short of `n`),
  `ReturnToHand(filter)` (Quirion Ranger's Forest — one permanent, and
  nothing in it says "untapped": tapping the land for mana and *then*
  returning it is the play, and the mana stays in the pool),
  `ExileFromGraveyard(filter)` (Moorland Haunt's creature card — one card out
  of **your own** graveyard, the zone saying whose, so the filter says only
  what kind of card; Mines of Moria's "three cards" is the part written three
  times, one question each, the way Time Sieve writes five sacrifices),
  `RemoveCounterSelf { kind, n }` (the Vivid lands, Tendo Ice Bridge,
  Scavenging Ghoul's `counters::CORPSE`; the reader writes it for
  `SubCounter<n/KIND>` with a fixed `n`, never for a loyalty cost or one
  that names where the counters come from),
  `RemoveCounterSelfX { kind }` (the storage lands: a number the player
  chooses as the ability is activated, bounded by the counters on the source
  and allowed to be zero, which the effects read back as `Amount::X`),
  `PutCounterSelf { kind, n }` (Devoted Druid: a counter put *on* the source,
  never refused — a permanent can always take one — and never doubled,
  because CR 614.16 gives a counter-doubling replacement the effects of
  resolving spells and abilities and not costs).

  A part with **named fields** keeps its braces —
  `cost!(TapSelf, RemoveCounterSelf { kind: CounterKind::Charge, n: 1 })` —
  which is the third shape the macro reads after "a word" and "a word with
  parentheses". The order of the parts is the printed order and is
  load-bearing: `docs/cost-model.md` has the rule and the lint that holds it.

  A **mana `{X}`** in an activation cost is the other half of that same
  announcement and needs nothing written beside it: `cost!("{X}{G}")` is
  asked for at CR 602.2b exactly as the counter part is, bounded above by
  what the pool can pay rather than by what is on the permanent, and read
  back by the effects as the same `Amount::X`. So Kessig Wolf Run's
  `+X/+0`, Treasure Vault's X Treasures and Blast Zone's X charge counters
  are ordinary cards. Two things it still cannot say: a **lower** bound, so
  Lair of the Hydra's "X can't be 0" is a `Coverage::Partial` reason, and
  *both* kinds of X in one cost, because one announcement is held in one
  field — `lints::no_cost_announces_two_different_xs` is the guard, and it
  fails with the card's name the day one prints both.

  A **Phyrexian symbol** in an activation cost needs nothing written beside
  it either: `cost!("{1}{G/P}", …)` (Birthing Pod) is offered when some way
  of paying each symbol — its colour or 2 life (CR 107.4f) — covers the
  cost, and the engine asks `YesNoPrompt::PayLife { amount: 2 }` per symbol
  only where both ways pay, after the X and before the targets (CR 601.2b
  through CR 602.2b). A *spell's* Phyrexian symbol is still paid with its
  colour only: the cast wizard does not offer the life.

  **Which counter** is a `CounterKind`, and there are three ways to name one.
  Nine counters have a variant because the rules know them by a word —
  `Loyalty`, `Lore`, `Time`, `Charge`, `Poison`, `Energy`, `Rad`,
  `Lifelink`, `Level`. Every counter that changes power and toughness is
  `Plus { power, toughness }` or `Minus { power, toughness }`, one variant
  for each of CR 122.1a's two forms, because the rule is one rule over an
  open-ended set of pairs — Magic prints eleven of them and a name apiece
  would go silent on the twelfth. `CounterKind::P1P1` and
  `CounterKind::M1M1` are **constants** for the two Magic prints everywhere:
  the same value, spelled the way a player says it, so a card writes
  `CounterKind::P1P1` and never `Plus { power: 1, toughness: 1 }`. Only
  those two are annihilated against each other (CR 704.5q); a -0/-1 and a
  +1/+1 both stay.
  Every other counter Magic prints is a word and a
  number the rules have never heard of, and those are `CounterKind::Custom`
  ids **assigned in `baylee_cards_dsl::counters`**: a card writes
  `counters::DEPLETION`, never a bare `CounterKind::Custom(2)`. Adding one is
  a constant with a doc comment naming the printed word; a bare number is the
  collision the module exists to prevent, and the prelude carries `counters`
  the way it carries `index`. Two lints hold the pool to it:
  `every_custom_counter_in_the_pool_is_an_assigned_id` (the compiled pool
  carries no id the registry does not name) and
  `no_card_file_spells_a_counter_id_as_a_number` (no card writes one).

  `Cost::FREE` is the empty cost and `Cost::TAP` a bare `{T}` — the two the
  macro would spell with no argument and one, and between them what two
  thirds of the pool's activated abilities cost (444 of 644 today, 432 of
  them written as the one-argument `mana_ability!`). `Cost { mana, parts }`
  is still the struct underneath and is what a *reader* in
  `baylee-cards-codegen` builds; card files say `cost!`.

**Two** of those may not appear on an **activated** ability, and one that
carries them fails the build rather than shipping. `ExileFromHand(filter)` and
`PayLifeX` are paid in the cast wizard, which an activation never enters, so
the ability *is* offered and the part is silently skipped — a pitch cost that
exiles nothing, and an activation that succeeds, which is why no test of an
action can fail on it. `nothing_in_the_pool_carries_an_activated_cost_the_engine_would_skip`
in `baylee-engine/src/engine/offer_tests.rs` is the pool-wide guard.

There is no way out of that one and `Coverage::Partial` is not it: a partial
card is offered as playable and dealt into real decks, so the label would ship
the pitch cost that exiles nothing rather than excuse it. The ability comes
**off** the card, leaving a `// NOT SUPPORTED:` line to say what was dropped.

`Sacrifice(filter)`, `Discard(filter)` and `TapOther(filter)` used to be on
that list and are not any more, and `ReturnToHand(filter)` and
`ExileFromGraveyard(filter)` were written after it was already gone. They name something to choose, an activation
had nowhere to ask, and `can_afford` refused them outright — so the two guards
that stood beside the one above (`no_implemented_card_hides_an_ability_the_engine_will_never_offer`
and its token twin) were about a *limitation* rather than a rule. `cost_wizard`
is the limitation's end: the engine suspends the activation on a
`Pending::ChooseCards`, the player answers, and `can_afford` asks the same
`cost_wizard::options` the player is about to be shown — so the cost is
refused only on a board that really has nothing to pay it with. Those two
guards are gone with it, and Viscera Seer, Krark-Clan Ironworks, Survival of
the Fittest and Recurring Nightmare are all `Coverage::Implemented`.

On a **spell** the parts are not interchangeable either, because a spell has
three cost lists and no two of them are paid by the same code. Each pays
exactly what its bullet above says and walks past the rest, so a part written
on the wrong list is a spell cast without paying it —
`offer_tests::no_spell_cost_list_carries_a_part_its_payment_walks_past` is the
build failure, and `cast_wizard`'s two predicates are what it reads, so the
guard and the payment cannot drift. `Sacrifice(filter)`, `Discard(filter)`,
`TapOther(filter)`, `ReturnToHand(filter)` and `ExileFromGraveyard(filter)` are paid on **no** list at all: an alternative cost is the
one list `can_afford` gates, so writing one there is refused rather than
skipped — a dead offer instead of a free spell, which is not an improvement
worth having either. `cost_wizard` does not reach here, and that is the line
between the two halves: an *activation* can suspend on a question, a **cast**
already has a wizard of its own and a second one inside it is a stage nobody
has built.

### Ability kinds

- `AbilityDef::Spell { effects, targets: Option<TargetReq> }`
- `AbilityDef::Activated { cost, effects, targets: Option<TargetReq>, timing, mana_ability, zone }`
- `AbilityDef::Triggered { trigger, effects, targets, once_per_turn }`
- `AbilityDef::Static(StaticAbility { layer, filter, modifier })` — written
  `static_ability!(filter, modifier)`, which takes no layer: CR 613.1 makes it
  a function of the modifier and `Modifier::layer` is that function
- `AbilityDef::Replacement(ReplacementRule)` — trigger multipliers/suppressors,
  token/counter doubling
- `AbilityDef::Loyalty { cost: i8, effects, targets }`
- `AbilityDef::CopyOnEnter { target, mods: &[CopyMod] }` — the `mods` are the
  card's "except …" clauses (CR 707.9). Types, supertypes, subtypes, keywords
  and entry counters are all sayable. A counter that depends on what the copy
  is ("…an additional +1/+1 counter on it if it's a creature") is
  `CopyMod::AddCounterIf(TypeSet::CREATURE, CounterKind::P1P1, 1)`, asked of
  what the permanent became (the copied values with the clause list's own
  type changes, CR 707.2, 707.9b) and never of the copier's printed types;
  a plain `AddCounter` would put it on every copy. "Except it has its
  **other** abilities" is sayable too (`CopyMod::KeepOtherAbilities`, CR
  707.9a), with one limit worth
  knowing before writing a card on it: the kept abilities are registered as
  the copy's own continuous effects, so a **static** survives and a triggered
  or activated one does not. A card that needs the second keeps a
  `Coverage::Partial`, and the test
  `combo_tests::every_copy_that_keeps_its_own_abilities_keeps_only_statics`
  will say so rather than letting it through. "Except it has '[quoted
  ability]'" is `CopyMod::Grant(&Modifier::GrantActivated { … })` or
  `CopyMod::Grant(&Modifier::GrantTriggered { … })`, and the quoted ability
  goes **inside** the clause, never beside it: every ability printed beside
  a copy ability is overwritten when the copy is made (CR 707.2), so a
  sibling written for the quotation is gone exactly when it is needed.
  Machine God's Effigy, Progenitor Mimic and Phantasmal Image all shipped
  that way. A card that also prints the ability as its own (the Effigy's
  `{T}: Add {U}`, for when it enters as itself) writes both. The grant is
  the copy's and is not copiable, so a second clone of it does not inherit
  it
- `AbilityDef::ModalSpell { modes: &[SpellMode] }` — overload & friends
- `AbilityDef::ModalTriggered { trigger, modes, once_per_turn }` — "choose
  one/up to one" ETB triggers (decline = an empty mode)
- `AbilityDef::Ward { mana }` — engine-level synthetic trigger (like
  prowess), supports generic mana ward from {0} through {10}. For a
  variable life payment, use `Trigger::Ward` with `PlayerMayPayLifeOr`,
  `PlayerRel::ControllerOfTarget`, and `CounterTargetSpellOrAbility`.
  The implicit subject is the offending stack object; it is not a chosen
  target. `Amount::SourcePower` is read at resolution, using last known
  power if the source left while the ability waited. See Phyrexian
  Fleshgorger, including its prototype form. Temporary ward uses
  `Modifier::GrantTriggered` with `Trigger::Ward` (Hall of Storm Giants).
  Each instance triggers separately, including on copies and retargeting;
  teammates are not opponents. **Undying** and **persist** are the same
  shape one step further along: they are not an `AbilityDef` at all but two
  bits on `keywords`, and `trigger.rs` reads them the way it reads prowess.
  A card prints one of them by setting the bit and writing no ability —
  `keywords = KeywordSet::UNDYING` is the whole of Young Wolf
- `AbilityDef::Toxic { poison }` — toxic N (CR 702.164), a static ability
  with a number, so data like ward and not a bit. The engine reads it where
  combat damage is dealt (`Engine::deal_combat_damage`): each journalled
  combat `DamageDealt` to a player gives that player poison counters equal
  to the source's total toxic value, summed over every `Toxic` it has
  (CR 702.164b). Tyrranax Rex
- `KeywordSet::SPLIT_SECOND` — split second (CR 702.61) is a bit and no
  ability: while a spell whose projected keywords carry it is on the stack,
  `Engine::compute_legal` offers no spell, no suspend and no activation but
  mana abilities and turning a face-down permanent up (CR 702.61b, 116.2b).
  Triggers still trigger. Krosan Grip
- `KeywordSet::BANDING` — banding (CR 702.22) is a bit; the engine asks the
  band as attackers are declared and who divides combat damage where a band
  is involved (`docs/engine-internals.md` §"Bands, and who divides combat
  damage"). Benalish Hero; granted like any keyword, Helm of Chatzuk's
  `PumpTarget { keywords: KeywordSet::BANDING, .. }`. "Bands with other"
  (702.22b) names a quality a bit cannot carry: the reader refuses
  `K:Bands with Other`, and such a card stays unread
- `AbilityDef::Suspend { counters }`

#### Write them through the macros

The six shapes that make up most of the pool have a macro that supplies the
fields the rules already imply, so an ability states what the card says and
nothing more:

```rust
mana_ability!(&[Effect::mana(ManaColor::Green, 1)])   // {T}: Add {G}
mana_ability!(SAC_COST, ANY_COLOR_MANA)               // any other cost
activated!(Cost::TAP, EFFECTS)                        // {T}: …
activated!(COST, EFFECTS, target = Some(TargetSpec::Object(&Filter::CREATURE)))  // target creature
activated!(COST, EFFECTS, targets = Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::CREATURE))))
activated!(EQUIP, EFFECTS, timing = ActivationTiming::SorcerySpeed)
mana_ability!(COST, EFFECTS, limit = ActivationLimit::PerTurn(1))  // "only once each turn"
triggered!(Trigger::ETB, EFFECTS)                     // when this enters
triggered!(UPKEEP, EFFECTS, condition = Some(Condition::SourceMatches(&Filter::Tapped)))
spell!(EFFECTS)
spell!(EFFECTS, targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))
loyalty!(-3, EFFECTS, targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))
static_ability!(Filter::YOUR_CREATURE, Modifier::ModifyPT(1, 1))  // an anthem
chapter!(1, EFFECTS)                                  // one chapter of a saga
equip!("{2}")                                         // Equip {2}
crew!(2)                                              // Crew 2
modal_triggered!(TRIGGER, &[mode!(SCRY), mode!(LIFE)])  // "choose one" ETB
mode!(DRAW_EFFECTS)                                   // one arm of a modal
```

`TargetReq::one` and `TargetReq::up_to_one` take a `TargetSpec`, never a
`&Filter` — a target is an object, a player, a spell or a card in a
graveyard, and the filter is only how an *object* target is picked.

An activated ability takes its target two ways, and a card writes one of
them. `target = Some(spec)` is the printed singular, "target creature",
which is exactly one (CR 115.1c) and is how nearly every activated ability
in the pool prints it. `targets = Some(TargetReq::…)` is a printed count:
`up_to_one` for "up to one target", `up_to(spec, n)` for "up to n",
`exactly(spec, 2)` for Wintermoon Mesa's "two target lands". Writing both
fails to compile. An "up to" is offered on a board with nothing to name, and
answered with nothing it still resolves (CR 115.6). An effect that reads
the target then reaches nobody, and a counter does not fall back onto the
source the way an untargeted "put a counter on it" does.

The required arguments come first and positionally, because they are the
ones an ability cannot be written without; everything after them is
`field = value` in any order, and anything left out takes its rules default:

| field | default | why that is the rules answer |
| --- | --- | --- |
| `timing` | `InstantSpeed` | CR 117.1b — unless the card restricts it |
| `mana_ability` | `false` | CR 605.1 makes it the exception |
| `zone` | `Battlefield` | CR 113.6 |
| `target` / `targets` | `None` | an ability targets only when it says "target"; `target` is exactly one, `targets` is a printed count |
| `once_per_turn` | `false` | a trigger fires on every occurrence |
| `condition` | `None` | most triggers print no intervening `if` |

`mana_ability = false` is the load-bearing one: an ability wrongly marked
`true` would silently skip the stack, and no test would read that as a rules
bug. That is why it is a default you have to opt *out* of, and why a mana
ability gets its own macro rather than a flag.

Three of them go one step further and drop a field the card never decided.

`static_ability!(filter, modifier)` has **no layer**, because CR 613.1 makes
the layer a function of the modifier: "all permanents are artifacts" is layer
4 whatever a card says. `Modifier::layer` is that function, and it is a
measurement rather than a preference — over the whole compiled pool, 108
`layer`/`modifier` pairings used 25 modifiers and put no modifier on two
different layers. Note that a `StaticAbility` owns its `Filter` **by value**,
where an `Effect::CreateContinuousEffect` borrows one.

`equip!("{2}")` takes **only the cost**, because CR 702.6 supplies the rest:
sorcery speed, "target creature you control", and attaching this permanent to
it. Every Equipment in the pool had written those eight lines by hand, with
that target named twice over a local `static` that was the same filter each
time.
Equip {0} is `equip!(Cost::FREE)` and not `equip!("{0}")` — a cost with no
mana cost is not the same data as a mana cost of zero generic.

`crew!(2)` takes **only the number**, for the same reason (CR 702.122a):
the cost is `CostPart::Crew(2)` alone, and the effect makes the source an
artifact creature until end of turn. The crew question is a
`Pending::ChooseCards` with `ChoicePrompt::CostCrew { power }`, `min: 1` and
`max` every creature offered; `apply` refuses an answer whose total power is
short, and `can_afford` offers the ability only when the creatures with a
power above zero reach the number (Unlicensed Hearse).

**An Aura's "enchant …" clause is one `spell!` and has to be**, because the
engine reads the card's continuing legality out of it and out of nowhere
else:

```rust
spell!(
    &[Effect::AttachSelf { target: TargetSpec::Object(&ENCHANTABLE) }],
    targets = Some(TargetReq::one(TargetSpec::Object(&ENCHANTABLE)))
)
```

Enchant is a static ability of the Aura *spell* (CR 702.5b): it says what the
spell targets as it is cast (CR 303.4a), and the Aura arrives already
attached to that permanent. The same sentence is also a standing
restriction — CR 303.4c puts an Aura enchanting an **illegal** object into
its owner's graveyard, and "illegal" is that filter's word rather than
"gone" — so the attachment state-based action reads the filter back off this
effect's `TargetSpec::Object`. Write the clause any other way (an ETB trigger
that attaches, a `TargetSpec::AnyTarget`) and the engine finds no
restriction: the card still plays, and its Aura sits on a host it may not
legally enchant for as long as that host is on the battlefield. An Aura on a
*player* has no shape here at all yet, because `Effect::AttachSelf` reads the
resolution's first target as an object.

An Equipment states nothing of the kind: CR 301.5b attaches it to a creature,
so the same state-based action asks that of the rules rather than of the
card — and answers it differently, because a host that stops being a creature
leaves the Equipment unattached on the battlefield (CR 704.5n) where it
destroys the Aura.

A creature is never attached to anything (CR 704.5p, first sentence): an
Equipment an effect animates comes off its creature and stays on the
battlefield. Reconfigure's reminder text is what keeps its Equipment on:
"While attached, this isn't a creature" (CR 702.151b) is
`static_ability!(Filter::This, Modifier::RemoveType(TypeSet::CREATURE),
condition = Some(Condition::SourceMatches(&Filter::IsAttached)))`, written
beside the `equip!` that is reconfigure's attaching half. Without it the
card attaches and falls off again at the next state-based check.

`activated!` and `mana_ability!` reach `AbilityDef::ActivatedConditional`
through one optional field, `condition = Some(Condition::…)`. That
is the only difference between the twins, which is exactly what makes them
easy to confuse: six readers across the engine, the client and the pool lints
once matched `AbilityDef::Activated` alone and skipped every conditional
ability there was.

`Condition` is the shared vocabulary for "only while this is true" and is
not activation-specific — it was called `ActivationCondition` after its one
reader. `ControlCount(&filter, n)` is metalcraft and the verge lands,
`ControlDistinctNames(&filter, n)` counts names rather than permanents
(Field of the Dead's "seven or more lands with different names"),
`OpponentGraveyardCountAtLeast(n)` is Sheoldred's flip,
`CountersOnSelf(kind, n)`, `CountersOnSelfExactly(kind, n)` and
`CountersOnSelfBetween(kind, lo, hi)` read the permanent the ability is
printed on, `SourceMatches(&filter)` points a
filter back at that permanent — "if this land is tapped" —
`DuringCombat` is "activate only during combat": the combat phase of any
turn (CR 506.1); the reader writes it for `ActivationPhases$
BeginCombat->EndCombat` and refuses the other phase spellings by name.
`CanSacrifice(&filter)` is whether you control a permanent the filter
matches with the source as its `This` (Lord of the Pit's "sacrifice a
creature other than this creature. If you can't, …", an `IfCondition`
around `SacrificeFilter`; `ControlCount` asks each permanent with itself as
`This`, so `Filter::Another` never matches there), and
`Any(&[..])` holds while **one** of the conditions it names does, and
`Not(&c)` while `c` does not — the printed "unless". One reader answers
all of them, `eval::condition_holds`.

`CitysBlessing` is "you have the city's blessing" (CR 702.131). A permanent
with ascend carries `KeywordSet::ASCEND`, and the engine gives its controller
the blessing whenever they control ten or more permanents
(`GameState::award_citys_blessings`, asked where enduring stories are).
Wayward Swordtooth's "can't attack or block unless you have the city's
blessing" is `static_ability!(Filter::This,
Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)),
condition = Some(Condition::Not(&Condition::CitysBlessing)))`: `CANT_ATTACK`
is the mirror of `CANT_BLOCK`, read by `combat::can_attack`.

There is no `All`, and that is not an omission. The printed sentence that
needs a disjunction is real and prints as one — "activate only if this land
entered this turn or you control a basic land" is Gathering Place, Gleaming
Bastion and Hidden Lair. A conjunction is printed too, but inside an
effect: the Urza lands' "if you control an Urza's Mine and an Urza's
Power-Plant, add {C}{C}{C} instead" is two nested `Effect::IfCondition`s,
one per land type. Add `All` the day a card prints one where a single
condition is all there is room for — an activation restriction or an
intervening `if` — not before, so that every variant here stands for a
sentence somebody printed.

`triggered!` and `modal_triggered!` take the same vocabulary as
`condition = Some(…)`, and there it is the printed **intervening `if`**
(CR 603.4): the `if` that stands between the trigger event and the effect.
Write it there and the rule that comes with it is free — the ability does
not trigger at all while the clause is false, and one that did trigger is
removed from the stack and does nothing if the clause has stopped being
true by the time it would resolve. The two `if`s are easy to confuse and
the difference is where the word sits on the card:

```text
At the beginning of your upkeep, if this land is tapped, put a counter on it.
                                 ^ intervening: condition = Some(…)
When this creature dies, if it had a +1/+1 counter on it, draw a card.
                         ^ intervening: condition = Some(…)
When this enters, choose one — if you control a Forest, …
                               ^ inside the effect: an Effect, not a condition
```

A reference script writes the first kind as `IsPresent$` / `PresentDefined$`
on the `T:` line, and the second as a condition inside the `SVar` chain.

**A station symbol is not an `if`.** `Condition::Station(n)` is `{N+}` on a
station card: "as long as this permanent has N or more charge counters on
it, it has [abilities]" (CR 721.2a), and every ability printed in that
striation carries it (CR 721.2) — on Inspirit, Flagship Vessel both "Flying"
and "Other artifacts you control have hexproof and indestructible" are 8+.
It counts what `CountersOnSelf(CounterKind::Charge, n)` counts and is asked
differently: on an activated ability it gates activation, on a
`static_ability!(filter, modifier, condition = Some(Condition::Station(n)))`
the effect exists only while it holds (the engine registers and removes it),
and on a triggered ability it decides whether the ability triggers and is
**not** asked again on resolution, because the ability on the stack no
longer depends on its source (CR 113.7a).

**Station itself is a cost, not a target.** "Station" is "Tap another
untapped creature you control: Put a number of charge counters on this
permanent equal to the tapped creature's power. Activate only as a sorcery"
(CR 702.184a), and it is spelled that way:
`activated!(cost!(TapOther(&Filter::ANOTHER_CREATURE_YOU_CONTROL)),
&[Effect::AddCounter { kind: CounterKind::Charge, amount:
Amount::TappedPower }], timing = ActivationTiming::SorcerySpeed)` (Evendo,
Waking Haven). The cost wizard asks which creature with
`ChoicePrompt::CostTap`, `pay_cost` writes it on the ability
(`PaidRecord::tapped`), and `Amount::TappedPower` reads its power as the
effect applies, or as it last existed on the battlefield when it has left by
then (CR 608.2h). A station written with a `targets` requirement is wrong
twice: shroud would stop it (CR 702.18a; hexproof would not, since the
creature is your own, CR 702.11b), and a creature killed in response would
fizzle the ability instead of counting. A third time as a `TapTarget` effect
under a free cost, which is how Inspirit and the Enterprise-D were written
until 2026-09-30: a tapped creature was a legal target, where CR 118.3 says a
tapped creature cannot pay. `lints::every_station_is_the_ability_its_keyword_spells`
holds every ability whose printed sentence is a station to the shape above.

**A level symbol** is the same shape with a range. `{LEVEL N1-N2}` is
`CountersOnSelfBetween(CounterKind::Level, n1, n2)` (CR 711.2a) and
`{LEVEL N3+}` is `CountersOnSelf(CounterKind::Level, n3)` (CR 711.2b); each
ability and the P/T box in the striation is its own
`static_ability!(Filter::This, …, condition = Some(…))`, the P/T box as
`Modifier::SetPT` ("base power and toughness"). Level up itself is the
activated ability CR 702.87a spells out. Hexdrinker is the model.

The vocabulary is the five sentences listed above and nothing else. A clause
it cannot say yet is a `Coverage::Partial` with the reason written out, never
a variant invented at the card.

`limit = ActivationLimit::PerTurn(n)` is "activate only once each turn" and
its cousins — the default is `Unlimited`, because CR 602.2 caps an
activation by nothing but its cost. It is a limit per **permanent** and per
turn, not per card and not per *your* turn: two Wall of Roots each get their
own, and a Wall used on your turn is available again on the opponent's.
There is no per-*game* variant; the cards that want one print the exhaust
keyword, which other cards look for ("whenever you activate an exhaust
ability") and which is therefore a keyword bit rather than a number.

`cost_reduction = Some(CostReduction::PerCount { amount, each })` is "This
ability costs {`each`} less to activate for each …" (Boseiju, Who Endures:
`amount` is an `Amount::CountOf` over the battlefield whose filter says
`ControlledByYou`). The default is `None`. The engine reads it once, as the
total cost is determined (CR 601.2f through CR 602.2b), for the offer and
the activation alike, and it takes generic mana only, never below {0}
(CR 118.7a). `FaceDef::cost_reduction` is the same enum for a spell, so
"this spell costs {1} less to cast for each …" is the same variant there.

A raw literal is still legal everywhere, and
`lints::every_layer_in_the_pool_is_the_one_its_modifier_derives` is what
stops one disagreeing with the macro beside it.

A shape without a macro (`Replacement`, `CopyOnEnter`, `Ward`, `Suspend`,
`ModalSpell`, `Echo`, `Prepared`) is written as the plain enum literal —
those have no fields the rules can supply for you.

### As-it-enters modifiers (`FaceDef::enter_modifiers`)

`Tapped`, `TappedUnless(filter)`,
`TappedUnlessCount { filter, at_least }` (the battle lands' "two or more
basic lands" — its own variant because a checkland asks about *a*
permanent and a card never restates a default; the entering permanent
never counts itself),
`TappedUnlessAtMost { filter, at_most }` (the same count bounded from
above — the fast lands' "two or fewer other lands", and the same predicate
the Forgotten Realms manlands print as its complement, "if you control two
or more other lands, this land enters tapped"),
`TappedUnlessOpponents { at_least }` and
`TappedUnlessSomeoneAtOrBelow { life }` (the two cycles whose condition
counts **players** — no filter reaches a seat, and which seats count is a
rule: a teammate is not an opponent and a player who has lost is out),
`TappedOrPayLife(n)`, `ChooseSubtype`
(Roaming Throne, Reflections of Littjara, Cavern of Souls — answer stored
on `obj.chosen_subtype`; creatures also gain the subtype in their base),
`ChooseBasicLandType` (Phantasmal Terrain — the same question and the same
place for the answer, offering the five basic land types of CR 205.3i and
nothing else; read back by `Modifier::SetLandTypeToChosen`; the reader
writes it for `DB$ ChooseType | Type$ Basic Land` behind
`K:ETBReplacement:Other`),
`ChooseColor` and `ChooseColorExcept(c)` (Uncharted Haven, the Thriving
cycle, the Gates — answer stored on `obj.chosen_color` and read back by
`ManaSource::Chosen`), `ChooseCardName` (Pithing Needle — any face of any
card of the pool, CR 201.4; answer stored on `obj.chosen_name` as the card
and face it names, and read back by `Modifier::ChosenNameCantActivate`),
`Prepared` (Emeritus of Woe),
`TappedUnlessReveal(filter)` and
`WithCounters { kind, amount }`.

`TappedUnlessReveal(filter)` is the reveal lands — "as this land enters, you
may reveal a Faerie card from your **hand**; if you don't, it enters
tapped" — and it is the only modifier here whose filter is read against a
**hidden** zone. Every `TappedUnless…` sibling walks the battlefield, which
is why not one of them could say this sentence and why eighteen lands sat at
`Coverage::Partial` entering untapped unconditionally, which is the half of
the card that is pure upside. Write the filter as the printed words alone —
`Filter::HasSubtype(subtypes::creature::FAERIE)`, with no `Filter::CREATURE`
and no `ControlledByYou`: "a Faerie card" is the subtype (Magic prints tribal
instants that carry a creature type), and the menu is built from the
controller's own hand already. It is a *choice of card* rather than a
yes-or-no because CR 701.20a shows the card to the table; naming nothing
declines it, and a hand with no matching card is not asked at all.

What it cannot say is a **disjunction**. Temple of the Dragon Queen and
Fortified Beachhead print "tapped unless you revealed a Soldier this way *or*
you control a Soldier", and a face carries a *list* of modifiers: every arm
of the entry scan only ever inserts `TAPPED`, so two side by side are an
`and`. Both stay `Coverage::Partial` with the control half written, which is
the stricter of the two readings one modifier can give.

**A modifier that asks a question is applied last**, whatever order the card
prints it in, and that is a property of the engine rather than of the card:
publishing a `Pending` returns from the entry scan, and the arrival is
already off the list it was read from, so anything the loop had not reached
would never be applied at all. Uncharted Haven is `ChooseColor` then
`Tapped` and was entering untapped. `ChooseSubtype` and `TappedOrPayLife`
had the same hole from the day they were written and nobody could see it,
because no card in the pool prints another modifier behind one of them. A
card with *two* questions would lose the second; none prints that either,
and `lints::no_face_asks_two_questions_as_it_enters` is what keeps that a
fact rather than a hope — the day one does, the lint fails with the card's
name and the answer is a second pass rather than a second field.

`WithCounters` takes an **`Amount`**, so "this enters with X +1/+1 counters
on it" is expressible and Walking Ballista is an ordinary card. CR 107.3m is
what makes that a rule rather than a convenience: a replacement effect on a
permanent that refers to X uses the value of X chosen for *the spell that
became that object as it resolved*, and the value of X for the permanent
itself is 0. So the engine reads the announced X off the entering object —
the same card reanimated, blinked or put onto the battlefield by an effect
arrives with nothing (CR 107.3g).

That second half is **one normalisation at the arrival** rather than a
question each reader asks. `apply_enter_modifiers` walks every permanent
that entered and knows which zone it came from, so an entry from anywhere
but the stack puts `x_value` back to 0 there, and every reader of CR 107.3m
downstream is a plain read. It is written that way because there are two
readers and only one of them could have asked: `WithCounters` is a
replacement effect and has the arrival in its hands, while an
enters-the-battlefield **triggered** ability is stacked a step later by
`collect_triggers` with nothing left to ask. The Meathook Massacre is the
card that proves it — `{X}{B}{B}`, "when this enters, each creature gets
-X/-X" — and it read X as 0 for as long as the guard sat on the reader
instead of on the field, which is a `Coverage::Implemented` sweeper that
swept nothing. Only an object's **own** enter trigger takes the number
(`Trigger::ETB`), because the rule says *its* enter trigger: a landfall
`EntersBattlefield(&Filter::YOUR_LAND)` is about some other permanent and
announces nothing.

The counters go through `replacement::put_counters` like every other
replacement effect, so a counter doubler has its say (CR 614.16): a Vivid
land under a Doubling Season enters with four charge counters.

### Triggers

`EntersBattlefield(filter)`, `LeavesBattlefield(filter)`, `Dies(filter)`,
`SpellCast(filter)`, `Draws(rel)`, `DrawsExceptFirst(rel)`,
`FirstNoncreatureSpellCast(rel)`, `Attacks(filter)`, `BecomesTarget`,
`EntersBattlefieldEvoked`, `StepBegin { step, whose }`,
`CountersReach { kind, n }`, `PlaysLand(rel)`, `TappedForMana { by, filter }`,
`State(&condition)` (a state trigger, CR 603.8; see the Alpha pieces).

`TappedForMana { by, filter }` is "whenever [a player] taps [a permanent]
for mana": a player `by` names activated a mana ability of a permanent
matching `filter` with {T} in the cost (CR 106.12), and it resolved and made
mana (CR 106.12a) — once per activation, however many colours. `by` is
`PlayerRel::You` for "whenever you tap" (Badgermole Cub), `EachPlayer` for
"whenever a player taps a land" (Manabarbs) and for "whenever a Mountain is
tapped for mana" (Gauntlet of Might), which names nobody, and `EachOpponent`
for "an opponent". The tapped permanent is the event's object, so
`PlayerRel::ControllerOfEvent` is "that player" and "its controller": only a
permanent's controller can activate its abilities (CR 602.2). Written with
no target and effects that add mana (`AddMana`, or `AddManaFor` for another
player's pool), the ability is itself a mana ability (CR 605.1b,
`AbilityDef::is_triggered_mana_ability`) and resolves as it triggers, off
the stack (CR 605.4a): write it as a plain `triggered!`, with no flag. One
that targets (Forbidden Orchard's) or makes no mana (Manabarbs') is an
ordinary trigger.

`PlaysLand(rel)` is "whenever [a player] plays a land" (Fastbond): the
special action (CR 116.2a, 305.1), out of the hand or from wherever a
permission allows (Crucible of Worlds), and never a land an effect puts
onto the battlefield, which a landfall `EntersBattlefield` would also see.
Fastbond's "if it wasn't the first land you played this turn" is the
intervening `condition = Some(Condition::LandsPlayedThisTurnAtLeast(2))`:
the land the trigger is about is already counted when it is collected.

`CountersReach { kind, n }` fires when the source's count of `kind` goes
from below `n` to `n` or more, the window CR 714.2b writes out for a
chapter. It is Druid Class's "When this Class becomes level 3" (`Level`,
`n: 2`, because a Class's level is kept as level counters over level 1). A
level-up payoff that **targets** is this trigger, never an effect riding on
the level-up activation: a target removed in response would take the level
with it (CR 608.2b).

`Trigger::ETB` is `EntersBattlefield(&Filter::This)`, which 99 of the pool's
110 enter-triggers are. It is a constant and not a macro because there is
nothing to parameterise: a trigger pointed at anything other than the source
keeps the variant and its filter.

### Target specs (`TargetSpec`)

`Object(filter)`, `Spell(filter)`, `StackOrBattlefield(filter)`,
`CardInGraveyard(filter, rel)`, `ThisObject`, `EventObject` (implicit —
the object the trigger was about), `AbilityOnStack(filter)`,
`SpellOrAbility(filter)` (Ertai), `Player(rel)`, `AnyPlayer`, `AnyOpponent`,
`AnyTarget`.

"Target cards from a single graveyard" (Unlicensed Hearse) is
`CardInGraveyard(filter, PlayerRel::Chosen)` on an activated ability. When
more than one graveyard holds a match, the activation first asks
`Pending::ChoosePlayer` over those graveyards, and the targets offered are
the named graveyard's cards. With one, nothing is asked. The offer and
CR 608.2b's re-check read every graveyard.

`AnyTarget` is "any target" (CR 115.4) — a creature, a planeswalker, a battle
**or a player**, chosen from one set that spans objects and players. It is its
own variant rather than a `Filter`, because no filter can match a player: a
player has no characteristics to filter on. Every burn spell printed says
this, so it is not a corner: `Pending::ChooseTargets` carries a
`player_options` beside `options`, `min`/`max` count across both, and the
answer is `PlayerAction::ChooseTargets { objects, players }`. The players
picked ride on the spell as a `SeatSet` — a bitmask, because CR 601.2c makes
targets distinct and a game object is copied once per AI ply.

`ThisObject` names the **source** and is not a target: nothing is chosen, no
`Pending::ChooseTargets` opens, and hexproof, shroud and protection have
nothing to answer (CR 115.6). That is what a card printing "return **Oboro**
to its owner's hand" says, as against "return **target** land". It is the one
spec the resolver reads off the effect rather than off the answer, and only
where an arm has been written to — today `Effect::ReturnToHand` and
`Effect::Destroy`, both through `resolve::zones::spec_object`. Any other
effect given a `ThisObject` compiles, claims `Coverage::Implemented`,
resolves, and does **nothing**: three cards sat that way until #147, and
neither `xtask validate` nor the pool lints could see it, because all three
say exactly the right thing. `engine::this_object_tests::every_this_object_in_the_pool_is_one_the_resolver_reads`
is the guard, so a card that reaches for a new one fails the build rather
than a player; the fix is to read the spec in that arm, not to work around
it in the card.

`EventObject` is the **second** implicit spec and has two homes, which is
what makes it easy to get wrong. As a trigger's own *target requirement*
(`targets = Some(TargetReq::one(TargetSpec::EventObject))`) it is filled from
the triggering object when the trigger is stacked, and five cards in the pool
are written that way — Storm of Saruman, Reflections of Littjara, both halves
of Jin-Gitaxias. On an **effect's** field it is the same story as
`ThisObject`: `spec_object` reads it and nothing else does. Journey to
Eternity's "when enchanted creature dies, return **it** to the battlefield"
names no target (CR 115.1), so its `GraveyardToBattlefield` read an empty
answer, left the creature in the graveyard, and returned only its own back
face transformed — `Coverage::Implemented` and half a card. Note which way
the fix went: the effect learned to read the spec, and the card was **not**
given a target requirement, because that would have turned a sentence that
does not target into one that does.

The catch-all in `spec_object` is gone with it. It was `_ =>
res.targets.first()`, and a spec that *names* something rather than asking
for it reads there as "nobody chose anything" — which is how the second one
hid behind the fix for the first. Every variant is listed, so a third gets a
compile error instead of a card that quietly does nothing, and
`every_event_object_in_the_pool_is_one_the_engine_reads` counts both homes.

Where an effect exists that acts on the source by name, prefer it:
`SacrificeSelf`, `ExileSource`, `PutSourceOnTopOfLibrary`, `UntapSelf`. And
do **not** reach for `targets = Some(TargetReq::one(TargetSpec::ThisObject))`
to make a `ThisObject` effect work — it does work, and it turns a sentence
that does not target into one that does.

`AnyPlayer` and `AnyOpponent` are targeting *requirements*, not effect
targets: the choice resolves into the spell or ability's `chosen_player`, and
the effect reads it back as `Player(PlayerRel::Chosen)`. Handing the
requirement to the effect instead is silently a card that does nothing —
`DealDamage` would look for an object target and find none. `AnyOpponent` is
the same choice over a smaller set (CR 115.1): in a game of four, "target
opponent" is three seats, and `Player(PlayerRel::Opponent)` — which is every
opponent and no choice at all — is a different card.

### Filters (composable data)

`Any`, `This`, `Another`, `And(&[..])`, `Or(&[..])`, `Not(&..)`,
`HasType`, `LacksType`, `HasSupertype`, `HasSubtype`, `HasColor`,
`IsColorless`, `Monocolored`, `IsToken`, `ControlledByYou`,
`ControlledByOpponent`, `OwnedByYou`, `Tapped`, `Untapped`, `Attacking`,
`HasKeyword`, `CmcAtMost`, `CmcAtLeast`, `MatchesChosenTypeOfSource`
(Roaming Throne & co.), `Named(&str)`, `EnteredThisTurn`,
`InZone(ZoneRef)` (incl. `NotBattlefield` for cross-zone effects).

`EnteredThisTurn` is the one filter that asks about **history** rather than
a characteristic — "target creature that entered this turn" (Drannith
Ruins), "each creature that entered this turn" (Novijen), "activate only if
this land entered this turn" (Mirrex). Every other filter reads the object
in front of it. This one reads `per_turn.entered_battlefield`, the turn's
list of arrivals, which `move_object` and a token's arrival write and every
turn start clears. Two consequences. It costs a search of that list rather
than a field compare, so it belongs in the narrow half of an `And` and not
the wide one. And a `PlayerView` carries no such list, so
`baylee_ai::filter` answers `None` for it and the client's
targeting reader refuses it: a seat cannot pre-compute the legal targets of
an ability that asks this, and takes the engine's enumeration instead.

`Named` is the one filter that carries a **name** rather than a handle, and
it carries a `&'static str` on purpose: CR 201.2 compares what an object is
*called*, so a clone, a face-down permanent turned face up and a card a
text-changing effect has renamed all answer by the name they carry now — a
`CardIndex` would answer by the card they were printed as.
`docs/card-identity.md` is normative on which handle may be stored where,
and a name is the one that may not be.

**Write them with `f!`, adjectives then noun.**

```rust
f!(CREATURE)                              // Filter::CREATURE itself
f!(owned CREATURE)                        // a creature you own
f!(another nontoken CREATURE)
f!(owned Filter::HasSubtype(ally::ALLY))  // an Ally you own
```

The adjective list is closed — `your`, `not_yours`, `opponents`, `owned`,
`another`, `token`, `nontoken`, `tapped`, `untapped`, `attacking`,
`colorless` — and each one is a single nullary `Filter` variant, except
`not_yours`, which is `Not(&ControlledByYou)`. It is the one negation with an
adjective of its own because "you don't control" is not "an opponent
controls": at a table with sides (`dev-table --teams`) a teammate's creature
is one you don't control and no opponent's, so spelling it `opponents` would
be a different card that reads the same in a duel. Anything that takes an
argument stays a variant (`Filter::HasColor(…)`, `Filter::CmcAtMost(1)`),
because `f!` is a shorter spelling of the filters we already have and not a
second filter language: it can say nothing `Filter` cannot.

It is a macro rather than a `const fn` because combining filters means
building a `&'static [Filter]` from parameters, and a slice built from a
parameter inside a `const fn` cannot be promoted to `'static` (E0716). A
macro expands in the caller's `static`, where the slice promotes like any
other literal.

**Compose them inline.** In `static` context a slice promotes to `'static`
automatically, so `f!(owned CREATURE)` needs no named `static` at all. Give a
filter a name only when the same card refers to it **twice** — that is the
whole rule, and 123 of the pool's 159 local filter statics are named for a
filter their card mentions once.

**Reach for the named ones first.** `Filter` carries constants for the
predicates the pool kept reinventing — `CREATURE`, `ARTIFACT`,
`ENCHANTMENT`, `LAND`, `PLANESWALKER`, `NONLAND`, `NONCREATURE`,
`BASIC_LAND`, `INSTANT_OR_SORCERY`, `ARTIFACT_OR_ENCHANTMENT`,
`ARTIFACT_OR_CREATURE`, `ARTIFACT_CREATURE_OR_ENCHANTMENT`,
`CREATURE_OR_PLANESWALKER`, `NONBASIC_LAND`, `NONTOKEN_CREATURE`,
`ANOTHER_CREATURE`, `LEGENDARY_CREATURE`, `ATTACKING_CREATURE`,
`YOUR_CREATURE`, `OPPONENT_CREATURE`, `YOUR_LAND`, `YOUR_BASIC_LAND`,
`YOUR_ARTIFACT`, `ANOTHER_CREATURE_YOU_CONTROL`,
`YOUR_CREATURE_WITH_POWER_4_OR_GREATER`. "A creature" had been
written out as `HasType(TypeSet::CREATURE)` in a differently-named `static`
in twenty-six card files, which is twenty-six chances to type `LacksType` by
accident and no way to grep for the one that did.

That list is checked rather than kept:
`the_authoring_contract_names_every_filter_constant` in `baylee-cards-dsl`
reads this file and fails on a constant it does not name. A hand-kept list of
what exists goes stale the first time something is added, and this one had —
it was six names short of `filter.rs` for exactly as long as nothing compared
the two.

**First** is meant literally, and the composite constants are where it
bites: `f!(your CREATURE)` expands to exactly the bytes
`Filter::YOUR_CREATURE` holds, so the two are one filter with two spellings
and the constant is the one to write.
`the_filter_macro_spells_the_constants_it_replaces` is what keeps the pair
from drifting. `f!` is for the combination no constant carries — which is
most of them, since a constant earns its place by being wanted twice.

A filter that is about *this pool* rather than about Magic goes in
`crates/baylee-cards/src/filters.rs` (`YOUR_ALLIES`, `ANOTHER_ALLY`), beside
`crate::tokens`, which draws the same line. It earns a place there by being
written twice; one card's own compound filter stays in that card's file,
where the oracle sentence it encodes is a line above it.

### Effects (ops)

**The common ones have a verb**, and the verb is the word the card prints:
`Effect::draw(1)`, `scry(2)`, `gain_life(3)`, `destroy(t)`,
`destroy_no_regen(t)`, `regenerate(t)`, `exile(t)`,
`blink_to_owner(t)`, `blink_to_you(t)`, `bounce(t)`, and
`continuous(filter, modifier, duration)`
with the layer derived. `Effect::mana` is the precedent — 219 uses in the
pool against zero raw `AddMana` literals.

Two rules keep that from growing into a phrasebook. **One verb per variant,
and only where the variant has one answer to give**: `SearchLibrary { filter,
finds, optional }` has two real choices in it, so it stays a literal rather
than becoming a `search` / `may_search` / `search_to_hand` family. And **the
name is the word this pool already says**, which is usually the printed one;
blink and `bounce` are the two that are not. Neither is a coinage: the
engine named `Blink` because "exile it, then return it" has no printed verb,
and `bounce` was in this repository before there was a verb to hang it on —
Cyclonic Rift's comment calls both of its modes a bounce and Aether
Channeler's effect list is `BOUNCE_EFFECTS`. Both are paid for the same way:
neither word appears in any `//! Oracle:` header, so a grep from the printed
sentence to the code stops at these two and nowhere else.

`ReturnAllToHand` is the counter-example that keeps the first rule honest —
Cyclonic Rift's overloaded half, one use, a filter *and* an `opponents_only`
flag — so it stays a literal.

A fixed count is the argument (83 of the pool's 84 draws are fixed);
`{X}` and anything else writes the literal, the way `Effect::mana_dynamic`
sits beside `Effect::mana`.

**Destruction is two verbs and picking the wrong one is silent.**
`Effect::destroy(t)` is a destruction a regeneration shield replaces;
`Effect::destroy_no_regen(t)` is the one that prints "it can't be
regenerated" (CR 701.19c), and `destroy_all` / `destroy_all_no_regen` are the
same pair for a sweep. Write whichever sentence the card prints and nothing
else — the two were the same function for as long as this engine had no
regeneration, so a card written before 23.09.2026 proves nothing about which
one it meant. `Effect::regenerate(t)` is the shield itself: it puts one on
the target, and the *next* destruction this turn is replaced by tapping it,
clearing its marked damage and removing it from combat. A shield does not
survive the cleanup step, and it replaces **destruction** and nothing else: a
creature at zero toughness is put into a graveyard without being destroyed
(CR 704.5f) and dies through a shield, and so does one that is exiled or
sacrificed, because neither of those is a destruction either.

**Reanimation is two verbs for the same reason.** `Effect::reanimate(t)`
returns a card from a graveyard to the battlefield under **your** control
with nothing on it, which is what fifteen cards in this pool print — Sun
Titan, Reanimate, Recurring Nightmare. `Effect::return_to_owner_with(t, kind,
n)` is the other sentence: under its **owner's** control and with a counter
on it, which is what undying (CR 702.93a), persist (CR 702.79a) and Luminous
Broodmoth say. Neither is a flag on the other because the two differ in both
halves at once, and a card printing one of them prints all of it. The counter
goes on through the same door `EnterModifier::WithCounters` uses, so a
doubler has its say (CR 614.16).

**Blink is two verbs for the same reason, and the difference is who ends
up controlling the card.** `Effect::blink_to_owner(t)` is "exile …, then
return it to the battlefield under its **owner's** control" (Ephemerate,
Soulherder, Emiel the Blessed); `Effect::blink_to_you(t)` is "… under **your**
control" (Restoration Angel, Aminatou's −1, Sword of Hearth and Home). Both
are `Effect::Blink { target, owner_control }`, the field and the question
`GraveyardToBattlefield` already had. Write the one the card prints, even
where a filter such as "you own" makes the two agree: there is no bare
`blink`, because an unmarked default is how Restoration Angel came to hand a
stolen creature back to its owner. What returns is a new object (CR 400.7),
so no control effect over the old one reaches it, and it enters under the
player the sentence names (CR 110.2a). CR 610.3c ("returns under its owner's
control unless otherwise specified") is about a card that comes back after
an "until" event — Palace Jailer's "until an opponent becomes the monarch",
Werefox Bodyguard's "until this creature leaves the battlefield" — and does
not decide an immediate blink. Only control is chosen: the owner never changes
(CR 108.3), so a creature kept this way still dies into its owner's
graveyard and leaves the game with its owner (CR 800.4a).

**A permanent that exiles itself and comes back answers the same
question.** `Effect::ExileSelfReturnAsFace { face, owner_control }` takes
`owner_control: true` exactly where its sentence says "under its **owner's**
control" (Sheoldred's `{4}{B}`, the Ojers' dies triggers). "… under **your**
control" (Fable of the Mirror-Breaker III, Welcome to … III, Journey to
Eternity) is `false`, and so is a sentence that names nobody (The True
Scriptures III: the card enters under the player the effect instructs,
CR 110.2a, the chapter ability's controller, CR 603.3a). A sentence that
only says "transform this" is not this effect but `Effect::TransformSource`:
the same permanent turns over (CR 701.27a) and every effect on it goes on
applying (CR 712.18), where an exile and a return is a new object that
enters and sheds them. Eight cards were written that way until 2026-09-30.
`lints::every_self_return_comes_back_under_the_control_its_sentence_prints`
holds every use in the pool to its printed sentence, and refuses one whose
sentence prints no return.

**A linked exile is two verbs as well, and the difference is when it ends.**
`Effect::exile_linked(t)` exiles with a link and no end of its own: the card
stays until another ability of the same object brings it back (Safe Haven and
Endless Sands, `ReturnLinkedToBattlefield`) or for good (Skyclave Apparition).
`Effect::exile_until(t, ExileUntil::…)` is an "until" sentence (CR 610.3):
`SourceLeavesBattlefield` for Werefox Bodyguard's "until this creature leaves
the battlefield", `OpponentBecomesMonarch` for Palace Jailer's "until an
opponent becomes the monarch" (an opponent of the player who controlled the
exiling ability, whoever controls the Jailer by then). Both are
`Effect::ExileLinked { target, until }`. The return is not a triggered
ability: it happens the moment the event does, uses no stack, and puts the
card back under its owner's control (CR 610.3c). If the event has already
happened when the exile would, nothing is exiled (CR 610.3a, 610.3b): a
Bodyguard sacrificed in response to its own trigger holds nothing. Write
`exile_until` wherever the card prints "until"; an `exile_linked` that stands
for one is a card that never gives its prisoner back. Every way back, a
host's effect, a new monarch or a host leaving, goes through
`GameState::return_linked`. The link ends as well when the card leaves exile
any other way (cast, returned to a hand): exiled again later, it is a new
object and not "exiled with" the old host (CR 400.7). So does everything
else the card was in exile, on an adventure, suspended, castable from exile
(`Rider::ends_as_it_leaves_exile`).

A card that says nothing about a graveyard cannot use either reanimation
verb: the effect checks that its object is still in one (CR 400.7). A
reanimation *spell* is already held to that by target legality (CR 608.2b) —
the guard is there for undying and persist, which target nothing at all.

Life/draw: `GainLife`, `GainLifeFor`, `GainLifeDoubleX`, `LoseLife`,
`DrawCards`, `DrawCardsFor`, `Scry`, `ScryFor`, `Mill`,
`RearrangeTopLibrary`/`ReorderTopLibrary`.
Combat/damage: `DealDamage`, `DealDamageToTargetController`,
`DealDamageEach` (`Effect::damage_each(n, &filter)`), `Fight`,
`DamageEqualToPower`.

Three damage sentences, three spellings, and none of them overlaps:

- **"… to target X"** is `DealDamage` with the choice on the ability's
  `targets`/`target`.
- **"… to each opponent / each player / you"** is
  `DealDamage { target: TargetSpec::Player(rel) }` with *no* `TargetReq`:
  `Player(EachOpponent)` is every opponent and nobody chooses (Mount Doom,
  Shivan Gorge).
- **"… to each creature …"** is `DealDamageEach { amount, filter }`: every
  permanent the filter matches as it resolves, none of them a target, so
  hexproof does not stop it (CR 115.10a).

A sentence naming both halves — "to you and each creature you control",
"to each creature and each player" — is both effects in one list. Spell the
filter with its **printed noun**: "each creature with flying" is
`Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)])`,
never `HasKeyword(FLYING)` alone. The resolver deals damage only to creatures
and planeswalkers whatever the filter says, but that is a backstop and not a
spelling to lean on — a flying planeswalker would be swept by the short one.
Several keywords are one set chained with `.union` (`KeywordSet` has no `|`),
and `HasKeyword` matches *any* bit of it, so `Not(&HasKeyword(set))` is "has
none of them" (Path of Mettle).

`Fight { fighter, foe }` (CR 701.14a) and `DamageEqualToPower { dealer, to }`
name their creatures by `TargetSlot` — `This` (the source, not a target),
`First` (the first instance of "target") and `Second` (the second) — because
a fight is the one sentence whose two creatures are two *different* instances
of "target": Khalni Ambush's "target creature you control fights target
creature you don't control" is two requirements, not one requirement for two
objects. The second is written `second_targets = Some(TargetReq::…)` on
`spell!`, `activated!` or `loyalty!` (Oko, Thief of Crowns' −5), or on a
choose-one modal spell's `mode!` (Archdruid's Charm's second mode), beside
`targets`/`target` — never on a mode of a spell that chooses several,
whose chosen modes take one instance each (`lints::modes_fault`), never on
a modal trigger's mode, which is put on the
stack without the cast wizard that asks it
(`no_modal_trigger_mode_prints_a_second_target`), and never on a mana
ability, which may not target at all (CR 605.1a) and which
`lints::mana_ability_fault` refuses through either instance.
It is asked after the first, is its own list at every layer, and is never
appended to `targets`: every reader of that list takes it to be one instance.
Each instance is narrowed on its own at resolution (CR 608.2b), and the spell
fizzles only when *every* instance lost all it named; a fight whose one side
is gone deals no damage at all (CR 701.14b), which the resolver checks on
both sides whatever the narrowing did, because a `This` fighter is never
narrowed. That last case is guarded by `resolve::life::fighting` and proved
by nothing: no card in the pool fights with `This` yet (Golden Guardian is
the one that will), and its first test is owed with it.

Both read **projected** power at the moment they resolve, and the projection
is refreshed before every effect in a list, so Bridgeworks Battle's
`PumpTarget` followed by its `Fight` fights at the pumped power. Negative
power deals 0 (CR 107.1b); damage to a planeswalker removes loyalty
(CR 306.8), which is Stump Stomp's "creature or planeswalker"; each creature
is the source of its own damage, so deathtouch and protection apply; and it
is not combat damage (CR 701.14d), so combat-only lifelink does not fire —
noncombat lifelink (CR 702.15b) is not implemented yet and no card in the
pool that fights has it.
Removal: `Destroy`, `DestroyAll`, `DestroyOthersNamedLike { target }`
(Maelstrom Pulse's "and all other permanents with the same name as that
permanent": it reads the target's name as it resolves, so it is written
*before* the `Destroy` that moves the target; a nameless target sweeps
nothing, CR 201.2a), `Regenerate`, `Exile`, `CounterTargetSpell`,
`CounterTargetAbility`, `CounterTargetSpellOrAbility`,
`TargetSourceLosesAbilities` (Tishana's Tidebinder: it reaches the permanent
whose ability an *earlier* `CounterTargetAbility` in the same effect list
countered, so it has to follow one; `source_filter` is the printed
restriction on which permanents it reaches), `SacrificeFilter`, `ReturnToHand`,
`ReturnAllToHand`, `ReturnChosenToHand`, `ChangeTarget { to }` ("change the
target of target spell", CR 115.7a: Misdirection with `Filter::Any`,
Hydroelectric Specimen's "to this creature" with `Filter::This`) and
`ChooseNewTargets` ("you may choose new targets for target spell or ability",
CR 115.7d: Deflecting Swat). The two are different sentences: the first must
move every target to another legal one or none, the second may leave any.

The three returns are three different sentences and picking by destination
gets them wrong. `ReturnToHand` **targets** one permanent, so the caster
chooses and hexproof answers; `ReturnAllToHand` sweeps every match with
nobody choosing at all; `ReturnChosenToHand { who, filter }` is the bounce
land's "return a land you control to its owner's hand" — each player in
`who` picks one of the permanents **they** control, on resolution, and the
card prints no "target", so CR 115.1 never applies. It is mandatory, because
the printed sentence is: "you may return …" is the same effect wrapped in
`MayDo`. `SacrificeFilter` and `DestroyChosenForPlayers` are its two
siblings, identical but for where the permanent ends up.
Zones: `SearchLibrary`, `SearchLibraryOf` (the search `SearchLibrary`
cannot say: another player's library — `library: ControllerOfTarget` with
`owner_searches: true` is "that player may search their library", `library:
Chosen` with `owner_searches: false` is Bribery's "search target opponent's
library … under your control" — or a `mana_value: Some(ManaValueBound { cmp,
amount })` the resolution computes, such as `Amount::Plus { base:
&Amount::SacrificedManaValue, offset: 2 }` for Eldritch Evolution; the
library searched is the one shuffled), `SearchLibraryUpTo { filter, count,
find }` ("search your library for up to X … cards": the count an `Amount`
read as the search begins, every card found going where `&Find` says; X = 0
shuffles and asks nobody — Nylea's Intervention), `SearchOpponentSplits {
filter, up_to, chosen }` (Realms Uncharted: up to four cards of different
names — one card per name is offered — revealed; an opponent sends `chosen`
of them to the graveyard, prompt `PutIntoGraveyard`, and the rest go to the
hand; at a table the caster names that opponent with a `ChoosePlayer`, and
`chosen` or fewer found are all chosen), `Find::…with_counter(kind, n)` for a
find that enters with counters (Neoform), `Find::…when_matching(filter,
&then)` for a find that forks on the card found (Archdruid's Charm: "onto
the battlefield tapped if it's a land card. Otherwise, into your hand" is
`Find::HAND.when_matching(&Filter::LAND, &Find::BATTLEFIELD_TAPPED)`, asked
of the card as it is in the library), `Find::…any_number()` for "any number
of" cards, the last find repeating for every card found, so the search may
take as many as match (The World Tree's Gods, in an optional search),
`PutFromHandOntoBattlefield {
filter, mana_value, optional }` (Aether Vial, with `Amount::CountersOnSource`
as its bound; not a cast and no land drop), `OptionalBasicLandSearchFor`,
`GraveyardToTop`,
`GraveyardToHand`, `GraveyardToBattlefield`, `YourGraveyardToBattlefield {
filter, tapped }` (every matching card in your graveyard, untargeted, read
before any moves and tapped as it arrives: World Shaper's and Lumra's
"return all land cards from your graveyard to the battlefield tapped"),
`Earthbend(n)` (CR 701.66a, on the ability's first
target, `TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND))`: the land
becomes a 0/0 land creature with haste, gets `n` +1/+1 counters, and a
delayed trigger returns it tapped under your control when it dies or is
exiled — Badgermole Cub, Ba Sing Se; never spell the three continuous
effects out), `ReturnToBattlefieldTapped { target }` (that delayed trigger's
own effect, on the `EventObject`; no card writes it), `ExileGraveyard`, `Blink`
(through its two verbs), `ExileLinked` (through its two verbs),
`ExileTargetsWithSource` (every target, each exiled with the
source for good, CR 406.6: Unlicensed Hearse; `Rider::ExiledWith`, never
`Linked`, which "until" exiles and the monarchy release),
`ReturnLinkedToBattlefield`, `PutFromHandOnTop`,
`PutSourceOnTopOfLibrary`, `ExileAndReturnAtEndStep` (Venser +2, Eerie
Interlude), `BottomCardFromHand`, `WishToHand` (Karn's −2: a card you own
from outside the game or face-up in your exile).
Playing from elsewhere: "you may play that card this turn" is a permission
for one object (`PlayPermission` in the engine's `PerTurn`, keyed on the
object's version, so it ends with the turn or as the card moves): a land is
the turn's land drop, a spell is cast by its timing, and `free: true` is
"without paying its mana cost" (X is 0). `ChooseExiledToPlay { owner,
counter, free }` chooses the card from any exile (Dauthi Voidwalker:
`owner: Opponent, counter: Some(counters::VOID), free: true`);
`LookAtTopKeepBottomPlay { count }` is Expressive Iteration's one to the
hand, one to the bottom, the rest exiled and playable.
`LookAtTopPick { count: Amount, pick, random }` is "look at the top X, put N
into your hand and the rest on the bottom", in any order (the player
arranges them) or, with `random: true`, "in a random order" (nobody is
asked).
Continuous: `CreateContinuousEffect` (any layer+filter+modifier+duration),
`PumpFilter` (a filter, where `Filter::This` is the *source*, plus
`controlled_by: Option<PlayerRel>` for a sentence that names a player rather
than the board — "creatures **target player** controls get -2/-2", where the
seat is a choice no `Filter` can be told about), `PumpTarget`
(the spell's or ability's targets, all of them — Giant Growth), both of which
carry a `KeywordSet` so "+2/+2 and gains trample" is one effect,
`SetPTFilter`, `ChangeController`, `AllCreaturesToOwner`,
`ExchangeControlOrSacrifice` (Gilded Drake), `ExchangeControl` (the first
target's controller and the second's swap them, CR 701.12a–b: all or
nothing, and nothing between two permanents of one player), `PhaseOut`,
`AttachSelf`, `UntapChosen { filter, count }` ("untap up to N lands" with no
"target": chosen as it resolves, Treachery), `Populate` (CR 701.36: a choice
on resolution, never a target, Nesting Dovehawk).
"For each opponent, … up to one target [filter] that player controls" is
`TargetReq::up_to(TargetSpec::ObjectOfEachOpponent(&FILTER), u8::MAX)` (The
True Scriptures I): the controller is asked once per opponent, in turn order,
each question offering only that player's permanents. Triggers only for now.
A spell "with a single target" is `TargetSpec::Spell(&Filter::WithSingleTarget)`
(CR 115.9a: every instance of "target" and every player counted, Misdirection).
"If this would be put into a graveyard from anywhere, exile it instead" (every
disturb back) is `AbilityDef::Replacement(ReplacementRule::ExileSelfInsteadOfGraveyard)`
on that face: registered on the battlefield, carried on the stack by the cast.
Tokens/copy: `CreateToken`, `CreateTokenN`, `CreateTokenForTargetController`,
`CreateTokenFromLinked`, `CreateTokenCopyOf`, `CreateTokenCopyOfEquipped`,
`CreateTokenCopyOfFirstToken`, `CopyTargetSpell`, `CopyTargetAbility`,
`Amass`. `CopyTargetAbility` is "copy target activated or triggered ability
you control. You may choose new targets for the copy" (Vantress Visions),
over `TargetSpec::AbilityOnStack(&Filter::ControlledByYou)`: the copy keeps
every decision made for the original, mode, targets, X and what paid its
costs (CR 707.10), and the same source (CR 707.10b); it is neither activated
nor triggered, so nothing watching for either sees it; and its controller is
then asked CR 115.7d's question target by target, naming nothing to keep one
(CR 707.10c). `CopyTargetSpell` copies a spell the same way, through the one
constructor `resolve::copy_spell`: mode, X, the face cast, a kicker, and its
object and player targets (CR 707.10). `CopyThisSpell` is the engine's and
never a card's: it is the replicate trigger's effect, one per payment, and
copies the spell the trigger came from, by last known information once that
spell has left the stack (CR 608.2h).
Costs/taxes: `PlayerMayPayOr` and `PlayerMayPayCostOr` — the two halves of
"… unless you <pay>", split by what the price is. The first charges *generic*
mana in an `Amount`, because Esper Sentinel's tax is its own power and a
number only resolution knows cannot live in a `ManaCost`. The second charges
one `CostPart` the player pays by **naming an object**, which is the Karoo
sentence: "sacrifice it unless you return an untapped Plains you control to
its owner's hand". There is no yes-or-no question on the second — the player
is shown what may pay and naming nothing is how they decline, so a price
nobody can pay asks nothing at all and the fallback simply runs. Its price
has to be one of the five parts a player answers with an object
(`Sacrifice`, `Discard`, `TapOther`, `ReturnToHand`, `ExileFromGraveyard`);
a `PayLife(2)` there
would put up an empty menu and decline itself on every board, which
`vocabulary_tests::every_price_paid_by_naming_an_object_puts_a_menu_up`
refuses over the compiled pool.
Prevention is a shield the effect leaves behind (CR 615.7):
`PreventNextDamage { target, amount }` is "prevent the next N damage that
would be dealt to <target> this turn", its `target` naming recipients as
`DealDamage`'s does (Samite Healer's any target, Conservator's
`Player(PlayerRel::You)`), and `PreventAllCombatDamageThisTurn` is Fog.
`PreventNextFromChosenSource { sources, combat_only, all_but, gain_life }`
is "the next time a <sources> of your choice would deal damage to you this
turn, prevent that damage" (the Circles of Protection): the source is chosen
as it resolves, `combat_only` and `all_but: 1` make Forcefield's "combat
damage … all but 1 of that damage", and `gain_life` is Reverse Damage's
"you gain life equal to the damage prevented this way". `sources` is both
what may be chosen and what the source must still be when it deals the
damage. All of them last until the turn's cleanup; `docs/engine-internals.md`
§"Prevention shields" says how they are spent.
Its redirection sibling is `RedirectNextFromChosenSource { target }`, Jade
Monolith's "the next time a source of your choice would deal damage to
target creature this turn, that source deals that damage to you instead":
any source may be chosen as it resolves, and `target` names the creature as
`DealDamage`'s target does (the ability's own `target` makes the choice).
The standing kind is `Modifier::RedirectDamageToYou(&from)`, Veteran
Bodyguard's "all damage that would be dealt to you by unblocked creatures is
dealt to this creature instead": `from` is what the source must be, the
affected filter is what takes the damage, and a condition on that ("as long
as this creature is untapped") goes into the affected filter,
`Filter::And(&[Filter::This, Filter::Untapped])`, so that it is read as the
damage is dealt. `docs/engine-internals.md` §"Redirection" says how both
apply.
`Modifier::CountersPreventDamage(kind)` on `Filter::This` is Rock Hydra's
"for each 1 damage that would be dealt to this creature, if it has a +1/+1
counter on it, remove a +1/+1 counter from it and prevent that 1 damage".
The mirror of the first is `PlayerMayPayThen { player, mana, effects }`:
"you may pay {1}. If you do, you gain 1 life" (Crystal Rod, Soul Net). The
same question and payment, with the effects on a yes; the price *is* the
"may", so it is never wrapped in a `MayDo` as well, which would ask twice.
"That player" in a cast trigger's tax is `PlayerRel::ControllerOfEvent`
— the one who cast the spell. `PlayerRel::Opponent` is the first living
opponent, which is the same seat heads-up and the wrong one at a table of
three (Mystic Remora).
Cumulative upkeep (CR 702.24a) is no keyword of its own but the triggered
ability it means: `Trigger::StepBegin { Upkeep, You }` with the printed
intervening `if` as `Condition::SourceMatches(&Filter::InZone(
ZoneRef::Battlefield))`, an `AddCounter` of `counters::AGE`, then
`PlayerMayPayOr { player: You, mana: Amount::CountersOnSource(counters::AGE),
effect: &Effect::SacrificeSelf }`. `Amount::CountersOnSource(kind)` reads
the source's counters as it resolves, after the counter above went on.
Also `AddCounter`, `AddCounterFilter`,
`DrainAllCountersIntoSelf` (Thief of Blood), `AddMana`,
`DelayedManaAtNextFirstMain` (Mana Drain), `SacrificeSelf`,
`PayCostOrLoseLater`, `ExileTargetsCreateTokens`.
Conditional: `IfKicked { then, otherwise }`, `IfEventPowerAtLeast { n, then,
otherwise }` (Tribute to the World Tree), and four that run `then` or nothing
— `IfCreaturesDiedAtLeast { n, then }`, `IfNotLostLifeThisTurn { then }`
(Luminarch Ascension), `IfControlGreatestCmc { filter, then }` (Padeem) and
`IfNoCountersOnSelf { kind, then }`. The last is the sentence the six
counter-sacrifice lands end with — "If there are no depletion counters on
this land, sacrifice it" — and it is an ordinary effect in the same list as
the mana, because that is how the card prints it: `mana_ability!(cost!(TapSelf,
RemoveCounterSelf { kind: counters::DEPLETION, n: 1 }), &[Effect::mana(ManaColor::Black, 2),
Effect::IfNoCountersOnSelf { kind: counters::DEPLETION, then: &[Effect::SacrificeSelf] }])`.
Magic prints the comparison against zero and no other, which is why there is
no threshold field.
Optional: `MayDo { effects }` — the printed "you may", wrapping the whole
clause the word covers. It asks the controller and runs `effects` only on a
yes, so it is the right variant **only** when the card prints the word and
the choice is not already offered somewhere else: "you may have *target*
player lose life" is a `TargetReq` with a minimum of zero, "you may pay 2
life" as a land enters is an `EnterModifier`, and "you may play those cards"
is a permission with nothing to ask. `xtask validate` holds every card
printing "you may" against that list and says which construct it accepted.
"You may [pay]. If you do, [effect]" makes the action a cost paid as the
ability resolves (CR 118.12): write the action as the **head** of the list
and what it buys after it — `MayDo { effects: &[SacrificeSelf, …] }` (Safe
Haven), `MayDo { effects: &[RemoveCounterSelf { kind, n }, …] }` (Living
Artifact). The engine reads the head and does not ask while the payment is
impossible (CR 608.2d): the source gone, or too few counters on it. A head
it has no rule for is asked unconditionally, so check `may_clause_possible`
before writing a new one.
Reflexive: `Reflexive { when, effects, target }` — "When you do, …"
(CR 603.12). It is written as the **last** op of the list, directly after
the action it waits for:
`&[Effect::SacrificeSelf, Effect::Reflexive { when: ReflexiveEvent::SacrificedThis, effects: &[…], target: None }]`
for a New Capenna fetch land, and `&[Mill, MayDo { effects: &[SacrificeSelf] }, Reflexive { … }]`
for Eden. It creates a triggered ability of its own, and only if the action
happened: a land bounced in response to its own enter trigger fetches
nothing. The ability goes on the stack after the resolution that created it,
so players can respond to it, and `target` is chosen at that point. That is
why Eden's return may take a card its own mill has just put into the
graveyard. It is not a `Trigger`, because a trigger fires on the event
whoever caused it (Brokers Hideout was once `LeavesBattlefield(&This)`, and
it fetched on a bounce), and it is not the rest of the same list, because
that list resolves with no stack object and no priority window.
`lints::every_reflexive_sits_where_it_can_trigger` holds the placement:
- the list is not a mana ability;
- the reflexive is last, and nested in nothing;
- the op before it is the action;
- nothing before the action is off a short allow-list, which today is
  `Mill` alone.

The engine counts any departure of the source by effect as the sacrifice,
and that count is exact only in this shape. `ReflexiveEvent` has one
variant, `SacrificedThis`. Grist's −2 and Agatha's Soul Cauldron need
`Sacrificed(&Filter)` and `Exiled(&Filter)`.
Utility: `UntapTarget`, `UntapSelf` (the source, with no target and no
question — CR 115.1c makes an activated ability targeted only when it says
the word, so "untap this creature" is the second variant and not the first
pointed at itself), `NegXFixed` (amount), `CreateTokenCopyOfFirstToken`,
`BecomeMonarch(PlayerRel)` (`You` on a card), `Sequence(&[..])`.
Modal/sequence: `Sequence(&[..])`.

### Modifiers (layer effects)

`AddType`, `RemoveType`, `AddSubtype`, `AllCreatureTypes`,
`ReplaceCreatureTypes(subtype)`, `AllBasicLandTypes`, `SetLandType(subtype)`,
`SetLandTypeToChosen`,
`BecomeType { types, subtype }`, `AddColor`, `SetColor`,
`AddKeyword`, `RemoveKeyword`, `LoseKeywords`, `LoseAllAbilities`, `ModifyPT`, `SetPT`, `SwitchPT`, `LegendRuleOff`,
`CantActivateArtifacts`, `ChosenNameCantActivate`, `OpponentsCastAsSorcery`,
`PlayersCantLose`,
`CantLoseLife`, `PreventDamageToIt`, `PreventDamageFromIt`, `RedirectDamageToYou(&from)`,
`CountersPreventDamage(kind)`,
`OpponentsCantSearch`, `NoMaxHandSize`, `GainControl`, `DoesNotUntap`,
`MayChooseNotToUntap`, `SkipUntapStep { who }`, `UntapAtMost { who, of, count }`,
`AttacksDespiteDefender`, `AttacksAsThoughHaste`,
`PlayLandsFromGraveyard`, `ExtraLandDrops`,
`DrawLimitPerTurn`, `CastPermanentSpellsFromGraveyard`,
`PermanentOfEachTypeFromGraveyard`, `CantBeTargetedBy`, `SetPTToCount`,
`ModifyPTHalfCount(count)`,
`ExileInsteadOfYourGraveyard`, `CastSpellsFromGraveyard`.

`ChosenNameCantActivate` is Pithing Needle's "activated abilities of sources
with the chosen name can't be activated unless they're mana abilities",
written `static_ability!(Filter::Any, Modifier::ChosenNameCantActivate)`
beside `EnterModifier::ChooseCardName`, which writes the name it reads. It
stops every activated ability the name reaches, every player's and in hand
as well (cycling), a loyalty ability included, and spares a mana ability
(CR 605.1a), turning a permanent face up and a prepared cast. A name chosen
by a *trigger* ("when this land enters, choose a land card name", Petrified
Hamlet) has no DSL yet.

`SetPTToCount(count)` is "this creature's power and toughness are each equal
to [count]" **granted** by an effect (Druid Class's animated land), with
`count` a `PtCount` and "you" in it the affected object's controller. CR
604.3a makes only a printed (or token-creating, or copied) ability
characteristic-defining, so the granted sentence sets power and toughness in
layer 7b; the printed one is `CharacteristicPT` in 7a (Ashaya, Soul of the
Wild). Write a printed `*/*` with `CharacteristicPT`, never as
`ModifyPTPerCount` over a 0/0 body: that is layer 7c and survives a 7b
"becomes 1/1" it should lose to.

`CantBeTargetedBy(&filter)` is "[this] can't be the target of [spells] or
abilities from [sources]" — protection's targeting half alone (CR 702.16b),
read at `eval::target_options` beside it. The filter is asked of the spell
or of the ability's source, with the static's controller as "you", so
Thrun, Breaker of Silence's "nongreen spells your opponents control or
abilities from nongreen sources your opponents control" is one filter.

`BecomeType` is "becomes a [subtype] [type]" with nothing retained (CR
205.1a): the card types and subtypes are replaced, supertypes stay, so
Oko's Elk is still legendary and no longer an artifact. A sentence that
says "in addition to its other types" or "still a …" (CR 205.1b) is
`AddType`/`AddSubtype` instead. "Becomes a [creature type] artifact
creature" (CR 205.1b's last sentence, Jade Statue's "3/6 Golem artifact
creature") keeps every card type and subtype except the creature types,
which `ReplaceCreatureTypes(subtype)` replaces; the card types it gains are
`AddType` beside it. The reader writes it for `Animate`'s
`RemoveCreatureTypes$ True`, and reads `Duration$ UntilEndOfCombat` as
`Duration::UntilEndOfCombat`. `LoseAllAbilities` (CR 613.1f) takes
keywords and printed abilities alike; a static of the object keeps only its
parts in layers 1, 2, 4 and 5 (CR 613.6), and a grant with a later
timestamp still lands (CR 613.7).

`SetLandType(subtype)` is "enchanted land is a Swamp" (Evil Presence), "all
Mountains are Plains" (Conversion) and "target land becomes a Forest"
(Gaea's Liege): an effect that sets a land's subtype to one basic land type
(CR 305.7). In layer 4 the land's other land types go and the new one comes,
and the land loses every ability its rules text gives it — its printed
keywords there, the rest through `Characteristics::rules_text_lost`, which
`GameObject::abilities` answers with nothing and which ends the land's own
statics except their layer-1 and layer-2 parts (the effect is layer 4, so a
layer-4 static of the land depends on it and never applies, CR 613.8a). It
makes the new type's mana through CR 305.6 alone. It is not
`LoseAllAbilities`: a keyword or ability another effect grants the land
stays, whatever its timestamp, and its card types and supertypes stay (a
basic Mountain made a Plains is still basic). "In addition to its other
types" is `AddSubtype`. The reader writes it for `RemoveLandTypes$ True`
beside one basic land type, on `S: Mode$ Continuous`'s `AddType$` and on
`Animate`'s `Types$`, and reads `Animate`'s `Duration$ UntilHostLeavesPlay`
as `Duration::WhileSourceOnBattlefield`. `SetLandTypeToChosen` is "enchanted
land is the chosen type" (Phantasmal Terrain): the same, for the basic land
type the effect's source was given as it entered
(`EnterModifier::ChooseBasicLandType`), and nothing while none was chosen;
the reader writes it for `AddType$ ChosenType` only on a card that asks that
question.

`DrawLimitPerTurn { who, limit }` is "each player can't draw more than one
card each turn" (Spirit of the Labyrinth) and its opponents-only twin
(Leovold). `who` is an ordinary `PlayerRel` read from the **effect's**
controller, so `EachPlayer` includes whoever played the card and
`EachOpponent` does not — a card written from the wrong sentence stops its
own draws or fails to stop them.

`CantLoseLife { who }` is "players can't lose life this turn" (Everybody
Lives!, `EachPlayer`), and its `who` is read the same way. The engine checks
it at the two doors every life total goes through. `GameState::change_life`
refuses the loss whatever caused it: damage, an effect, anything else. The
damage itself is still dealt. `GameState::can_pay_life` refuses a payment
before it is made (CR 119.8), which also caps a pay-X-life cost at X = 0.
Either `who` may only name a relation the game state can answer on its own
(`lints::a_continuous_player_relation_is_one_the_state_can_answer`):
`Chosen`, `ControllerOfTarget` and `ControllerOfEvent` need a
resolution, and a continuous effect has none.

What makes it a variant rather than a replacement effect is the second
sentence of CR 121.2b: the limit "applies to individual card draws", so an
instruction to draw three under a limit of one is **partially carried out** —
one card arrives and the other two do not. The engine enforces it inside
`GameState::draw_cards`'s own loop for exactly that reason. Two such effects
do not add up and the newest does not win: the lowest limit holds, which is
what "can't" means (CR 101.2). The half CR 121.2b spends its own second half
on is not covered — a player under the limit also cannot *choose* to draw
more, nor pay a cost that draws more — and no card in the pool prints either
today.

`DoesNotUntap` is Basalt Monolith's whole special clause and needs neither a
filter beyond `Filter::This` nor a duration. It changes a **rule** and not a
characteristic — CR 502.3 lets an effect keep a permanent from untapping,
CR 613.11 puts such an effect outside the layer order — so it sits in the
`Layer::Text` bucket with the other rules-modifiers and `progress::untap_step`
reads it. Whose untap step is not a field: CR 502.3 only untaps the active
player's permanents, which is the same player every printing of the sentence
names. It is **not** the way to say "doesn't untap during your *next* untap
step" — that is a created effect with `Duration::UntilYourNextUntapStep`
(`Effect::continuous(&Filter::This, Modifier::DoesNotUntap, …)`).

`MayChooseNotToUntap` is the other half of the same rule and the storage
lands' clause: CR 502.3 has the active player *determine* which of their
permanents untap, and this variant is what gives that determination a second
answer. The engine then suspends inside the untap step with a
`Pending::ChooseCards` whose answer names what stays tapped — no priority is
granted, which CR 502.4 forbids and this is not. It takes `Filter::This` and
no duration like its neighbour, and the two compose: a permanent an effect
already keeps from untapping is left off the menu, because both answers to
that question would do the same thing.

`SkipUntapStep { who }` is Stasis's "players skip their untap steps". A
skip is a replacement effect that replaces the step with nothing (CR 614.1b,
614.10), but it is written as a static ability because nothing is put in the
step's place: `progress::untap_step` asks it first and goes straight on to
the upkeep, so no permanent phases (502.1), the day/night check does not run
(502.2) and nothing untaps (502.3). `who` is read from the effect's
controller like `CantLoseLife`'s. An effect lasting "until your next untap
step" is not spent by a skipped one (CR 614.10a): it waits for the first
untap step that happens, which is why the skip does not pass through
`finish_untap_step`. The reader writes it from `R:Event$ BeginPhase |
Phase$ Untap | Skip$ True` on the battlefield with no player named.

`UntapAtMost { who, of, count }` is "players can't untap more than one
creature during their untap steps" (Smoke; Winter Orb's lands, with the
Orb's "as long as this is untapped" as the static's `condition`). It limits
CR 502.3's determination and nothing else: everything still untaps by
default, so the active player names *which* permanents counted by `of`
untap (`ChoicePrompt::Untap`, at least one per answer), and the question is
asked again until no limit has room for anything left. Limits add up and do
not merge: a permanent counts against every limit it matches (the Smoke and
Winter Moon rulings), and two copies of one limit still let `count` through.
The "may choose not to untap" question (`LeaveTapped`) is asked first, so a
permanent kept tapped by choice takes no room. Nothing is asked when every
limit can take everything still tapped. The reader writes it from
`Affected$ <player> | AddKeyword$ UntapAdjust:<valid>:<n>`.

`AttacksDespiteDefender` and `AttacksAsThoughHaste` are "can attack as
though it didn't have defender" (Animate Wall) and "…as though it had
haste" (Instill Energy), on the creatures the static's filter names. An "as
though" effect applies only to what it states (CR 609.4), so
`combat::can_attack` reads them only where defender (CR 702.3b) or summoning
sickness (CR 302.6) would stop the attack: a "can't attack" from anything
else still holds, and the creature's {T} abilities still wait (CR 702.10c is
not part of it). The reader writes them from `S:Mode$ CanAttackDefender` and
`S:Mode$ CanAttackIfHaste` with `ValidCard$`.

`GainControl` is layer 2 and must be paired with `Layer::Control` — any
other layer applies it out of order with respect to the effects that read
the controller. With `Filter::This` and `Duration::UntilEndOfTurn` it is
Act of Treason; with `WhileSourceOnBattlefield` it is Mind Control. Use it
rather than `Effect::ChangeController` whenever the control comes back:
the one-shot effect never returns the permanent.

`PlayLandsFromGraveyard` and `ExtraLandDrops` are the two halves of a land
drop and are deliberately not one variant. CR 305.1 says where a land may be
played from and CR 305.2 says how many, and a card prints one without the
other: Crucible of Worlds and Ramunap Excavator grant the zone and no extra
drop, Exploration grants the drop and no zone. Merging them would make
Crucible a second land drop, which CR 305.2b forbids in those words — "a
player can't play a land, for any reason, if the number of lands the player
can play this turn is equal to or less than the number they have already
played." `ExtraLandDrops` carries a `u8` because the rule is written to be
modified, and two of them add up (Exploration beside Azusa) rather than the
larger winning. Both take `Filter::Any`: they are about their controller and
about no object, which is how every player-scoped modifier here is read.

Note that `PlayLandsFromGraveyard` is **not** `GrantsFlashback` with a wider
filter. Flashback grants *casting* a card from a graveyard; playing a land is
not casting anything at all (CR 305.1), and the two sentences share no code —
one goes through `casting::can_cast` and the stack, the other through
`casting::play_land` and no stack.

New `Modifier` variants must be added to FIVE places, and the compiler names
four of them because those matches are exhaustive on purpose: `Modifier::layer`
in `baylee-cards-dsl`, `effects::locks_its_set`, the "handled elsewhere" arm
in `layers.rs`, and the modifier hash in `state.rs`. The fifth is the one
nothing will ask for — whatever system enforces the rule (SBAs, combat,
casting) — and a variant with no enforcer is a static ability that compiles,
hashes, layers and does nothing. This paragraph said THREE until
`PlayLandsFromGraveyard` was added and the compiler named two more.

### Pieces added for Maik's European Highlander (29.09.2026)

- **Transform.** `Effect::TransformSource` turns the source over now, and
  `Effect::TransformSourceAtNextUpkeep` does it at the beginning of the next
  upkeep (Archangel Avacyn). Either is ignored once the permanent has
  transformed since the ability was put on the stack or created
  (CR 701.27f). The face that turns away takes its statics and replacement
  effects with it (`GameState::transform`, CR 604.2). `Trigger::TransformsIntoThis` is "whenever this
  creature transforms into [this face]" (Huntmaster of the Fells). The
  conditions `NoSpellsCastLastTurn` and `APlayerCastLastTurnAtLeast(n)` are
  the werewolf upkeep checks.
- **`PlayerRel::ControllerOfEvent`** is the controller of the event's object
  (Massacre Wurm: "its controller loses 2 life").
- **`Effect::CreateTokenCopyOfTarget { mods, sacrifice_at_next_end_step }`**
  (Kiki-Jiki) copies the first target's copiable values (CR 707.2), applies
  `mods` (for example `CopyMod::AddKeyword(HASTE)`), and can register a
  delayed "sacrifice it at the beginning of the next end step". That delayed
  sacrifice is `DelayedAction::Sacrifice { card, version }`. It does nothing
  if the token has left the battlefield or changed controller.
- **`Effect::RevealTopAndSort { filter, matched, otherwise }`** (Coiling
  Oracle, CR 701.20a) reveals the top card of your library. It puts the card
  where `matched` says if it matches `filter`, and where `otherwise` says if
  it doesn't. The `SearchDest` values are the ones a library search uses.
- **`Effect::LookAtTopMayPut { filter, matched, otherwise }`** (Risen Reef)
  is its "look" and "you may" sibling: nothing is revealed, and a matching
  top card is a `ChooseCards` of that one card with `min: 0` (prompt
  `PutOntoBattlefield` for a battlefield `matched`), so only the asked
  player sees it. Named, it goes where the `Find` says (tapped if the find
  is); not named, or not matching, it goes `otherwise`.
- **`Effect::DiscardUpToThenDraw { count }`** (Fable of the Mirror-Breaker's
  chapter II): "You may discard up to `count` cards. If you do, draw that
  many cards." One `ChooseCards` over the hand, `min: 0`, prompt `Discard`;
  each named card still in hand is discarded (`GameEvent::Discarded`), and
  the draw is the number actually discarded. An empty hand asks nothing.
- **`Effect::SearchLibraryOrGraveyard { filter, find }`** (Finale of
  Devastation): "search your library and/or graveyard for a [filter] card and
  put it [where `find` says]. If you search your library this way, shuffle."
  The graveyard is public, so its matches come first: a `ChooseCards` with
  `min: 0`, `max: 1`, prompt `FromGraveyard`. A card named there is the whole
  search, and nothing is shuffled. Naming none, or a graveyard with no match,
  is the one-card library search `SearchLibrary` makes, shuffle and all, so a
  library that was seen is always shuffled.
- **`Condition::XAtLeast(n)`** is "if X is `n` or more", read off the
  announced X on the source, where `Filter::CmcAtMostX` reads it. A source
  that is gone or announced no X counts as X = 0. Finale of Devastation puts
  its +X/+X and haste behind it in an `Effect::IfCondition`.
- **`Effect::RevealUntil { filter, found }`** (Nissa, Resurgent Animist):
  "reveal cards from the top of your library until you reveal a [filter]
  card. Put that card [`found`] and the rest on the bottom of your library in
  a random order." Nobody is asked anything. Every card turned over is in one
  `GameEvent::Revealed`, the match goes where `found` says, and the table's
  generator orders the rest on the bottom. A library with no match reveals
  every card and puts all of them on the bottom.
- **`Effect::IfResolvedTimesThisTurn { times, then }`** is "if this is the
  `times`th time this ability has resolved this turn". The engine counts each
  resolution of each ability of an object (`PerTurn::resolved`, keyed on the
  source's id and version, CR 400.7) as the ability begins to resolve, so the
  resolution asking is counted. The branch runs at exactly `times`; a third
  resolution is not the second.
- **"When you cast this spell"** is `Trigger::SpellCast(&Filter::This)`. A
  trigger that cannot fire from the battlefield works from the stack
  (CR 113.6k), so `trigger::collect` asks each spell cast in the batch for
  these abilities, and only these. Its other abilities do not work there.
- **`Effect::Cascade`** is cascade's effect (CR 702.85a), the body of such a
  trigger. "Cascade, cascade" is two of them (CR 702.85c). It exiles from the
  top until a nonland card whose mana value is less than the spell's (X
  included while the spell is on the stack). Then it asks
  `YesNoPrompt::CastWithoutPaying`.
  - A yes is a `DelayedWhen::AsResolutionEnds` entry, which
    `finish_resolution` hands to the engine ahead of everything queued. The
    engine casts the card through `start_permitted_free_cast`, which asks for
    modes and targets and ignores timing.
  - A card with nothing to target goes to the bottom instead.
  - Every card not cast goes to the bottom in a random order.
- **`Effect::MayCastTarget { then_no_more_spells }`** is "you may cast that
  card" about the ability's first target, paying its costs (Conduit of
  Worlds, CR 608.2g). It asks `YesNoPrompt::CastPaying`.
  - A yes is a `DelayedAction::CastPaying` at
    `DelayedWhen::AsResolutionEnds`, which opens a CR 605.3a payment window
    for the card's mana cost. Passing it starts `start_paid_cast`: the
    card's own cost, paid out of the pool, timing ignored, X, targets and
    modes asked as usual. A pool that cannot pay casts nothing.
  - `then_no_more_spells` is "If you do, you can't cast additional spells
    this turn": the finished cast sets `PerTurn::no_more_spells`, and
    `casting::may_begin_casting` refuses every door a cast comes through
    after it (the offer, free casts, miracle, a prepared copy).
  - "If you haven't cast a spell this turn" is
    `Condition::YouCastNoSpellThisTurn`, wrapped around it in an
    `Effect::IfCondition`.
- **`Modifier::CastPermanentSpellsFromGraveyard`** is "you may cast permanent
  spells from your graveyard" (Wrenn and Realmbreaker's emblem), uncounted
  and at the card's own price. `casting::graveyard_cast_permission` is the
  one reader: the offer, `can_cast_form` and the cast wizard all ask it.
- **`Modifier::CastSpellsFromGraveyard`** is the same permission without the
  word "permanent": Forgotten Cellar's "you may cast spells from your
  graveyard this turn", written `Effect::continuous(&Filter::Any,
  Modifier::CastSpellsFromGraveyard, Duration::UntilEndOfTurn)`. The same
  reader asks it first, before the permanent-only permissions. It is not
  flashback (CR 702.34a): an instant cast under it is not exiled by it, and a
  card with printed flashback is offered at its mana cost beside its
  flashback cost. A land card is played, never cast (CR 305.9).
- **`Modifier::PermanentOfEachTypeFromGraveyard`** is Muldrotha's "during
  each of your turns, you may play a land and cast a permanent spell of each
  permanent type from your graveyard". The engine writes each play under it
  down (`PerTurn::graveyard_plays`, per source and version) and allows a
  cast while the spells cast so far and this one can each be given a type of
  their own. So an artifact creature takes whichever type is still open, and
  the choice the rules make as it is cast (Muldrotha's ruling) is left open
  until a later spell needs it. Land is a play of its own, once a turn.
- **Emblem statics** register from the command zone (CR 114.4), once, with
  `Duration::Indefinitely` (`progress::emblem_statics`). Before this, an
  emblem's static ability compiled and did nothing.
- **`ActivationZone::Graveyard`** is an ability activated from its owner's
  graveyard (eternalize, embalm). The offer walks the graveyard beside the
  hand, with the same arms, and `ExileSelf` pays "exile this card from your
  graveyard".
- **`Effect::CreateTokenCopyOfSource { mods }`** is "create a token that's a
  copy of it, except …" where "it" is the source card, wherever the cost put
  it (CR 707.2). The new `CopyMod`s are `SetPT(p, t)`, `SetColor(colors)`
  (replaces the colors) and `NoManaCost` (mana value 0), all copiable values
  of the token (CR 707.9b). Fanatic of Rhonas writes eternalize with them.
- **`Effect::MillMayTakeOne { amount, filter }`** is "mill `amount` cards. You
  may put a [filter] card from among the milled cards into your hand" (Wrenn's
  −2). A `ChooseCards` with `min: 0`, `max: 1`, prompt `PutIntoHand`, over the
  milled cards that match, found in the public zone they moved to
  (CR 701.17c): the graveyard, or wherever a replacement sent them.
- **`Effect::RevealAndSeparate { count }`** is "reveal the top `count` cards
  of your library. An opponent separates those cards into two piles. Put one
  pile into your hand and the other into your graveyard" (Fact or Fiction).
  The opponent answers a `ChooseCards` (prompt `FirstPile`, `min: 0`, any
  number: the named cards are the first pile, the rest the second), and the
  controller a `Pending::ChoosePile`, answered with `ChooseMode(position)`.
  A pile may be empty (CR 700.3d); the cards stay in the library until the
  choice (CR 700.3c). At a table with several opponents the controller first
  names the one who separates (`ChoosePlayer`). The client draws the piles
  as rows naming their cards, and the reveal's sheet stays open under the
  question.
- **`Modifier::CharacteristicPT { count, toughness_plus }`** is a
  characteristic-defining P/T (layer 7a, CR 613.4a). `count` is a `PtCount`:
  - `YouControl(filter)`
  - `CardTypesInAllGraveyards`
  - `ExiledWithThis`: the cards in exile exiled with this object as it is now
    (`ExileTargetsWithSource`; a Hearse that left and came back counts none).

  Power is the count, and toughness is the count plus `toughness_plus`
  (Pyrogoyf: `+1`). A characteristic-defining ability works in every zone
  (CR 604.3), so this one does too: written on the card itself
  (`Filter::This`, no condition, front face), it is read at setup into
  `GameState::printed_pt_cda`, and the projection applies it to the card in
  a library, a hand, a graveyard, exile or on the stack. On the battlefield
  it is an ordinary registered static, so an effect that removes abilities
  removes it. Recruiter of the Guard's "toughness 2 or less" and
  Reveillark's "power 2 or less" read the real number. A graveyard or exile
  change, and every move of such a card, invalidates the projection.
  `SetPTToCount` is granted, so it is never characteristic-defining (see
  above) and stays a battlefield static.
- **`Effect::EventObjectDealsDamageEqualToPower { target }`**: "that creature
  deals damage equal to its power to any target". The dealer is the event's
  object, and its power is read now, or as it last existed on the
  battlefield (`GameState::ltb_powers`).
- **`Modifier::CantBeBlockedBy(filter)`** is "can't be blocked by [filter]".
  Examples: Questing Beast (`PowerAtMost(2)`) and Delney (`PowerAtLeast(3)`).
  `combat::can_block` enforces it.
- **`Filter::PowerLessThanSourcePower`** and
  **`Filter::ToughnessLessThanSourcePower`** compare with the source's
  projected power, strictly: Stone Giant's "target creature you control
  with toughness less than this creature's power". A source with no power
  bounds nothing in. The reader writes them from `powerLTX` and
  `toughnessLTX` only where `X` is `Count$CardPower`.
- **`Effect::AtNextEndStep { effects }`** is "[effects] at the beginning of
  the next end step", a delayed trigger (CR 603.7) with this ability's
  source and controller that uses the stack. "That creature" in `effects`
  is `TargetSpec::EventObject`, the first target as the object it was when
  this resolved; one that has left its zone since is a new object and is
  not affected (CR 603.7c, 400.7). Stone Giant: `AtNextEndStep { effects:
  &[Effect::destroy(TargetSpec::EventObject)] }` after its pump. The reader
  reads `AtEOT$ Destroy` on a targeted `Pump` only. An ability that never
  said "target" has its **source** there instead, as the object it is as
  this resolves: Dragon Whelp's "sacrifice this creature at the beginning
  of the next end step" is `AtNextEndStep { effects:
  &[Effect::SacrificeObject { target: TargetSpec::EventObject }] }`, and a
  Whelp that left and came back is not sacrificed.
- **`Effect::AtEndOfCombat { about, effects }`** is "[effects] at end of
  combat", a delayed trigger (CR 603.7) with this ability's source and
  controller that triggers as the next end of combat step begins
  (CR 511.2) and uses the stack. `about` **names** the object it
  remembers, read as this resolves (`EventObject`, `ThisObject`, or a
  target spec for the first target), and "that creature" in `effects` is
  `TargetSpec::EventObject`, that object as it was then (CR 603.7c).
  Named rather than derived, as `AtNextEndStep` derives it, because one
  triggered ability can have a target, an event object and a source.
  Cockatrice: `AtEndOfCombat { about: TargetSpec::EventObject, effects:
  &[Effect::destroy(TargetSpec::EventObject)] }`.
- **`Trigger::BlocksOrBecomesBlockedBy(filter)`** is "whenever this
  creature blocks or becomes blocked by a [filter] creature": once per
  blocker–attacker pair (CR 509.3b, 509.3d), so blocked by two it
  triggers twice. The filter is the **other** creature, as it is when the
  block is declared (CR 509.3f), and that creature is the event object —
  whichever side of the block this creature is on. The reader reads the
  reference's two-line spelling (`AttackerBlockedByCreature`, one line
  per side, the second `Secondary$ True`) as one ability, and only whole:
  either half alone is another sentence and is refused.
- **`Effect::SacrificeObject { target }`** is "sacrifice that creature": the
  ability's controller sacrifices the object the spec names, only if they
  control it, it is on the battlefield and phased in (CR 701.21a).
  `SacrificeSelf` is the source by id and cannot tell a source that came
  back from the one the delayed trigger was about.
- **`Effect::IfActivatedThisTurnAtLeast { n, then }`** is "if this ability
  has been activated `n` or more times this turn" (Dragon Whelp). It counts
  **activations** (CR 602.2), taken as each is put on the stack and paid
  for, in `GameState::ability_fires` — only for an ability whose effects
  carry this branch, the way "activate only once each turn" is counted — so
  four stacked activations all count before the first resolves. A source
  that left the battlefield has a fresh count (CR 400.7).
- **`Modifier::CombatDamageCantBePrevented`** makes combat damage dealt by the
  matching creatures unpreventable. It overrides prevention effects and
  protection's prevention (CR 615.12, 702.16e).
- **`Effect::ProtectionFromChosenColor { duration }`** (Sejiri Steppe) asks
  the controller for a color as it resolves (`Pending::ChooseColor`). It then
  grants layer-6 protection from that color to the first target for
  `duration`.

### Pieces added for Maik's deck, second round (29.09.2026)

Choosing modes:

- **`AbilityDef::ModalSpell { modes, choose }`.** `choose` is a `ModeCount`
  saying how many modes are chosen (CR 700.2):
  - `ModeCount::ONE` is "Choose one —". Every modal spell before Farewell
    has it.
  - `ModeCount::TWO` is "Choose two —".
  - `ModeCount::ONE_OR_MORE` is "Choose one or more —", and also what spree
    means (CR 702.172a).

  A spell that chooses several is cast as one set of modes
  (`CastModeKind::Modes(bits)`, bit `i` for mode `i`, no mode twice,
  CR 700.2d). The chosen modes happen in printed order (CR 608.2c),
  whatever order they were picked in. At most two chosen modes may say
  "target": the first takes the spell's first instance of the word, the
  second its second (CR 700.2c, 115.3). The second may name objects only,
  and no mode of such a spell may print `second_targets` of its own.
  `lints::modes_fault` holds all of this.
- **`SpellMode::additional_cost`** is the cost printed before a mode:
  spree's "+ {1} —" (CR 700.2h). It is added to the card's cost when the
  mode is chosen. It is not an alternative cost, so a spell cast without
  paying its mana cost still pays it (CR 118.9d). It never goes with
  `cost_override` (overload), and only on a spell that chooses several.
  Codegen reads the "+ {cost} —" lines as that card's list of modes, the way
  it reads bullets.

Effects:

- **`Effect::ExileAll { filter }`** is "Exile all [permanents]" (Farewell).
  Nothing is targeted.
- **`Effect::ChooseYoursThen { filter, then }`** is "Choose a creature you
  control. It …" (Final Showdown). The choice is made as the effect
  resolves (CR 608.2d), is not a target, and must be made if it can be.
  Inside `then`, `Filter::This` is the chosen permanent. With nothing to
  choose, nothing happens (CR 609.3). `then` holds only effects that ask
  nothing, which is `lints::chosen_then_fault`; today that means
  `CreateContinuousEffect`.
- **`Effect::DealDamageDivided { amount }`** is "deals N damage divided as
  you choose among any number of targets" (Fury). The division is asked as
  the triggered ability goes on the stack, one share per target
  (`Pending::ChooseNumber` with `NumberPrompt::DivideDamage`), and the last
  target takes the rest. A target that is illegal at resolution gets
  nothing, and its share goes to nobody (CR 608.2b). Only a triggered
  ability may divide (`lints::every_divided_damage_is_a_trigger_that_can_divide`).
- **`Effect::Discover { mana_value }`** is discover N (Trumpeting
  Carnosaur, CR 701.57a). The cast is offered after the resolution, before
  anyone gets priority. A modal spell cast this way picks its mode, and a
  spree spell pays its mode costs.
- **`Effect::RevealTopOnePerType { count }`** (Atraxa, Grand Unifier) asks
  one question per card type among the revealed cards (CR 205.2a).
- **`Effect::MayDoOnceEachTurn { effects }`** is "You may …. Do this only
  once each turn." (The Reaper, King No More). Only a yes uses the turn's
  go.
- **`Effect::NthResolutionThisTurn { effects }`** runs the nth effect on
  the ability's nth resolution this turn (Omnath, Locus of Creation).
- **`Effect::IfTargetMatches { filter, then }`** is "… target … if it's
  [filter]" (Prismatic Ending). It is not a targeting restriction.
- **`Effect::OwnerPutsOnTopOrBottom { target }`** (Subtlety): the owner,
  not the controller, picks the end of the library.
- **`Effect::ExileIfDiesThisTurn { target }`** (Mawloc) is a replacement
  effect on that object for the rest of the turn. The reader writes it from
  `ReplaceDyingDefined$ Targeted` on any line with an object target
  ("if that creature would die this turn, exile it instead"), and
  `ThisTargetedCard.Creature` inside `IfTargetMatches { CREATURE }`
  (Disintegrate's "if it's a creature"). `Remembered` ("a creature dealt
  damage this way") is refused: it asks whether damage was dealt.
- **`Effect::CantBeRegeneratedThisTurn { target }`** (Disintegrate) is "it
  can't be regenerated this turn" (CR 701.19c): a shield on that object
  still stands and saves nothing, for the rest of the turn and that object
  only. "Destroy … It can't be regenerated" is `destroy_no_regen`, not
  this. The reader writes it from an `Effect` whose one static is
  `CantRegenerate` on what it remembers, the line's target.
- **`Effect::GraveyardAllToHand { filter }`** (Garna, the Bloodflame)
  returns every matching card in your graveyard. Nothing is targeted.

Triggers, reflexive events and amounts:

- **`Trigger::CycledThis`** is "When you cycle this card" (CR 702.29c).
  What counts as cycling is `AbilityDef::is_cycling`: from the hand, the
  cost discards the card, and the effect draws one card. Typecycling is not
  read.
- **`Trigger::DealsCombatDamageToOpponent(filter)`** (Questing Beast)
  carries the player and the amount on the trigger. **`Amount::EventAmount`**
  is "that much".
- **`Trigger::DealsDamageToOpponent(filter)`** is the same with any damage,
  combat or not (Hypnotic Specter: "whenever this creature deals damage to
  an opponent, that player discards a card at random", `PlayerRel::DamagedPlayer`).
- **`Trigger::DealtDamage(filter)`** is "whenever [a permanent] is dealt
  damage" (Fungusaur). All combat damage in a step is dealt at once
  (CR 510.2), so a creature blocked by three triggers it once (CR 603.2c);
  every other damage event triggers it once, and prevented damage never.
- **`Trigger::PlayerDealtDamage(rel)`** is "whenever you're dealt damage"
  (Living Artifact, Lich), the player's side of `DealtDamage`: once for a
  step's combat damage to that player and once for every other damage
  event. "That many" is `Amount::EventAmount`, which for combat is the
  step's whole total to that player, not the first attacker's share.
- **`ReflexiveEvent::ExiledThis`** is "You may exile it. When you do, …"
  (The Balrog of Moria). The action is `Effect::ExileSource`.

Targets:

- **`TargetSpec::OpponentOrObject(filter)`** is "target opponent or
  [filter]" (Ravager of the Fells).
- **`TargetSpec::ObjectOfFirstTargetsPlayer(filter)`** is a second instance
  of "target" limited to the player the first instance named. It is
  written only as `second_targets`, which triggered abilities now have too.
- **`TargetSpec::ObjectOfEventPlayer(filter)`** is "target [filter] that
  player controls", where that player is the one the event dealt damage to
  (Questing Beast).
- `TargetSpec::ObjectControlledBy(filter, player)` is what the engine binds
  those two to. It is never written on a card.

Filters, conditions, modifiers and durations:

- **`Filter::PutIntoGraveyardThisTurn`** (Garna).
- **`Filter::HasCounter(kind)`** (The Reaper). For a permanent that just
  left the battlefield it reads the counters it had as it left
  (CR 603.10a).
- **`Filter::CmcAtMostColorsSpent`** is converge's count (Prismatic
  Ending).
- **`Condition::XAtLeast(n)`** is "if X is N or more", read off the
  source's announced X.
- **`Modifier::ModifyPTPerGraveyardCard { filter, p, t }`** (Fiend
  Artisan) is layer 7c.
- **`Duration::WhileYouControlSource`** is "for as long as you control this
  creature" (Extraction Specialist, CR 611.2b).

### Pieces added for the friends' decks, last round (29.09.2026)

- **`Effect::TapAll { filter }`** is "Tap all [permanents]" (Cryptic
  Command's "Tap all creatures your opponents control", with
  `Filter::OPPONENT_CREATURE`). Nothing is targeted, so hexproof does not
  stop it; a permanent already tapped stays as it is (CR 701.26a). Cryptic
  Command is `ModeCount::TWO`: a pair of its four modes, each pair at the
  card's own cost.
- **`Effect::ExileTopMayCast { who }`** is "exile the top card of [who]'s
  library. Until end of turn, you may cast that card" (Ragavan, Nimble
  Pilferer). The card goes to its owner's exile face up, and the controller
  holds a cast-only `PlayPermission` for it: a spell is cast at its own
  price and timing, and a land is neither played nor cast (CR 601.1a,
  305.9). Nothing is targeted.
- **`PlayerRel::DamagedPlayer`** is "that player" of a trigger on damage
  dealt to a player (`Trigger::DealsCombatDamageToPlayer` and its
  siblings): the seat the damage went to, read off the triggered ability.
  At a table of three it is the one Ragavan hit, not "an opponent".
- **`FaceDef.dash: Option<ManaCost>`** is "Dash [cost]" (CR 702.109a). The
  cast offers `CastModeKind::Dash` beside the mana cost, from wherever the
  card may be cast. The engine writes the rest: the permanent the spell
  becomes has haste, and a delayed trigger returns it to its owner's hand at
  the beginning of the next end step, if it is still that permanent (a
  blinked or bounced one is a new object, CR 400.7). No card writes the
  haste or the return. `Condition::DashCostPaid` is what that trigger asks;
  no card prints it.
- **`FaceDef.escape: Option<Escape>`** is "Escape—[mana], Exile [N] other
  cards from your graveyard" (CR 702.138a): `Escape { cost, exile }`, the
  shape every printed escape cost has. From its owner's graveyard the cast
  offers `CastModeKind::Escape` once the mana is affordable and at least
  `exile` other cards lie there; the cast then asks which, as
  `ChoicePrompt::CostExile` with `min == max == exile`, and exiles them after
  the mana is paid. Beside a graveyard permission (Muldrotha) the mana cost is
  offered too, as `Normal`. From a hand nothing changes.
- **`Condition::Escaped`** is "unless it escaped" and "if it escaped"
  (CR 702.138b): the source is the spell cast with escape or the permanent
  it became. A blinked or bounced one is a new object (CR 400.7) and did not
  escape. Uro's "sacrifice it unless it escaped" is
  `IfCondition { condition: Escaped, then: &[], otherwise: &[SacrificeSelf] }`
  on an enters trigger. "Escapes with" counters (CR 702.138c) are not
  written yet.

### Pieces added for Limited Edition Alpha (30.09.2026)

- **`PtCount::OnBattlefield(&filter)`** counts every permanent on the
  battlefield the filter matches, whoever controls it: Plague Rats' "the
  number of creatures named Plague Rats on the battlefield", beside
  `YouControl`'s one side of the table.
- **`Effect::UntapAll { filter }`** and **`Effect::RegenerateAll { filter }`**
  are `TapAll`'s mirror and a regeneration shield (CR 701.19a) on every
  permanent the filter matches as the effect resolves, targeting nothing.
  An Aura's "untap enchanted creature" (Instill Energy) and "regenerate
  enchanted creature" (Regeneration) name the host through
  `Filter::AttachedToBySource`.
- **`Condition::BattlefieldCount(&filter, n)`** and
  **`BattlefieldCountAtMost(&filter, n)`** are `ControlCount` and
  `ControlCountAtMost` over the whole battlefield: Pestilence's "if no
  creatures are on the battlefield" is `BattlefieldCountAtMost(&CREATURE, 0)`.
- **`Filter::ControlledByActivePlayer`** matches what the player whose turn
  it is controls (CR 102.1): Karma's "the number of Swamps they control", at
  the beginning of each player's upkeep. It is not a relation to "you".
- **`Filter::CmcExactlyX`** is "mana value X" read off the source's
  announced X, where `CmcAtMostX` reads "X or less" (Spell Blast). X is
  announced before targets are chosen (CR 601.2b, 601.2c), so the target
  menu is the objects of the X just announced; the cast is offered while
  any X would find one.
- **`Effect::PlayerMayPayManaOr { player, cost, effect }`** and
  **`PlayerMayPayManaThen { player, cost, effects }`** are `PlayerMayPayOr`
  and `PlayerMayPayThen` with a printed price that has colour in it:
  Phantasmal Forces' "sacrifice it unless you pay {U}" (`cost: mana!("{U}")`),
  Force of Nature's `{G}{G}{G}{G}`, Farmstead's "you may pay {W}{W}. If you
  do, …". "Unless" means "may pay; if they don't" (CR 118.12a), and the
  question is put and paid exactly as the generic tax's, CR 605.3a window
  included; only the pool that can pay differs, since two red do not pay
  `{U}`. Generic prices stay on the `Amount` pair: that price may be known
  only as the ability resolves (Esper Sentinel), a printed colour never is.
- **`Effect::TapAllOf { who, filter }`** is `TapAll` over the permanents the
  players in `who` control as it resolves: Mana Short's "tap all lands target
  player controls" is `TapAllOf { who: PlayerRel::Chosen, filter:
  &Filter::LAND }` beside `targets = Some(TargetReq::one(TargetSpec::AnyPlayer))`.
  The player may be targeted; the permanents are not.
- **`Effect::LoseUnspentMana { who }`** empties each named player's mana pool
  (CR 106.4: "the player is said to lose this mana"), all of it, including
  mana an effect lets stay as steps end: the effect empties the pool, not
  the end of a step.
- **`Effect::AddManaFor { who, color, amount }`** is fixed mana in the pool
  of each player `who` names, who need not be the ability's controller:
  Gauntlet of Might's and Wild Growth's "its controller adds an additional
  {R}" (`who: PlayerRel::ControllerOfEvent` under `TappedForMana`). It is
  mana like `AddMana`'s otherwise, so a trigger that makes it with no target
  is a mana ability.
- **`Trigger::BecomesTapped(filter)`** is any permanent the filter matches
  becoming tapped, for any reason: City of Brass's `Filter::This`, Lifetap's
  "a Forest an opponent controls", Psychic Venom's enchanted land. The
  tapped permanent is the event's object: `PlayerRel::ControllerOfEvent` is
  "that land's controller".
- **`Effect::ToggleTapTarget`** taps each untapped target and untaps each
  tapped one. "Tap or untap target permanent" is `MayDo { effects:
  &[Effect::ToggleTapTarget] }` (Twiddle), with or without a printed "may":
  only an untapped permanent can be tapped and only a tapped one untapped
  (CR 701.26a, 701.26b), so one of the two choices always does nothing and
  choosing it is declining. The yes or no is asked as the effect resolves,
  which is when the printed choice is made; a pair of modes would ask it on
  casting.
- **`Effect::DiscardHand { who }`** is "each player discards their hand"
  (Wheel of Fortune): every card of each named hand, nobody choosing, each
  journaled as a discard of its own.
- **`Effect::ShuffleIntoLibrary { who, hand, graveyard }`** moves each named
  player's hand and/or graveyard into their own library, then each of them
  shuffles (Timetwister's "each player shuffles their hand and graveyard
  into their library"; "target player shuffles their graveyard into their
  library" is `who: PlayerRel::Chosen, hand: false, graveyard: true`). A
  commander among the cards is asked about first (CR 903.9b).
  `ShuffleGraveyardIntoLibrary` is yours alone and older.
- **`Effect::ShuffleLibrary { who }`** shuffles each named library: Natural
  Selection's "you may have that player shuffle" is `MayDo { effects:
  &[Effect::ShuffleLibrary { who: PlayerRel::Chosen }] }`.
- **`Effect::ReorderTopLibraryOf { who, count }`** is `ReorderTopLibrary`
  on another player's library: the ability's controller looks at the top
  `count` cards of the first player `who` names and orders them (Natural
  Selection). Seeing them is the question's entitlement, as for a scry of
  another library.
- **`Modifier::AttacksEachCombat`** is "attacks each combat if able" (CR
  508.1d): `static_ability!(Filter::This, Modifier::AttacksEachCombat)` on
  Juggernaut, and the same modifier in an until-end-of-turn
  `CreateContinuousEffect` for a grant. It is a modifier and not a keyword
  bit, so a creature that loses its abilities keeps what another permanent
  gave it. The engine refuses a declaration that leaves out a creature that
  must attack and could.
- **Blocks** (CR 509.1a, 509.1c) are four rules modifiers.
  `Modifier::CanBlockAdditional(n)` is "can block `n` additional creatures
  each combat" (Two-Headed Giant of Foriys, `n = 1`; two such effects add
  up), `Modifier::CanBlockAnyNumber` is "can block any number of
  creatures". `Modifier::MustBeBlockedByAllAble` is Lure's "all creatures
  able to block enchanted creature do so", on the attacker:
  `static_ability!(Filter::And(&[Filter::CREATURE,
  Filter::AttachedToBySource]), Modifier::MustBeBlockedByAllAble)`.
  `Modifier::BlocksEachAttackerIfAble` is "blocks each attacking creature if
  able", on the blocker. Blaze of Glory grants the last two to its target
  until end of turn, as two `Effect::continuous(&Filter::This, …)`.
- **`Filter::ControlledByDefendingPlayer`** is "[a creature] defending player
  controls": during combat, an opponent of the active player's (CR 506.2,
  802.2); outside combat nothing matches. The client's target preview reads
  it in a duel only, as it does `ControlledByOpponent`.
- **`Modifier::CantAttackUnlessDefenderControls(&filter)`** is "can't attack
  unless defending player controls [filter]" (CR 508.1c): Sea Serpent's and
  Pirate Ship's Island. The filter is asked of the permanents of the player
  the creature would attack, so an Island of your own does nothing.
- **`Trigger::State(&condition)`** is a state trigger (CR 603.8): "When you
  control no Islands, sacrifice this creature" is
  `triggered!(Trigger::State(&NO_ISLANDS), &[Effect::SacrificeSelf])` with
  `static NO_ISLANDS: Condition = Condition::ControlCountAtMost(&ISLAND, 0);`
  above the literal (a named static, because the condition borrows a filter
  static and a promoted temporary may not). The condition is the trigger's,
  never `condition = Some(…)`: that is an intervening "if" (CR 603.4), asked
  again as the ability resolves, and a state trigger is not. It triggers
  whenever the condition holds for its controller and the ability is neither
  waiting to go on the stack nor on it.
- **Windows in the turn** (CR 506.7) are `Condition`s. On an activated
  ability, `condition = Some(…)` as always; on a spell,
  `spell!(effects, condition = Some(…))` is "cast this spell only [when]".
  `Condition::BeforeStep(StepKind::CombatDamage)` is "only before the combat
  damage step" (Berserk); "only during an opponent's turn, before attackers
  are declared" is `Condition::All(&[Condition::OpponentsTurn,
  Condition::BeforeStep(StepKind::DeclareAttackers)])` (Siren's Call,
  Nettling Imp); "only during your upkeep" is `Condition::All(&[
  Condition::YourTurn, Condition::DuringStep(StepKind::Upkeep)])`. `StepKind`
  names the steps a card can name: `Upkeep`, `Draw`, `CombatBegin`,
  `DeclareAttackers`, `DeclareBlockers`, `CombatDamage`, `End`. "Only during
  combat before blockers are declared" is `Condition::All(&[
  Condition::DuringCombat, Condition::BeforeStep(StepKind::DeclareBlockers)])`
  (Blaze of Glory).
- **`Filter::AttackedThisTurn`** is "attacked this turn": declared as an
  attacker this turn, as the object it is now (one put onto the battlefield
  attacking never attacked, CR 508.4). **`Filter::ControlledSinceTurnBegan`**
  is "its controller has controlled it continuously since the beginning of
  the turn" (CR 302.6's measure, asked of any permanent); beside
  `Filter::ControlledByActivePlayer` it is Nettling Imp's target. Both are
  history, so the house AI and the client's target preview refuse them.
- **`Effect::IfEventObjectMatches { filter, then }`** runs `then` when a
  delayed trigger's "that creature" ([`TargetSpec::EventObject`]) still is
  that object and matches `filter`: Berserk's `Effect::AtNextEndStep {
  effects: &[Effect::IfEventObjectMatches { filter:
  &Filter::AttackedThisTurn, then: &[Effect::destroy(TargetSpec::EventObject)]
  }] }`. It is `IfTargetMatches` for the event object.
- **`Modifier::ModifyPTHalfCount(count)`** is "+X/+Y, where X is half
  [count], rounded down, and Y is half [count], rounded up" (Aspect of
  Wolf), layer 7c like `ModifyPTPerCount`. "You" in the count is the
  static's controller: an Aura's "Forests you control" are the Aura's
  controller's, whoever controls the creature.
- **`PtCount::DefendingPlayerControls(&filter)`** counts what the defending
  player controls, for a creature that is attacking (CR 508.5): the player
  it attacks, or the controller of the planeswalker it attacks, also after
  that planeswalker has left (CR 506.4c keeps the creature attacking). It
  is 0 while the creature is not attacking. Declaring attackers and the end
  of combat re-project a permanent whose count this is
  (`GameState::board_state_changed`), so the count needs no condition to
  stay current. Keep the filter free of `ControlledByYou`: the count
  already names whose permanents it reads.
- **A P/T sentence that holds only "as long as" something is not
  characteristic-defining** (CR 604.3a's fifth criterion), even printed on
  the card: it is a layer 7b `SetPTToCount` static with a `condition`, and
  off the battlefield the card is its printed `*/*`, 0/0. Gaea's Liege is
  two of them, `SetPTToCount(YouControl(&FORESTS))` under
  `Condition::SourceMatches(&NOT_ATTACKING)` and
  `SetPTToCount(DefendingPlayerControls(&FORESTS))` under
  `SourceMatches(&Filter::Attacking)`: their conditions exclude each other,
  so their order never matters. `CharacteristicPT` stays for the
  unconditional printed `*/*`.

## Worked examples

A land with two basic land types must print its own mana ability. CR 305.6
grants one ability *per* basic type, and the engine's intrinsic shortcut
(`casting::intrinsic_mana`) can only return a single colour with no way to
ask which — so it declines any land with more than one, and such a land taps
for nothing at all unless the card supplies the choice itself:

```rust
abilities = &[mana_ability!(&[Effect::mana_choice(&[
    ManaColor::White,
    ManaColor::Black,
])])],
```

`baylee-cards`'s `a_land_with_two_basic_types_prints_its_own_mana_ability`
turns that into a build failure. A land with exactly one basic type (a plain
Swamp) may rely on the shortcut, though every basic in the pool prints the
ability anyway.

Every mana line is one `Effect::AddMana`, which answers three independent
questions — which colors (`ManaSource`), how much (`Amount`), and what the
mana may be spent on (`ManaRestriction`). Card files say it through the
constructors, which read like the printed line:

```rust
Effect::mana(ManaColor::Green, 1)                      // Add {G}.
Effect::mana(ManaColor::Colorless, 2)                  // Add {C}{C}.
Effect::mana_choice(&[ManaColor::White, ManaColor::Black])  // Add {W} or {B}.
Effect::mana_of_any_color()                            // Add one mana of any color.
Effect::mana_combination(COLORS, Amount::Fixed(2))     // …in any combination of colors.
Effect::mana_commander_identity()                      // …in your commander's color identity.
Effect::mana_land_color(false)                         // …any color that a land an opponent controls could produce.
Effect::mana_land_type(true)                           // …any type that a land you control could produce.
Effect::mana_dynamic(ManaColor::Black, Amount::CountOf { .. })
Effect::mana_of_any_color().restricted(&FILTER, SpendRider::Uncounterable)
Effect::mana(ManaColor::Colorless, 1).when_spent(&FILTER, SpendRider::Uncounterable)
```

`restricted` is "Spend this mana only …": the mana pays for nothing its
filter does not match. `when_spent` is "When that mana is spent to cast …" /
"If that mana is spent on …" with no "only" before it (Path of Ancestry,
Boseiju, Who Shelters All): the mana is ordinary mana and pays for anything,
and only a spell the filter matches sets its rider off, once per unit spent
(CR 106.6, 106.6a; #232). A card printing a rider without "only" never uses
`restricted`: that is the defect #232 fixed, where Path of Ancestry paid for
almost nothing.

`mana_land_color` and `mana_land_type` differ by one printed word, and the
word is a rule: colorless mana is a type of mana and not a color (CR 106.1a,
106.1b), so "any color that a land … could produce" never makes `{C}` and
"any type" does. Exotic Orchard and Fellwar Stone offered `{C}` across a
Wastes while the two were one constructor.

`mana_combination` is not decoration: "in any combination of colors" is one
color pick *per mana*, while a plain choice picks one color for the whole
amount. Harabaz Druid was written with the wrong one and paid X² mana. Its
`Amount` is the ordinary one and carries `Amount::X` as readily as a number:
the Time Spiral storage lands print "Add X mana in any combination of {W}
and/or {U}", where X is what their own `RemoveCounterSelfX` announced, and
`resolve::mana::add_mana` splits whatever the amount evaluates to into that
many picks of one.

An amount with a **constant in front of it** is `Amount::Plus`, wrapping the
amount it offsets — Muscle Burst's "3 plus the number of cards named Muscle
Burst in all graveyards":

```rust
static COPIES: Amount = Amount::Plus {
    base: &Amount::CountOf {
        filter: &Filter::Named("Muscle Burst"),
        zone: ZoneSel::GraveyardAll,
    },
    offset: 3,
};
```

It saturates rather than wraps, because a count is a count. It is a wrapper
for the same reason the next one is: the offset is said once instead of
doubling every counting amount there is.

An amount that counts **downwards** is `Amount::Negated`, wrapping the amount
it negates:

```rust
Effect::PumpTarget {
    power: Amount::Negated(&Amount::CountOf {
        filter: &Filter::YOUR_ARTIFACT,
        zone: ZoneSel::Battlefield,
    }),
    // …and the same for `toughness`.
}
```

`Amount::NegX` and `Amount::NegXFixed` are the two negatives that came before
it, and each is a variant of its own magnitude; there is one `CountOf`, so a
card wanting its negative had nothing to write and Irradiate stayed a stub.
The wrapper says the sign once instead of doubling every amount there is.

Two things follow that a card file has to respect. **The sign is not in the
number**: `eval::amount` answers a magnitude, because the engine consumes one
at eighteen sites and seventeen of them are counting cards to draw or tokens
to make, where a negative has nothing to mean. Whatever reads a signed amount asks
`Amount::is_negative`, and never `matches!` on the negative variants — three
places did, and each would have read `Negated` as a bonus. **And it only
means anything where something asks.** Six of `Effect`'s twenty-two `Amount`
fields carry a sign anything reads — `SetPTFilter`, `PumpFilter` and
`PumpTarget`, through `power` and `toughness` — all three through
`resolve::counters::signed`. A
`Negated` in `DrawCards`, `GainLife` or `PlayerMayPayOr` compiles, claims
`Coverage::Implemented`, resolves, and does the *positive* thing:
`amount_sign_tests::every_negated_amount_in_the_pool_sits_in_a_field_that_reads_the_sign`
is what turns that into a build failure.

Fetchland (activated with composite cost + filtered search):

```rust
abilities = &[activated!(
    cost!(TapSelf, SacrificeSelf, PayLife(1)),
    &[Effect::SearchLibrary {
        filter: &LAND_TYPE_PAIR,
        finds: &[Find::BATTLEFIELD],
        optional: false,
    }]
)],
```

Note what is *not* written: this ability uses the stack, is activatable at
instant speed and functions on the battlefield, and all three are what the
rules already say. Only the cost and the effect are the card.

And note the one word that *is* the card. `Find::BATTLEFIELD` against
`Find::BATTLEFIELD_TAPPED` is the whole difference between the two families
of land that this example otherwise fits both of: a fetchland pays a life and
puts its dual in untapped, Evolving Wilds pays nothing but itself and puts a
basic in tapped. This snippet said `BATTLEFIELD_TAPPED` under the heading
"Fetchland" until four of the ten fetchlands in the pool had copied it
(`docs/observed-faults.md` §54). `xtask validate` now reads every `finds:`
list against the printed "onto the battlefield tapped", so the next one fails
the gate rather than the game.

A search says *what* it looks for and *where* each find goes; the two rules
every printed search also obeys are derived, not declared, so a card cannot
get them wrong:

- **The library is always shuffled afterwards.** Of the 1015 printed
  searches in the scripts reference, three do not say "then shuffle", and all
  three empty the library instead.
- **A find is revealed** when the search is narrower than "a card"
  (`Filter::Any`) *and* at least one destination is hidden (hand, top of
  library). Mystical Tutor reveals; Demonic Tutor does not; a fetchland does
  not, because the battlefield is public anyway. The reveal is journalled as
  `GameEvent::Revealed` — it is what holds the searcher to the filter.

Rally trigger (filter "self or another Ally you control"):

```rust
use crate::filters::YOUR_ALLIES;

abilities = &[triggered!(
    Trigger::EntersBattlefield(&YOUR_ALLIES),
    &[Effect::gain_life(1)]
)],
```

The filter is shared rather than restated: the rally wording is "this
creature **or another** Ally", so the source is part of the match, and six
card files had each written that out. A seventh writing `Another` instead
would have been a silent rules bug in a card that still compiled.

Static anthem via layers (deregisters itself when the source leaves):

```rust
abilities = &[static_ability!(Filter::YOUR_CREATURE, Modifier::ModifyPT(1, 1))],
```

There is no layer here and no `cross_zone` either. The layer is derived
(CR 613.1 — see below), and the flag is gone: an effect that reaches past the
battlefield says so with a `Filter::InZone`, which is the only statement the
engine reads.

## Explicitly not supported yet (M3+)

This list is hand-kept, and it has a **measured counterpart**.
`data/card-refusals.tsv` carries 121 rows from the card batch of 4.–5.09.2026,
115 of them genuine refusals: each names the printed sentence, what the DSL
cannot express *in the DSL's own vocabulary*, and the nearest variant that
does exist. `tools/cardbatch/README.md` §"The refusal file" is why those two
columns are shaped that way, and the shape of the answers is worth knowing
before reading them — `Effect` and `Filter` account for most of it, which is
the usual case rather than a surprise: what is missing is normally a variant
that cannot be *said*, not a subsystem that is absent.

It is a **snapshot with a date on it**, and the date is the point: the batch
read a DSL that has moved since. `EnterModifier::TappedUnlessCount` landed at
7b989538 on 10.09, five days after the batch, and **eight** rows here still
say "`EnterModifier` has no variant for count-based entry conditions" — which
was true when it was written and names a variant that exists today. Three of
the eight are the ones it reaches ("two or more basic lands", "two or more
other lands"); the other five are a *different* gap the row does not
distinguish, because `at_least` has no counterpart — three fastlands print
"two or **fewer** other lands" and two invert the sentence entirely ("*if* you
control two or more, this land enters tapped"). So a row is a lead and never a
worklist item: check that the variant it names is still absent, and read the
printed sentence rather than the row's summary of it. The list below is where
a blocker goes once it has been re-read.

The other half of the same question is measured continuously and needs no such
care. `cargo run -p xtask -- transcode-report` ranks what the *whole* script
corpus is refused for — 4619 of 33 826 read in full as of 18.09, with
`AlternateMode:` (887), `Charm` (619) and an unreadable value in `Pump` at the
top — computed against the DSL as it stands rather than as it stood. The
line under it says the same of the reference's **token** scripts (627 of 852,
and 41 of the 184 that print a rules line), because a token's abilities are
read by this same transcoder.

**Add `--stubs` when the goal is a card rather than the DSL.** That ranks the
same question over this pool's own unfinished cards — 640 of the 648 stubs
have a reference script — and it is a different list, not a shorter one:
`Charm` is 619 corpus-wide and 2 here, and the `DamageDone` trigger 443 and 1.
What actually holds this pool's stubs shut is `AlternateMode:` (89), a
`ChangeZone` with neither a target nor a `Defined$` (23) and `Effect` (22).

Three entries have left that list rather than shrunk, and each of them was a
sentence the DSL could nearly say. An unreadable `Mana` value was 29 until the
reader learned a counted amount and a combination — fourteen lands in one
commit, because what the entry stood for was four different sentences.
`Sacrifice` was 24 until the API was read at all: bare it is `SacrificeSelf`,
and with an `UnlessCost$` it is the Karoo sentence, which needed
`Effect::PlayerMayPayCostOr` to have a price a player pays by naming an
object. `cost 'Sac'` (11) and `cost 'tapXType'` (6) fell the same day and for
a smaller reason — `cost_expr` had learnt `Return<1/…>` and none of its three
siblings, though `cost_wizard` had answered all four since activation costs
were written. Twenty-seven lands came out of the three together. `Surveil` was
25 before all of that (22 cards) and `ETBReplacement` 31 before that (15). A
stub is by construction a card no reader could write, so corpus-wide progress
reaches none of them until one of *their* blockers falls.

Read one of those for **order** — whichever matches what the work is for —
and these 115 rows for what a specific sentence cannot say.

The 48 land implementations that same batch produced are **not** in the tree.
They were written against the flat `cards/` layout and the pre-taxonomy
macros, so every one of them collides with the file standing there today.
They are at the tag `archive/gemini-batch`, to be read as a reference when
those cards are taken up — and taking them up is worth doing *after* a DSL
change rather than before, or they are written twice.

Landed since the freeze (no longer blockers): MDFC face casting, miracle,
delve, convoke, flashback grants, protection (damage/target/block),
until-EOT layer-1 copies, extra turns, lifelink counters, search locks,
no-max-hand-size, damage prevention, choose-a-type, ward, monarch,
spell-copy target re-choice, sideboard / outside-the-game access.

**And thirteen more, which is why this list is read against the pool and not
believed.** It carried fourteen entries until 22.09.2026, and each was
checked that day the one way a hand-kept list can be: by opening the card it
names and reading its `coverage` and its `// NOT SUPPORTED:` lines. Thirteen
of the fourteen named a card that is `Coverage::Implemented` today — Mox Opal
carries the metalcraft condition the entry said could not be written, Urza's
Saga its chapters, Mirrorhall Mimic its disturb, Chromatic Lantern its land
grant, Venser his emblem's trigger, Opposition Agent the real search
takeover rather than the lock the entry described, and Path of Ancestry,
Padeem, Reflections of Littjara, Wizard Class, City of Brass, Everybody
Lives! and the daybound villagers the rest of it. Only battles survived, on
the strength of Invasion of Ikoria still being a stub.

That is not a bookkeeping slip, it is the expensive kind of stale. This file
is handed verbatim to every card lane as its contract, so a sentence here
saying a mechanic cannot be written is a sentence that makes a model write
`Coverage::Partial` — or refuse the card — for a rule the engine has had for
weeks. A list of what the DSL cannot say has to be re-read against the DSL,
and an entry is cheap to check: it names a card, and the card says.

What is genuinely absent today:

- Battles (Invasion of Ikoria is a stub; `TypeSet::BATTLE` exists and
  nothing plays one).
- Dungeons and the venture mechanic.
- The initiative and Undercity (`TakeInitiative` is a refused effect in
  `transcode-report`).
- Stickers and attractions.
- Subgames, and ante.

And one thing this list used to call absent that is not absent but is a
footgun: **multiplayer**. `PlayerRel::Opponent` takes the *first* opponent —
the whole answer at a duel, and an unannounced choice at three seats or more,
which `resolve`'s own comment says where it takes `.first()`: "a duel
assumption and is wrong at a bigger table; the fix is a `Pending` and not an
index." A card whose printed sentence is "an opponent" of your choosing
writes `PlayerRel::Chosen`, which asks through `Pending::ChoosePlayer`;
`Opponent` is for a sentence where no choice is announced.

When you hit one of these: implement everything expressible, then
`Coverage::Partial("…")` + `// NOT SUPPORTED:` on the specific line. When
you hit a sentence this file says is impossible and the card it names looks
finished, **believe the card** — and say so, so this list can lose another
entry.
