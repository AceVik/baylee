//! Emberwilde Djinn — {2}{R}{R} — Creature — Djinn
//! Oracle: Flying
//! Oracle: At the beginning of each player's upkeep, that player may pay {R}{R} or 2 life. If the player does, they gain control of this creature.
//! Set: MIR #172 — Mirage | Scryfall ID: c9e8c9f5-61dd-4dcc-bd40-8f366374ea18 | Oracle ID: 3e1a7146-75d5-499f-8718-303bc6ea139b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMBERWILDE_DJINN,
    oracle_id = "3e1a7146-75d5-499f-8718-303bc6ea139b",
    scryfall_id = "c9e8c9f5-61dd-4dcc-bd40-8f366374ea18",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Emberwilde Djinn",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(5),
        toughness = Some(4),
        keywords = KeywordSet::FLYING,
    ),],
    // "pay {R}{R} or 2 life": two questions, the mana first. A player who
    // declines the mana is asked about the life; declining both leaves the
    // Djinn where it is. Known edge (CR 119.4): the life branch is a
    // `LoseLife`, so a player at 1 life is still offered it and can only
    // kill themselves by taking it; `PlayerMayPayLifeOr` has no paid branch.
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::PlayerMayPayManaEachThenElse {
            player: PlayerRel::ActivePlayer,
            each: mana!("{R}{R}"),
            times: Amount::Fixed(1),
            paid: &[Effect::ChangeController {
                new_controller: PlayerRel::ActivePlayer,
            }],
            unpaid: &[Effect::MayDoFor {
                player: PlayerRel::ActivePlayer,
                effects: &[
                    Effect::LoseLife {
                        amount: Amount::Fixed(2),
                        target: PlayerRel::ActivePlayer,
                    },
                    Effect::ChangeController {
                        new_controller: PlayerRel::ActivePlayer,
                    },
                ],
            }],
        }],
    )],
);
