//! Exalted Angel — {4}{W}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: Whenever this creature deals damage, you gain that much life.
//! Oracle: Morph {2}{W}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: MKC #63 — Murders at Karlov Manor Commander | Scryfall ID: 7a6b087b-a3ac-4c4d-a519-ec5584b4b42d | Oracle ID: 2f6e9ab8-6951-4c5f-bcde-0e96e0fe87f2
// IMPLEMENTED — flying, damage-based life gain, and morph.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EXALTED_ANGEL,
    oracle_id = "2f6e9ab8-6951-4c5f-bcde-0e96e0fe87f2",
    scryfall_id = "7a6b087b-a3ac-4c4d-a519-ec5584b4b42d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Exalted Angel",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(4),
        toughness = Some(5),
        morph = Some(mana!("{2}{W}{W}")),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamage(&Filter::This),
        &[Effect::GainLife {
            amount: Amount::EventAmount,
        }],
    )],
);
