//! Goblin Shrine — {1}{R}{R} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: As long as enchanted land is a basic Mountain, Goblin creatures get +1/+0.
//! Oracle: When this Aura leaves the battlefield, it deals 1 damage to each Goblin creature.
//! Set: ME4 #122 — Masters Edition IV | Scryfall ID: 07cc3153-ca6a-4a64-9e0b-955417e2c328 | Oracle ID: 5e35df8a-2404-4b3b-888a-25d663bd4383

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BASIC_MOUNTAIN: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
]);

const GOBLIN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::GOBLIN),
]);

card!(
    index = index::GOBLIN_SHRINE,
    oracle_id = "5e35df8a-2404-4b3b-888a-25d663bd4383",
    scryfall_id = "07cc3153-ca6a-4a64-9e0b-955417e2c328",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Shrine",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        abilities = &[
            spell!(
                &[Effect::AttachSelf {
                    target: TargetSpec::Object(&Filter::LAND),
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
            ),
            static_ability!(
                GOBLIN_CREATURE,
                Modifier::ModifyPT(1, 0),
                condition = Some(Condition::AttachedMatches(&BASIC_MOUNTAIN)),
            ),
            triggered!(
                Trigger::LeavesBattlefield(&Filter::This),
                &[Effect::DealDamageEach {
                    amount: Amount::Fixed(1),
                    filter: &GOBLIN_CREATURE,
                }],
            ),
        ],
    ),],
);
