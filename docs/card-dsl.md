# Card DSL Cookbook (frozen at M2.S8)

**This is the authoring contract for card implementations — humans and LLMs
alike.** If a mechanic is not expressible with the vocabulary here, the card
gets `Coverage::Partial("exact reason")` and a `// NOT SUPPORTED:` comment.
Never hack around the DSL; extend the DSL instead (in a new milestone).

## File standard (one file per card)

Location: under `crates/baylee-cards/src/cards/`, in the branch the card's own
type line puts it in — `instants/mv_1/lightning_bolt.rs`. The next section is
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

One import, and it is the only one most cards need — the prelude carries the
whole vocabulary plus the macros below. Add a second `use` line only for
something outside it: a subtype module
(`use baylee_core::generated::subtypes::creature;`), a token
(`use crate::tokens::…`), or a shared filter (`use crate::filters::…`).

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
  Kindred trails so Crib Swap is an instant. CR 205.1a lists the types and
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
card! {
    index: 104,
    oracle_id: "f4232466-dd6a-49bf-be6c-95905c3ded17",
    scryfall_id: "ced43447-fefc-482a-b8fa-33b9616aa532",
    faces: &[face! {
        name: "Ondu Cleric",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::CLERIC, creature::ALLY],
        power: Some(1),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[/* … */],
}
```

`card!` and `face!` *are* those literals — they expand to `CardDef { … ,
..CardDef::DEFAULT }` and `FaceDef { … , ..FaceDef::DEFAULT }`. Two things
come with them: the tail can no longer be forgotten, and `card!` writes the
doc comment on the `pub static CARD` it defines. The three identity fields
are mandatory and come first, in the order codegen writes them.

Never write a field back just to restate its default (`loyalty: None`,
`delve: false`, `partner: PartnerKind::None`, …) — a reviewer should be able
to read the literal as the card's printed face. Adding a field to `FaceDef`
then costs one line in `baylee-cards-dsl` instead of one line in ~200 card
files.

Two defaults are the *pessimistic* value rather than the common one, and
both are load-bearing:

- `CardDef::DEFAULT.index` is `0`, which collides with card 0. The
  `every_card_sits_at_the_index_it_claims` test in `baylee-cards` turns a
  forgotten index into a build failure instead of a card that silently
  resolves as another one.

  Where the number comes from: `data/card-index.tsv`, the append-only ledger
  codegen keeps. A `CardIndex` is an identity, not a position — `DeckEntry`
  stores one, the gateway persists decks made of them, and a replay names
  them — so adding a card takes the next free index and never renumbers a
  card that is already there. A card that leaves the pool retires its index;
  the slot stays empty (`BY_INDEX` holds `None` there) rather than being
  handed to someone else. Never edit the ledger by hand: `codegen --check`
  fails if a run would change it.
- `CardDef::DEFAULT.coverage` is `Coverage::Unimplemented`, so a stub that
  was never finished cannot reach the deckbuilder as playable just because
  a line went missing. An implemented card writes
  `coverage: Coverage::Implemented` by hand.

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
`castable_from_hand: false` for any back face that prints no mana cost and
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
  it prints. So an ordinary card still writes one `keywords:` line, and a
  transforming card writes one per face — including the keyword both faces
  share, which is written twice on purpose. Daybound is printed on a front
  face and nightbound on a back one (CR 702.145a), and a card that stated
  either for the whole card would be a permanent that turns over at night
  and turns back in the same breath.
- `color_indicator` (CR 105.2c). A face with no mana cost has nothing else
  to say what colour it is; Dire-Strain Brawler is green only because of the
  dot printed on it.
- `castable_from_hand`, above.

## Generated cards, and why they may say `Implemented`

Most card files are hand-written. Two of them are not, and the distinction
matters when you open one:

- **Lands** are read from their printed text by
  `crates/baylee-cards-codegen/src/landgen.rs`.
- **Everything else with a local forge-reference script** is read by
  `crates/baylee-cards-codegen/src/forgegen.rs` (the checkout is an
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

A generated file is hand-owned the moment it exists: `codegen` only writes
files that are missing or still carry `// GENERATED STUB`, so editing one is
exactly like editing any other card file.

If you want more cards generated, the lever is usually **this document's
vocabulary**, not the readers. `cargo run -p xtask -- forge-report` ranks
what the corpus is waiting on, and the top entries are effects the DSL cannot
express at all yet.

## The vocabulary

### Card faces & costs

- `mana_cost: baylee_core::mana!("{2}{W/U}{W/P}")` — compile-time parsed
  (generic, color, hybrid, 2-or, Phyrexian, hybrid-Phyrexian, snow, X/Y/Z).
- `FaceDef.alternative_costs: &[AlternativeCost { cost, condition }]` —
  pitch/evoke/conditional-free (conditions: `Always`, `NotYourTurn`,
  `CommanderControlled`).
- `FaceDef.additional_costs: &[Cost]` — kicker (optional, yes/no at cast).
- `FaceDef.mandatory_additional_costs: &[CostPart]` — e.g. `PayLifeX`.
- `Cost { mana, parts }` — parts: `TapSelf`, `UntapSelf`, `SacrificeSelf`,
  `Sacrifice(filter)`, `Discard(filter)`, `DiscardSelf` (cycling),
  `PayLife(n)`, `PayLifeX`, `ExileSelf`, `ExileFromHand(filter)`.

### Ability kinds

- `AbilityDef::Spell { effects, targets: Option<TargetReq> }`
- `AbilityDef::Activated { cost, effects, target, timing, mana_ability, zone }`
- `AbilityDef::Triggered { trigger, effects, targets, once_per_turn }`
- `AbilityDef::Static(StaticAbility { layer, filter, modifier, cross_zone })`
- `AbilityDef::Replacement(ReplacementRule)` — trigger multipliers/suppressors,
  token/counter doubling
- `AbilityDef::Loyalty { cost: i8, effects, target }`
- `AbilityDef::CopyOnEnter { target, mods: &[CopyMod] }`
- `AbilityDef::ModalSpell { modes: &[SpellMode] }` — overload & friends
- `AbilityDef::ModalTriggered { trigger, modes, once_per_turn }` — "choose
  one/up to one" ETB triggers (decline = an empty mode)
- `AbilityDef::Ward { mana }` — engine-level synthetic trigger (like
  prowess), supports ward {1}/{2}
- `AbilityDef::Suspend { counters }`

#### Write them through the macros

The five shapes that make up most of the pool have a macro that supplies the
fields the rules already imply, so an ability states what the card says and
nothing more:

```rust
mana_ability!(&[Effect::mana(ManaColor::Green, 1)])   // {T}: Add {G}
mana_ability!(SAC_COST, ANY_COLOR_MANA)               // any other cost
activated!(Cost::TAP, EFFECTS)                        // {T}: …
activated!(EQUIP, EFFECTS, timing: ActivationTiming::SorcerySpeed)
triggered!(Trigger::EntersBattlefield(&Filter::This), EFFECTS)
spell!(EFFECTS)
spell!(EFFECTS, targets: Some(TargetReq::one(&Filter::CREATURE)))
loyalty!(-3, EFFECTS, target: Some(TargetSpec::Object(&Filter::CREATURE)))
mode!(DRAW_EFFECTS)                                   // one arm of a modal
```

The required arguments come first and positionally, because they are the
ones an ability cannot be written without; everything after them is
`field: value` in any order, and anything left out takes its rules default:

| field | default | why that is the rules answer |
| --- | --- | --- |
| `timing` | `InstantSpeed` | CR 602.2 — unless the card restricts it |
| `mana_ability` | `false` | CR 605.1 makes it the exception |
| `zone` | `Battlefield` | CR 113.6 |
| `target` / `targets` | `None` | an ability targets only when it says "target" |
| `once_per_turn` | `false` | a trigger fires on every occurrence |

`mana_ability: false` is the load-bearing one: an ability wrongly marked
`true` would silently skip the stack, and no test would read that as a rules
bug. That is why it is a default you have to opt *out* of, and why a mana
ability gets its own macro rather than a flag.

A shape without a macro (`Static`, `Replacement`, `CopyOnEnter`, `Ward`,
`Suspend`, `ModalSpell`, `ModalTriggered`, `SagaChapter`, `Echo`,
`Prepared`) is written as the plain enum literal — those have no fields the
rules can supply for you.

### As-it-enters modifiers (`FaceDef::enter_modifiers`)

`Tapped`, `TappedUnless(filter)`, `TappedOrPayLife(n)`, `ChooseSubtype`
(Roaming Throne, Reflections of Littjara, Cavern of Souls — answer stored
on `obj.chosen_subtype`; creatures also gain the subtype in their base).

### Triggers

`EntersBattlefield(filter)`, `LeavesBattlefield(filter)`, `Dies(filter)`,
`SpellCast(filter)`, `Draws(rel)`, `DrawsExceptFirst(rel)`,
`FirstNoncreatureSpellCast(rel)`, `Attacks(filter)`, `BecomesTarget`,
`EntersBattlefieldEvoked`, `StepBegin { step, whose }`.

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
(Roaming Throne & co.), `InZone(ZoneRef)` (incl. `NotBattlefield` for
cross-zone effects).

**Compose them inline.** In `static` context a slice promotes to `'static`
automatically, so `Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])`
needs no named `static` at all. Give a filter a name only when the same card
refers to it twice.

**Reach for the named ones first.** `Filter` carries constants for the
predicates the pool kept reinventing — `CREATURE`, `ARTIFACT`,
`ENCHANTMENT`, `LAND`, `PLANESWALKER`, `NONLAND`, `NONCREATURE`,
`BASIC_LAND`, `NONTOKEN_CREATURE`, `INSTANT_OR_SORCERY`,
`ARTIFACT_OR_ENCHANTMENT`, `ANOTHER_CREATURE`, `YOUR_CREATURE`,
`OPPONENT_CREATURE`. "A creature" had been written out as
`HasType(TypeSet::CREATURE)` in a differently-named `static` in twenty-six
card files, which is twenty-six chances to type `LacksType` by accident and
no way to grep for the one that did.

A filter that is about *this pool* rather than about Magic goes in
`crates/baylee-cards/src/filters.rs` (`YOUR_ALLIES`, `ANOTHER_ALLY`), beside
`crate::tokens`, which draws the same line. It earns a place there by being
written twice; one card's own compound filter stays in that card's file,
where the oracle sentence it encodes is a line above it.

### Effects (ops)

Life/draw: `GainLife`, `GainLifeFor`, `GainLifeDoubleX`, `LoseLife`,
`DrawCards`, `DrawCardsFor`, `Scry`, `ScryFor`, `Mill`,
`RearrangeTopLibrary`/`ReorderTopLibrary`.
Combat/damage: `DealDamage`, `DealDamageToTargetController`.
Removal: `Destroy`, `DestroyAll`, `Exile`, `CounterTargetSpell`,
`CounterTargetAbility`, `CounterTargetSpellOrAbility`,
`TargetSourceLosesAbilities`, `SacrificeFilter`, `ReturnToHand`,
`ReturnAllToHand`, `RedirectTarget` (Misdirection).
Zones: `SearchLibrary`, `OptionalBasicLandSearchFor`, `GraveyardToTop`,
`GraveyardToHand`, `GraveyardToBattlefield`, `ExileGraveyard`, `Blink`,
`ExileLinked`, `ReturnLinkedToBattlefield`, `PutFromHandOnTop`,
`PutSourceOnTopOfLibrary`, `ExileAndReturnAtEndStep` (Venser +2, Eerie
Interlude), `BottomCardFromHand`, `WishToHand` (Karn's −2: a card you own
from outside the game or face-up in your exile).
Continuous: `CreateContinuousEffect` (any layer+filter+modifier+duration),
`PumpFilter` (a filter, where `Filter::This` is the *source*), `PumpTarget`
(the spell's or ability's targets, all of them — Giant Growth), both of which
carry a `KeywordSet` so "+2/+2 and gains trample" is one effect,
`SetPTFilter`, `ChangeController`, `AllCreaturesToOwner`,
`ExchangeControlOrSacrifice` (Gilded Drake), `PhaseOut`, `AttachSelf`.
Tokens/copy: `CreateToken`, `CreateTokenN`, `CreateTokenForTargetController`,
`CreateTokenFromLinked`, `CreateTokenCopyOf`, `CreateTokenCopyOfEquipped`,
`CreateTokenCopyOfFirstToken`, `CopyTargetSpell`, `Amass`.
Costs/taxes: `PlayerMayPayOr`, `AddCounter`, `AddCounterFilter`,
`DrainAllCountersIntoSelf` (Thief of Blood), `AddMana`,
`DelayedManaAtNextFirstMain` (Mana Drain), `SacrificeSelf`,
`PayCostOrLoseLater`, `ExileTargetsCreateTokens`.
Conditional: `IfEventPowerAtLeast` (Tribute to the World Tree).
Utility: `UntapTarget`, `NegXFixed` (amount), `CreateTokenCopyOfFirstToken`,
`BecomeMonarch`, `Sequence(&[..])`.
Modal/sequence: `Sequence(&[..])`.

### Modifiers (layer effects)

`AddType`, `RemoveType`, `AddSubtype`, `AllCreatureTypes`,
`AllBasicLandTypes`, `AddColor`, `SetColor`, `AddKeyword`, `RemoveKeyword`,
`LoseKeywords`, `ModifyPT`, `SetPT`, `SwitchPT`, `LegendRuleOff`,
`CantActivateArtifacts`, `OpponentsCastAsSorcery`, `PlayersCantLose`,
`CantLoseLife`, `PreventDamageToIt`, `PreventDamageFromIt`,
`OpponentsCantSearch`, `NoMaxHandSize`, `GainControl`.

`GainControl` is layer 2 and must be paired with `Layer::Control` — any
other layer applies it out of order with respect to the effects that read
the controller. With `Filter::This` and `Duration::UntilEndOfTurn` it is
Act of Treason; with `WhileSourceOnBattlefield` it is Mind Control. Use it
rather than `Effect::ChangeController` whenever the control comes back:
the one-shot effect never returns the permanent.

New `Modifier` variants must be added to THREE places: the
"handled elsewhere" arm in `layers.rs`, the modifier hash in
`state.rs`, and whatever system enforces them (SBAs, combat, casting).

## Worked examples

A land with two basic land types must print its own mana ability. CR 305.6
grants one ability *per* basic type, and the engine's intrinsic shortcut
(`casting::intrinsic_mana`) can only return a single colour with no way to
ask which — so it declines any land with more than one, and such a land taps
for nothing at all unless the card supplies the choice itself:

```rust
abilities: &[mana_ability!(&[Effect::mana_choice(&[
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
amount. Harabaz Druid was written with the wrong one and paid X² mana.

Fetchland (activated with composite cost + filtered search):

```rust
abilities: &[activated!(
    Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf, CostPart::SacrificeSelf, CostPart::PayLife(1)],
    },
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
  searches in the forge reference, three do not say "then shuffle", and all
  three empty the library instead.
- **A find is revealed** when the search is narrower than "a card"
  (`Filter::Any`) *and* at least one destination is hidden (hand, top of
  library). Mystical Tutor reveals; Demonic Tutor does not; a fetchland does
  not, because the battlefield is public anyway. The reveal is journalled as
  `GameEvent::Revealed` — it is what holds the searcher to the filter.

Rally trigger (filter "self or another Ally you control"):

```rust
use crate::filters::YOUR_ALLIES;

abilities: &[triggered!(
    Trigger::EntersBattlefield(&YOUR_ALLIES),
    &[Effect::GainLife { amount: Amount::Fixed(1) }]
)],
```

The filter is shared rather than restated: the rally wording is "this
creature **or another** Ally", so the source is part of the match, and six
card files had each written that out. A seventh writing `Another` instead
would have been a silent rules bug in a card that still compiled.

Static anthem via layers (deregisters itself when the source leaves):

```rust
abilities: &[AbilityDef::Static(StaticAbility {
    layer: Layer::PtModify,
    filter: Filter::YOUR_CREATURE,
    modifier: Modifier::ModifyPT(1, 1),
    cross_zone: false,
})],
```

## Explicitly not supported yet (M3+)

Landed since the freeze (no longer blockers): MDFC face casting, miracle,
delve, convoke, flashback grants, protection (damage/target/block),
until-EOT layer-1 copies, extra turns, lifelink counters, search locks,
no-max-hand-size, damage prevention, choose-a-type, ward, monarch,
spell-copy target re-choice, sideboard / outside-the-game access.

- Multiplayer player-choice for targeted triggers (protocol M3) — use
  `PlayerRel::Opponent` (heads-up auto-resolve) + `Partial` note for MP.
- Sagas (Urza's Saga chapters, The True Scriptures): lore counters,
  chapter triggers, granted abilities, sacrifice after the last chapter.
- Disturb (Mirrorhall Mimic's back): graveyard face-casting.
- Activation conditions (Mox Opal metalcraft, Bleachbone Verge) — abilities
  currently activate unconditionally (`Partial` note).
- Mana-source tracking / restricted mana riders (Cavern of Souls
  uncounterable, Path of Ancestry scry) — pool mana has no provenance.
- Search takeover (Opposition Agent's real hijack; approximated as a lock).
- Tap events (City of Brass's becomes-tapped trigger).
- Comparative conditions (Padeem's greatest-cmc upkeep).
- Ability-granting statics (Chromatic Lantern's land grant; also blocks
  Urza's Saga ch. I/II).
- Emblems with triggered abilities (Venser −8) — engine supports emblem
  objects; trigger scan for command zone is pending.
- Cost reducers (Surgical Metamorph's not-starting-player {1} less).
- Permanent-spell copies resolving as tokens (Reflections of Littjara
  rider).
- Player hexproof (Everybody Lives! rider).
- Day/night, dungeons, initiative, battles, classes (Wizard Class levels),
  stickers/attractions, subgames, ante.

When you hit one of these: implement everything expressible, then
`Coverage::Partial("…")` + `// NOT SUPPORTED:` on the specific line.
