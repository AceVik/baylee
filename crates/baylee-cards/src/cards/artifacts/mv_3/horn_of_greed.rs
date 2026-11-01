//! Horn of Greed — {3} — Artifact
//! Oracle: Whenever a player plays a land, that player draws a card.
//! Set: CN2 #211 — Conspiracy: Take the Crown | Scryfall ID: 14026e22-391c-4a57-a305-665e6066e847 | Oracle ID: b8181d53-1954-4f46-8670-8696440208e8
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HORN_OF_GREED,
    oracle_id = "b8181d53-1954-4f46-8670-8696440208e8",
    scryfall_id = "14026e22-391c-4a57-a305-665e6066e847",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Horn of Greed",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::PlaysLand(PlayerRel::EachPlayer),
        &[Effect::DrawCardsFor {
            amount: Amount::Fixed(1),
            who: PlayerRel::EventPlayer,
        }],
    )],
);
