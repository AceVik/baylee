//! Blood of the Martyr — {W}{W}{W} — Instant
//! Oracle: Until end of turn, if damage would be dealt to any creature, you may have that damage dealt to you instead.
//! Set: CHR #4 — Chronicles | Scryfall ID: 823b637b-e089-479a-88c0-be014516429a | Oracle ID: 0e69e6c2-fdc5-4c1a-bd96-c4283b82743a

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLOOD_OF_THE_MARTYR,
    oracle_id = "0e69e6c2-fdc5-4c1a-bd96-c4283b82743a",
    scryfall_id = "823b637b-e089-479a-88c0-be014516429a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blood of the Martyr",
        mana_cost = mana!("{W}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::CreateContinuousEffect {
        layer: Layer::Text,
        filter: &Filter::CREATURE,
        modifier: Modifier::MayRedirectDamageToYou,
        duration: Duration::UntilEndOfTurn,
    }])],
);
