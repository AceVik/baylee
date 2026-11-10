//! Spirit Shackle — {B}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature becomes tapped, put a -0/-2 counter on it.
//! Set: ME3 #74 — Masters Edition III | Scryfall ID: 686898fa-353b-46a6-ace3-677a1a88bb3b | Oracle ID: ddb770c1-a783-49c9-a36d-2434b8580743
// IMPLEMENTED — enchant creature; whenever it becomes tapped it gets a -0/-2
// counter.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPIRIT_SHACKLE,
    oracle_id = "ddb770c1-a783-49c9-a36d-2434b8580743",
    scryfall_id = "686898fa-353b-46a6-ace3-677a1a88bb3b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spirit Shackle",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::BecomesTapped(&Filter::AttachedToBySource),
            &[Effect::AddCounter {
                kind: CounterKind::Minus {
                    power: 0,
                    toughness: 2
                },
                amount: Amount::Fixed(1),
            }],
            targets = Some(TargetReq::one(TargetSpec::EventObject))
        ),
    ],
);
