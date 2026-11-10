//! Haunting Apparition — {1}{U}{B} — Creature — Spirit
//! Oracle: Flying
//! Oracle: As this creature enters, choose an opponent.
//! Oracle: Haunting Apparition's power is equal to 1 plus the number of green creature cards in the chosen player's graveyard.
//! Set: MIR #267 — Mirage | Scryfall ID: 8ce9c58f-6470-4e0f-8f6b-457fbaac7451 | Oracle ID: ced9130c-2865-49df-abdd-84069aa1b180
// NOT SUPPORTED: characteristic-defining power equal to 1 plus the number of green creature cards in the chosen player's graveyard; chooses an opponent as it enters and plays as a 1/2 creature with flying.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HAUNTING_APPARITION,
    oracle_id = "ced9130c-2865-49df-abdd-84069aa1b180",
    scryfall_id = "8ce9c58f-6470-4e0f-8f6b-457fbaac7451",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Partial(
        "has flying, chooses an opponent as it enters, and has base power and toughness 1/2; setting its power equal to 1 plus the number of green creature cards in the chosen player's graveyard is not in PtCount",
    ),
    faces = &[face!(
        name = "Haunting Apparition",
        mana_cost = mana!("{1}{U}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(1),
        toughness = Some(2),
        keywords = KeywordSet::FLYING,
        enter_modifiers = &[EnterModifier::ChooseOpponent],
    ),],
);
