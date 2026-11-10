//! Wild Dogs — {G} — Creature — Dog
//! Oracle: At the beginning of your upkeep, if a player has more life than each other player, the player with the most life gains control of this creature.
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: USG #284 — Urza's Saga | Scryfall ID: 7c7811e2-8bff-44a9-96bf-8e302c3cade9 | Oracle ID: 711cda97-bf57-4b22-b9c1-854ef448d27a
// IMPLEMENTED — most-life upkeep transfer and cycling.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WILD_DOGS,
    oracle_id = "711cda97-bf57-4b22-b9c1-854ef448d27a",
    scryfall_id = "7c7811e2-8bff-44a9-96bf-8e302c3cade9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wild Dogs",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DOG],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::ChangeController {
                new_controller: PlayerRel::MostLife
            }],
            condition = Some(Condition::APlayerHasMostLife),
        ),
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand,
        ),
    ],
);
