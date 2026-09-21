//! Murmuring Bosk — (no cost) — Land — Forest
//! Oracle: ({T}: Add {G}.)
//! Oracle: As this land enters, you may reveal a Treefolk card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {W} or {B}. This land deals 1 damage to you.
//! Set: DMC #220 — Dominaria United Commander | Scryfall ID: 5aca73a9-e90d-48c6-bdd9-9a3f4f552de3 | Oracle ID: 42b9d383-3fe2-4fc8-ab86-f80a288d502b
// PARTIAL — Forest tap-for-{G} and painland tap for {W} or {B}; revealing a
// Treefolk card from hand on entering is not expressible in EnterModifier.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MURMURING_BOSK,
    oracle_id = "42b9d383-3fe2-4fc8-ab86-f80a288d502b",
    scryfall_id = "5aca73a9-e90d-48c6-bdd9-9a3f4f552de3",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::White]),
    coverage = Coverage::Partial(
        "EnterModifier has no variant for revealing a card from hand as a condition to enter untapped",
    ),
    faces = &[face!(
        name = "Murmuring Bosk",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::FOREST],
    ),],
    abilities = &[
        // NOT SUPPORTED: As this land enters, you may reveal a Treefolk card from your hand. If you don't, this land enters tapped.
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        mana_ability!(&[
            Effect::mana_choice(&[ManaColor::White, ManaColor::Black]),
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ]),
    ],
);
