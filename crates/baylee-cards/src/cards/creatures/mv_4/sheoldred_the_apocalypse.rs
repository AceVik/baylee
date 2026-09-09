//! Sheoldred, the Apocalypse — {2}{B}{B} — Legendary Creature — Phyrexian Praetor
//! Oracle: Deathtouch
//! Oracle: Whenever you draw a card, you gain 2 life.
//! Oracle: Whenever an opponent draws a card, they lose 2 life.
//! Set: DMU #107 — Dominaria United | Scryfall ID: d67be074-cdd4-41d9-ac89-0a0456c4e4b2 | Oracle ID: 34f34409-326d-4994-a0ea-1a69aa278f03
// IMPLEMENTED — deathtouch + draw-punish both directions.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 145,
    oracle_id: "34f34409-326d-4994-a0ea-1a69aa278f03",
    scryfall_id: "d67be074-cdd4-41d9-ac89-0a0456c4e4b2",
    faces: &[face! {
        name: "Sheoldred, the Apocalypse",
        mana_cost: baylee_core::mana!("{2}{B}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::PHYREXIAN, creature::PRAETOR],
        power: Some(4),
        toughness: Some(5),
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::DEATHTOUCH,
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::Draws(PlayerRel::You), &[Effect::GainLife {
                amount: Amount::Fixed(2),
            }]),
        triggered!(Trigger::Draws(PlayerRel::Opponent), &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::Opponent,
            }]),
    ],
}
