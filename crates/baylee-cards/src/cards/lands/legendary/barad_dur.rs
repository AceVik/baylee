//! Barad-dûr — (no cost) — Legendary Land
//! Oracle: Barad-dûr enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {B}.
//! Oracle: {X}{X}{B}, {T}: Amass Orcs X. Activate only if a creature died this turn.
//! Set: LTR #253 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: eb5038af-06b0-401e-8dea-a1a8483788ae | Oracle ID: 88159872-d37d-4847-b048-e4a9af6437bd
// PARTIAL — the enters-tapped-unless modifier and the {B} mana ability are
// built; the amass ability is not (see NOT SUPPORTED at the foot).

use baylee_cards_dsl::prelude::*;

static LEGENDARY_CREATURE_YOU_CONTROL: Filter = f!(your LEGENDARY_CREATURE);

card!(
    index = index::BARAD_DUR,
    oracle_id = "88159872-d37d-4847-b048-e4a9af6437bd",
    scryfall_id = "eb5038af-06b0-401e-8dea-a1a8483788ae",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "the {X}{X}{B}, {T}: Amass Orcs X ability is not built: no Condition says a creature died this turn, and Effect::Amass carries a fixed u16 where the card prints X"
    ),
    faces = &[face!(
        name = "Barad-dûr",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::TappedUnless(&LEGENDARY_CREATURE_YOU_CONTROL)],
    ),],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);

// NOT SUPPORTED: "{X}{X}{B}, {T}: Amass Orcs X. Activate only if a creature
// died this turn." — the printed activation restriction has no `Condition`
// variant (it is not a control count, a graveyard count, a counter count or a
// filter pointed back at the source, and `Effect::IfCreaturesDiedAtLeast` is
// a branch inside a resolving effect rather than a gate on activating one),
// and `Effect::Amass`'s `amount` is a fixed `u16` with no `Amount` in it, so
// the announced X has nowhere to go.
