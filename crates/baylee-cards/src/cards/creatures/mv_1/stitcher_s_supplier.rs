//! Stitcher's Supplier — {B} — Creature — Zombie
//! Oracle: When this creature enters or dies, mill three cards. (Put the top three cards of your library into your graveyard.)
//! Set: TDC #196 — Tarkir: Dragonstorm Commander | Scryfall ID: 2edcde06-b326-476e-884d-770187c785fe | Oracle ID: 7fd61a18-6e4f-40c5-aa00-3d101ec1ec82
// IMPLEMENTED — one printed sentence firing on two events: entering the
// battlefield and dying each mill its controller three cards.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STITCHER_S_SUPPLIER,
    oracle_id = "7fd61a18-6e4f-40c5-aa00-3d101ec1ec82",
    scryfall_id = "2edcde06-b326-476e-884d-770187c785fe",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Stitcher's Supplier",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::Mill {
                amount: Amount::Fixed(3),
                target: PlayerRel::You,
            }]
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::Mill {
                amount: Amount::Fixed(3),
                target: PlayerRel::You,
            }]
        ),
    ],
);

// Engine-level test belongs in baylee-engine (mill/zone-change triggers):
// three cards leave the library when it enters and three more when it
// dies, and a bounce to hand fires neither trigger.
