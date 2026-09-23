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
- `FaceDef.mandatory_additional_costs: &[CostPart]` — e.g. `PayLifeX`. Pays
  `PayLifeX` and `PayLife`, and is the one cost list nothing gates at all.
  `PayLifeX` is bounded where it is asked instead — the wizard offers X up to
  the caster's life total (CR 119.4) and never up to a constant. A `PayLife(n)`
  written here is bounded by nothing and would be paid past zero.
- `cost!("{1}{G}", TapSelf, SacrificeSelf)` — a cost, read left to right the
  way the card prints it: the mana string first (omitted when there is none),
  then the parts. A part is named without its `CostPart::` prefix, which on a
  fetchland was the same word three times.

  The parts: `TapSelf`, `UntapSelf`, `SacrificeSelf`, `Sacrifice(filter)`,
  `Discard(filter)`, `DiscardSelf` (cycling), `PayLife(n)`, `PayLifeX`,
  `ExileSelf`, `ExileFromHand(filter)`, `ReturnSelfToHand`,
  `TapOther(filter)` (the convoke lands, Earthcraft),
  `ReturnToHand(filter)` (Quirion Ranger's Forest — one permanent, and
  nothing in it says "untapped": tapping the land for mana and *then*
  returning it is the play, and the mana stays in the pool),
  `ExileFromGraveyard(filter)` (Moorland Haunt's creature card — one card out
  of **your own** graveyard, the zone saying whose, so the filter says only
  what kind of card; Mines of Moria's "three cards" is the part written three
  times, one question each, the way Time Sieve writes five sacrifices),
  `RemoveCounterSelf { kind, n }` (the Vivid lands, Tendo Ice Bridge),
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
  the way it carries `index`.

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
- `AbilityDef::Activated { cost, effects, target, timing, mana_ability, zone }`
- `AbilityDef::Triggered { trigger, effects, targets, once_per_turn }`
- `AbilityDef::Static(StaticAbility { layer, filter, modifier })` — written
  `static_ability!(filter, modifier)`, which takes no layer: CR 613.1 makes it
  a function of the modifier and `Modifier::layer` is that function
- `AbilityDef::Replacement(ReplacementRule)` — trigger multipliers/suppressors,
  token/counter doubling
- `AbilityDef::Loyalty { cost: i8, effects, target }`
- `AbilityDef::CopyOnEnter { target, mods: &[CopyMod] }` — the `mods` are the
  card's "except …" clauses (CR 707.9). Types, supertypes, subtypes, keywords
  and entry counters are all sayable, and so is "except it has its **other**
  abilities" (`CopyMod::KeepOtherAbilities`, CR 707.9a) — with one limit worth
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
  prowess), supports ward {1}/{2}. **Undying** and **persist** are the same
  shape one step further along: they are not an `AbilityDef` at all but two
  bits on `keywords`, and `trigger.rs` reads them the way it reads prowess.
  A card prints one of them by setting the bit and writing no ability —
  `keywords = KeywordSet::UNDYING` is the whole of Young Wolf
- `AbilityDef::Suspend { counters }`

#### Write them through the macros

The six shapes that make up most of the pool have a macro that supplies the
fields the rules already imply, so an ability states what the card says and
nothing more:

```rust
mana_ability!(&[Effect::mana(ManaColor::Green, 1)])   // {T}: Add {G}
mana_ability!(SAC_COST, ANY_COLOR_MANA)               // any other cost
activated!(Cost::TAP, EFFECTS)                        // {T}: …
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
modal_triggered!(TRIGGER, &[mode!(SCRY), mode!(LIFE)])  // "choose one" ETB
mode!(DRAW_EFFECTS)                                   // one arm of a modal
```

`TargetReq::one` and `TargetReq::up_to_one` take a `TargetSpec`, never a
`&Filter` — a target is an object, a player, a spell or a card in a
graveyard, and the filter is only how an *object* target is picked.

The required arguments come first and positionally, because they are the
ones an ability cannot be written without; everything after them is
`field = value` in any order, and anything left out takes its rules default:

