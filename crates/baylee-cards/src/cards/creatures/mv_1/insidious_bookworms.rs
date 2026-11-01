//! Insidious Bookworms — {B} — Creature — Worm
//! Oracle: When this creature dies, you may pay {1}{B}. If you do, target player discards a card at random.
//! Set: ALL #51a — Alliances | Scryfall ID: 4bfb7c7e-5a0a-4d4d-be98-ffed0386592b | Oracle ID: ff3eeed9-6801-4cab-a97a-72f5f28acb24
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INSIDIOUS_BOOKWORMS,
    oracle_id = "ff3eeed9-6801-4cab-a97a-72f5f28acb24",
    scryfall_id = "4bfb7c7e-5a0a-4d4d-be98-ffed0386592b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Insidious Bookworms",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WORM],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::You,
            cost: mana!("{1}{B}"),
            effects: &[Effect::DiscardRandom {
                who: PlayerRel::Chosen,
                count: Amount::Fixed(1),
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
