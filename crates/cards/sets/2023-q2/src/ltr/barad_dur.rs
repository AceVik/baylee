//! Barad-dûr — (no cost) — Legendary Land
//! Oracle: Barad-dûr enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {B}.
//! Oracle: {X}{X}{B}, {T}: Amass Orcs X. Activate only if a creature died this turn.
//! Set: LTR #253 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: eb5038af-06b0-401e-8dea-a1a8483788ae | Oracle ID: 88159872-d37d-4847-b048-e4a9af6437bd
use baylee_cards_dsl::prelude::*;

static LEGENDARY_CREATURE_YOU_CONTROL: Filter = f!(your LEGENDARY_CREATURE);

card!(
    index = index::BARAD_DUR,
    oracle_id = "88159872-d37d-4847-b048-e4a9af6437bd",
    scryfall_id = "eb5038af-06b0-401e-8dea-a1a8483788ae",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Barad-dûr",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::TappedUnless(&LEGENDARY_CREATURE_YOU_CONTROL)],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            cost!("{X}{X}{B}", TapSelf),
            &[Effect::Amass {
                token: &crate::tokens::ARMY_0_0_BLACK,
                subtype: baylee_core::generated::subtypes::creature::ORC,
                amount: Amount::X,
            }],
            condition = Some(Condition::CreaturesDiedThisTurnAtLeast(1))
        ),
    ],
);
