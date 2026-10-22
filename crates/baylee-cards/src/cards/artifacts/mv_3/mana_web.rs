//! Mana Web — {3} — Artifact
//! Oracle: Whenever a land an opponent controls is tapped for mana, tap all lands that player controls that could produce any type of mana that land could produce.
//! Set: WTH #152 — Weatherlight | Scryfall ID: 2c72ec90-dacc-496f-a7f5-f18bfce5eb3e | Oracle ID: 82909ff4-f6f7-44f1-bbd0-6e54afa5ee6d
// PARTIAL — Taps all lands that player controls when a land an opponent controls is tapped for mana; filtering by lands that could produce a type of mana that land could produce is not yet modeled.

use baylee_cards_dsl::prelude::*;

static OPPONENT_LAND: Filter = Filter::And(&[Filter::LAND, Filter::ControlledByOpponent]);

card!(
    index = index::MANA_WEB,
    oracle_id = "82909ff4-f6f7-44f1-bbd0-6e54afa5ee6d",
    scryfall_id = "2c72ec90-dacc-496f-a7f5-f18bfce5eb3e",
    coverage = Coverage::Partial(
        "Taps all lands that player controls when a land an opponent controls is tapped for mana; filtering by lands that could produce a type of mana that land could produce is not yet modeled."
    ),
    faces = &[face!(
        name = "Mana Web",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::TappedForMana {
            by: PlayerRel::EachOpponent,
            filter: &OPPONENT_LAND
        },
        &[Effect::TapAllOf {
            who: PlayerRel::EventPlayer,
            filter: &Filter::LAND
        }],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        )))
    ),],
);
