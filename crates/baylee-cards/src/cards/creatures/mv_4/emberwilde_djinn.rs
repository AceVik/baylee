//! Emberwilde Djinn — {2}{R}{R} — Creature — Djinn
//! Oracle: Flying
//! Oracle: At the beginning of each player's upkeep, that player may pay {R}{R} or 2 life. If the player does, they gain control of this creature.
//! Set: MIR #172 — Mirage | Scryfall ID: c9e8c9f5-61dd-4dcc-bd40-8f366374ea18 | Oracle ID: 3e1a7146-75d5-499f-8718-303bc6ea139b
// PARTIAL — upkeep trigger offers the active player to pay {R}{R} to gain control of Emberwilde Djinn; the alternative payment of 2 life is not in PlayerMayPayManaThen.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMBERWILDE_DJINN,
    oracle_id = "3e1a7146-75d5-499f-8718-303bc6ea139b",
    scryfall_id = "c9e8c9f5-61dd-4dcc-bd40-8f366374ea18",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "has flying and at the beginning of each player's upkeep that player may pay {R}{R} to gain control of Emberwilde Djinn; the alternative payment of 2 life instead of {R}{R} is not in PlayerMayPayManaThen",
    ),
    faces = &[face!(
        name = "Emberwilde Djinn",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(5),
        toughness = Some(4),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::ActivePlayer,
            cost: mana!("{R}{R}"),
            effects: &[Effect::ChangeController {
                new_controller: PlayerRel::ActivePlayer,
            }],
        }],
    )],
);