| field | default | why that is the rules answer |
| --- | --- | --- |
| `timing` | `InstantSpeed` | CR 117.1b — unless the card restricts it |
| `mana_ability` | `false` | CR 605.1 makes it the exception |
| `zone` | `Battlefield` | CR 113.6 |
| `target` / `targets` | `None` | an ability targets only when it says "target" |
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

`activated!` and `mana_ability!` reach `AbilityDef::ActivatedConditional`
through one optional field, `condition = Some(Condition::…)`. That
is the only difference between the twins, which is exactly what makes them
easy to confuse: six readers across the engine, the client and the pool lints
once matched `AbilityDef::Activated` alone and skipped every conditional
ability there was.

`Condition` is the shared vocabulary for "only while this is true" and is
not activation-specific — it was called `ActivationCondition` after its one
reader. `ControlCount(&filter, n)` is metalcraft and the verge lands,
`OpponentGraveyardCountAtLeast(n)` is Sheoldred's flip,
`CountersOnSelf(kind, n)` and `CountersOnSelfExactly(kind, n)` read the
permanent the ability is printed on, `SourceMatches(&filter)` points a
filter back at that permanent — "if this land is tapped" — and
`Any(&[..])` holds while **one** of the conditions it names does. One
reader answers all of them, `eval::condition_holds`.

There is no `All`, and that is not an omission. The printed sentence that
needs a disjunction is real and prints as one — "activate only if this land
entered this turn or you control a basic land" is Gathering Place, Gleaming
Bastion and Hidden Lair — while a conjunction has never appeared on a card
in this pool: a second clause is written as a second sentence, and a second
sentence is a second `condition` on a second ability. Add `All` the day a
card prints one, not before, so that every variant here stands for a
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
`ChooseColor` and `ChooseColorExcept(c)` (Uncharted Haven, the Thriving
cycle, the Gates — answer stored on `obj.chosen_color` and read back by
`ManaSource::Chosen`), `Prepared` (Emeritus of Woe),
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
`EntersBattlefieldEvoked`, `StepBegin { step, whose }`.

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
in front of it; this one reads the journal from `state.turn_start_seq` for
a `ZoneChanged` into the battlefield naming that object. Two consequences.
It costs a scan rather than a field compare, so it belongs in the narrow
half of an `And` and not the wide one. And a `PlayerView` carries no
journal, so `baylee_ai::filter` answers `None` for it and the client's
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
`YOUR_ARTIFACT`, `ANOTHER_CREATURE_YOU_CONTROL`. "A creature" had been
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
`blink(t)`, `bounce(t)`, and `continuous(filter, modifier, duration)`
with the layer derived. `Effect::mana` is the precedent — 219 uses in the
pool against zero raw `AddMana` literals.

Two rules keep that from growing into a phrasebook. **One verb per variant,
and only where the variant has one answer to give**: `SearchLibrary { filter,
finds, optional }` has two real choices in it, so it stays a literal rather
than becoming a `search` / `may_search` / `search_to_hand` family. And **the
name is the word this pool already says**, which is usually the printed one;
`blink` and `bounce` are the two that are not. Neither is a coinage: the
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

