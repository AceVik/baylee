//! Carrion Feeder — {B} — Creature — Zombie
//! Oracle: This creature can't block.
//! Oracle: Sacrifice a creature: Put a +1/+1 counter on this creature.
//! Set: MH1 #81 — Modern Horizons | Scryfall ID: 0a19da90-880e-4eca-8cf7-6d7baf090d53 | Oracle ID: a1cc5e37-b09a-4b7f-afd5-77c1c35aa425
// IMPLEMENTED — "can't block" as the keyword bit `combat::can_block`
// reads, and the sacrifice for a +1/+1 counter.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CARRION_FEEDER,
    oracle_id = "a1cc5e37-b09a-4b7f-afd5-77c1c35aa425",
    scryfall_id = "0a19da90-880e-4eca-8cf7-6d7baf090d53",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Carrion Feeder",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(1),
        toughness = Some(1),
        // A face keyword and not a `static_ability!`: it is printed on this
        // creature about itself, which is what `FaceDef::keywords` is, and
        // the layer system hands it to `combat::can_block` the same way it
        // hands over defender.
        keywords = KeywordSet::CANT_BLOCK,
    ),],
    abilities = &[activated!(
        cost!(Sacrifice(&Filter::YOUR_CREATURE)),
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }]
    ),],
);
