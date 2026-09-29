//! Ragavan, Nimble Pilferer — {R} — Legendary Creature — Monkey Pirate
//! Oracle: Whenever Ragavan deals combat damage to a player, create a Treasure token and exile the top card of that player's library. Until end of turn, you may cast that card.
//! Oracle: Dash {1}{R} (You may cast this spell for its dash cost. If you do, it gains haste, and it's returned from the battlefield to its owner's hand at the beginning of the next end step.)
//! Set: MH2 #138 — Modern Horizons 2 | Scryfall ID: a9738cda-adb1-47fb-9f4c-ecd930228c4d | Oracle ID: 37108cd4-bbab-4ce3-9ed6-f60e8422e703

use crate::tokens::TREASURE;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAGAVAN_NIMBLE_PILFERER,
    oracle_id = "37108cd4-bbab-4ce3-9ed6-f60e8422e703",
    scryfall_id = "a9738cda-adb1-47fb-9f4c-ecd930228c4d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ragavan, Nimble Pilferer",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::MONKEY, subtypes::creature::PIRATE],
        power = Some(2),
        toughness = Some(1),
        // Dash (CR 702.109a): haste and the return at the next end step
        // come with the cost, from the engine.
        dash = Some(mana!("{1}{R}")),
    ),],
    abilities = &[
        // "That player" is the one the combat damage was dealt to; the
        // exiled card is cast, never played, so a land stays in exile.
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::This),
            &[
                Effect::CreateToken { token: &TREASURE },
                Effect::ExileTopMayCast {
                    who: PlayerRel::DamagedPlayer,
                },
            ]
        ),
    ],
);
