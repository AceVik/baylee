//! Hidden Herd — {G} — Enchantment
//! Oracle: When an opponent plays a nonbasic land, if this permanent is an enchantment, it becomes a 3/3 Beast creature.
//! Set: USG #262 — Urza's Saga | Scryfall ID: 20f07992-1f81-440a-a80a-164c44d0a471 | Oracle ID: 8e7c5649-41c2-4a0c-b873-e6f6c5908567
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIDDEN_HERD,
    oracle_id = "8e7c5649-41c2-4a0c-b873-e6f6c5908567",
    scryfall_id = "20f07992-1f81-440a-a80a-164c44d0a471",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "triggering only when an opponent plays a nonbasic land is not in Trigger::PlaysLand; triggers whenever an opponent plays any land to become a 3/3 Beast creature if it is an enchantment",
    ),
    faces = &[face!(
        name = "Hidden Herd",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::PlaysLand(PlayerRel::EachOpponent),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::BEAST,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(3, 3), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