A card that says nothing about a graveyard cannot use either: the effect
checks that its object is still in one (CR 400.7). A reanimation *spell* is
already held to that by target legality (CR 608.2b) — the guard is there for
undying and persist, which target nothing at all.

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
`spell!` or `activated!`, beside `targets`/`target` — never on a mana
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
Removal: `Destroy`, `DestroyAll`, `Regenerate`, `Exile`, `CounterTargetSpell`,
`CounterTargetAbility`, `CounterTargetSpellOrAbility`,
`TargetSourceLosesAbilities` (Tishana's Tidebinder: it reaches the permanent
whose ability an *earlier* `CounterTargetAbility` in the same effect list
countered, so it has to follow one; `source_filter` is the printed
restriction on which permanents it reaches), `SacrificeFilter`, `ReturnToHand`,
`ReturnAllToHand`, `ReturnChosenToHand`, `RedirectTarget` (Misdirection).

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
Zones: `SearchLibrary`, `OptionalBasicLandSearchFor`, `GraveyardToTop`,
`GraveyardToHand`, `GraveyardToBattlefield`, `ExileGraveyard`, `Blink`,
`ExileLinked`, `ReturnLinkedToBattlefield`, `PutFromHandOnTop`,
`PutSourceOnTopOfLibrary`, `ExileAndReturnAtEndStep` (Venser +2, Eerie
Interlude), `BottomCardFromHand`, `WishToHand` (Karn's −2: a card you own
from outside the game or face-up in your exile).
Continuous: `CreateContinuousEffect` (any layer+filter+modifier+duration),
`PumpFilter` (a filter, where `Filter::This` is the *source*, plus
`controlled_by: Option<PlayerRel>` for a sentence that names a player rather
than the board — "creatures **target player** controls get -2/-2", where the
seat is a choice no `Filter` can be told about), `PumpTarget`
(the spell's or ability's targets, all of them — Giant Growth), both of which
carry a `KeywordSet` so "+2/+2 and gains trample" is one effect,
`SetPTFilter`, `ChangeController`, `AllCreaturesToOwner`,
`ExchangeControlOrSacrifice` (Gilded Drake), `PhaseOut`, `AttachSelf`.
Tokens/copy: `CreateToken`, `CreateTokenN`, `CreateTokenForTargetController`,
`CreateTokenFromLinked`, `CreateTokenCopyOf`, `CreateTokenCopyOfEquipped`,
`CreateTokenCopyOfFirstToken`, `CopyTargetSpell`, `Amass`.
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
Utility: `UntapTarget`, `UntapSelf` (the source, with no target and no
question — CR 115.1c makes an activated ability targeted only when it says
the word, so "untap this creature" is the second variant and not the first
pointed at itself), `NegXFixed` (amount), `CreateTokenCopyOfFirstToken`,
`BecomeMonarch`, `Sequence(&[..])`.
Modal/sequence: `Sequence(&[..])`.

### Modifiers (layer effects)

`AddType`, `RemoveType`, `AddSubtype`, `AllCreatureTypes`,
`AllBasicLandTypes`, `AddColor`, `SetColor`, `AddKeyword`, `RemoveKeyword`,
`LoseKeywords`, `ModifyPT`, `SetPT`, `SwitchPT`, `LegendRuleOff`,
`CantActivateArtifacts`, `OpponentsCastAsSorcery`, `PlayersCantLose`,
`CantLoseLife`, `PreventDamageToIt`, `PreventDamageFromIt`,
`OpponentsCantSearch`, `NoMaxHandSize`, `GainControl`, `DoesNotUntap`,
`MayChooseNotToUntap`, `PlayLandsFromGraveyard`, `ExtraLandDrops`,
`DrawLimitPerTurn`.

`DrawLimitPerTurn { who, limit }` is "each player can't draw more than one
card each turn" (Spirit of the Labyrinth) and its opponents-only twin
(Leovold). `who` is an ordinary `PlayerRel` read from the **effect's**
controller, so `EachPlayer` includes whoever played the card and
`EachOpponent` does not — a card written from the wrong sentence stops its
own draws or fails to stop them.

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
step" — that is a created effect with a duration, and does not exist yet.

`MayChooseNotToUntap` is the other half of the same rule and the storage
lands' clause: CR 502.3 has the active player *determine* which of their
permanents untap, and this variant is what gives that determination a second
answer. The engine then suspends inside the untap step with a
`Pending::ChooseCards` whose answer names what stays tapped — no priority is
granted, which CR 502.4 forbids and this is not. It takes `Filter::This` and
no duration like its neighbour, and the two compose: a permanent an effect
already keeps from untapping is left off the menu, because both answers to
that question would do the same thing.

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
Effect::mana_land_color(true)                          // …a land you control could produce.
Effect::mana_dynamic(ManaColor::Black, Amount::CountOf { .. })
Effect::mana_of_any_color().restricted(&FILTER, SpendRider::Uncounterable)
```

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
