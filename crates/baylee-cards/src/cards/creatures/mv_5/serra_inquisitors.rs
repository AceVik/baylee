//! Serra Inquisitors — {4}{W} — Creature — Human Cleric
//! Oracle: Whenever this creature blocks or becomes blocked by one or more black creatures, this creature gets +2/+0 until end of turn.
//! Set: HML #16 — Homelands | Scryfall ID: f1fbe3c8-92fb-41b9-b778-726d22c63054 | Oracle ID: 3c6a32e8-0c2a-45cb-8b79-c7f022fe5129
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

card!(
    index = index::SERRA_INQUISITORS,
    oracle_id = "3c6a32e8-0c2a-45cb-8b79-c7f022fe5129",
    scryfall_id = "f1fbe3c8-92fb-41b9-b778-726d22c63054",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Serra Inquisitors",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedByOneOrMore(&BLACK_CREATURE),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(2, 0),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
