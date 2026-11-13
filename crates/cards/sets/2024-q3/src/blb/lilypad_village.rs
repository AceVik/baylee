//! Lilypad Village — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {U}. Spend this mana only to cast a creature spell.
//! Oracle: {U}, {T}: Surveil 2. Activate only if a Bird, Frog, Otter, or Rat entered the battlefield under your control this turn.
//! Set: BLB #255 — Bloomburrow | Scryfall ID: 7e95a7cc-ed77-4ca4-80db-61c0fc68bf50 | Oracle ID: 5bb06e6f-e3af-4caa-b66d-77248ad46b61

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static VILLAGERS: &[baylee_core::ids::SubtypeId] = &[
    creature::BIRD,
    creature::FROG,
    creature::OTTER,
    creature::RAT,
];

card!(
    index = index::LILYPAD_VILLAGE,
    oracle_id = "5bb06e6f-e3af-4caa-b66d-77248ad46b61",
    scryfall_id = "7e95a7cc-ed77-4ca4-80db-61c0fc68bf50",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(name = "Lilypad Village", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana(ManaColor::Blue, 1).restricted(&Filter::CREATURE, SpendRider::None)
        ]),
        activated!(
            cost!("{U}", TapSelf),
            &[Effect::Surveil {
                amount: Amount::Fixed(2)
            }],
            condition = Some(Condition::EnteredSubtypeUnderYourControlThisTurn(VILLAGERS)),
        ),
    ],
);
