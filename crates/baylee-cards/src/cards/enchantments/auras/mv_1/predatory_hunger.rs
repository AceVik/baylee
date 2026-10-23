//! Predatory Hunger — {G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever an opponent casts a creature spell, put a +1/+1 counter on enchanted creature.
//! Set: EXO #117 — Exodus | Scryfall ID: db6d9d28-3a05-4dfa-a322-36b4cc2697d4 | Oracle ID: eed2606b-9a27-4af3-8120-c45a1d6b5c00
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PREDATORY_HUNGER,
    oracle_id = "eed2606b-9a27-4af3-8120-c45a1d6b5c00",
    scryfall_id = "db6d9d28-3a05-4dfa-a322-36b4cc2697d4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Predatory Hunger",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::SpellCast(&Filter::OPPONENT_CREATURE),
            &[Effect::ChangeAttachedCounters {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
                remove: false,
            }]
        ),
    ],
);
