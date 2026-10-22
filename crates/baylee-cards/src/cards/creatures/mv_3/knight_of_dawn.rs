//! Knight of Dawn — {1}{W}{W} — Creature — Human Knight
//! Oracle: First strike
//! Oracle: {W}{W}: This creature gains protection from the color of your choice until end of turn.
//! Set: TMP #26 — Tempest | Scryfall ID: bf0e5034-a134-4eb6-af8e-b2419b92b3a6 | Oracle ID: 51262155-f1be-45a8-a21f-94e920a93e55
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KNIGHT_OF_DAWN,
    oracle_id = "51262155-f1be-45a8-a21f-94e920a93e55",
    scryfall_id = "bf0e5034-a134-4eb6-af8e-b2419b92b3a6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Partial(
        "Has first strike and '{W}{W}: This creature gains protection from black until end of turn'; Effect::ProtectionFromChosenColor requires a target, so a non-targeted self-protection color choice is not yet modeled."
    ),
    faces = &[face!(
        name = "Knight of Dawn",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{W}{W}"),
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::PROTECTION_BLACK,
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
