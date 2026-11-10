//! Shocker — {1}{R} — Creature — Insect
//! Oracle: Whenever this creature deals damage to a player, that player discards all the cards in their hand, then draws that many cards.
//! Set: TMP #204 — Tempest | Scryfall ID: 90954848-af7e-47b3-82e7-9fedde6ad606 | Oracle ID: 8981d1d1-e102-4a3d-96ad-3904c83d1611

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHOCKER,
    oracle_id = "8981d1d1-e102-4a3d-96ad-3904c83d1611",
    scryfall_id = "90954848-af7e-47b3-82e7-9fedde6ad606",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shocker",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: PlayerRel::EachPlayer,
        },
        &[
            Effect::DiscardHand {
                who: PlayerRel::DamagedPlayer,
            },
            Effect::DrawCardsFor {
                who: PlayerRel::DamagedPlayer,
                amount: Amount::ThisWay(ThisWay::Discarded),
            },
        ],
    )],
);
