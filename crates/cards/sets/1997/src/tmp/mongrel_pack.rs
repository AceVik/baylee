//! Mongrel Pack — {3}{G} — Creature — Dog
//! Oracle: When this creature dies during combat, create four 1/1 green Dog creature tokens.
//! Set: TMP #237 — Tempest | Scryfall ID: 56b84315-5287-4401-a4c8-34c192423270 | Oracle ID: e1208d39-86e7-4eae-8dd6-b5bfb29def84
// IMPLEMENTED

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MONGREL_PACK,
    oracle_id = "e1208d39-86e7-4eae-8dd6-b5bfb29def84",
    scryfall_id = "56b84315-5287-4401-a4c8-34c192423270",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mongrel Pack",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DOG],
        power = Some(4),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::CreateTokensFor {
            player: PlayerRel::You,
            token: &generated_tokens::DOG_1_1_GREEN,
            amount: Amount::Fixed(4),
        }],
        condition = Some(Condition::DuringCombat),
    )],
);
