//! Sabertooth Cobra — {2}{G} — Creature — Snake
//! Oracle: Whenever this creature deals damage to a player, that player gets a poison counter. The player gets another poison counter at the beginning of their next upkeep unless they pay {2} before that step. (A player with ten or more poison counters loses the game.)
//! Set: MIR #238 — Mirage | Scryfall ID: 48ead72b-f3f5-4065-a33c-0992cf1fdb34 | Oracle ID: 115c7ca7-c27b-43ca-9281-be6913e90059
// PARTIAL: Delayed next-upkeep poison counter unless the damaged player pays {2} before that step is not in the engine; triggers when Sabertooth Cobra deals damage to a player to give that player a poison counter.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SABERTOOTH_COBRA,
    oracle_id = "115c7ca7-c27b-43ca-9281-be6913e90059",
    scryfall_id = "48ead72b-f3f5-4065-a33c-0992cf1fdb34",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "delayed next-upkeep poison counter unless the damaged player pays {2} before that step is not in the engine; triggers when Sabertooth Cobra deals damage to a player to give that player a poison counter",
    ),
    faces = &[face!(
        name = "Sabertooth Cobra",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: PlayerRel::EachPlayer,
        },
        &[Effect::Poison {
            who: PlayerRel::EventPlayer,
            amount: Amount::Fixed(1),
        }],
    )],
);
