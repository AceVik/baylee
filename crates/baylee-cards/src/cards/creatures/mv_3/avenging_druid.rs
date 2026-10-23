//! Avenging Druid — {2}{G} — Creature — Human Druid
//! Oracle: Whenever this creature deals damage to an opponent, you may reveal cards from the top of your library until you reveal a land card. If you do, put that card onto the battlefield and put all other cards revealed this way into your graveyard.
//! Set: EXO #105 — Exodus | Scryfall ID: fca9fd31-639a-4fbc-84bd-c3078df29c0a | Oracle ID: 1f3fb367-60b5-4b36-9604-3064afdac0d9

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVENGING_DRUID,
    oracle_id = "1f3fb367-60b5-4b36-9604-3064afdac0d9",
    scryfall_id = "fca9fd31-639a-4fbc-84bd-c3078df29c0a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Putting the nonland cards revealed by RevealUntil into your graveyard is not supported; reveals cards from the top of your library until a land card is revealed, puts that land onto the battlefield, and puts the rest on the bottom of your library in a random order."
    ),
    faces = &[face!(
        name = "Avenging Druid",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToOpponent(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::RevealUntil {
                filter: &Filter::LAND,
                found: SearchDest::Battlefield,
            }],
        }],
    )],
);
