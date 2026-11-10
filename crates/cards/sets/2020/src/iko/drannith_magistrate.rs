//! Drannith Magistrate — {1}{W} — Creature — Human Wizard
//! Oracle: Your opponents can't cast spells from anywhere other than their hands.
//! Set: IKO #11 — Ikoria: Lair of Behemoths | Scryfall ID: 98b0a4a8-9319-451b-9b79-b0bca7a41e91 | Oracle ID: aadd10d0-6dd0-4bdc-8d93-ff08e29a5863

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRANNITH_MAGISTRATE,
    oracle_id = "aadd10d0-6dd0-4bdc-8d93-ff08e29a5863",
    scryfall_id = "98b0a4a8-9319-451b-9b79-b0bca7a41e91",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Drannith Magistrate",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(3),
    ),],
    // "From anywhere other than their hands" is the zone read as a filter
    // over the card being cast: a commander in the command zone, a
    // flashback card in a graveyard and an adventure in exile are all one
    // sentence, and a card in hand is the only exception it prints.
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::OpponentsCantCast(&Filter::Not(&Filter::InZone(ZoneRef::Hand)))
    )],
);
