//! Shimmering Wings — {U} — Enchantment — Aura
//! Oracle: Enchant creature (Target a creature as you cast this. This card enters attached to that creature.)
//! Oracle: Enchanted creature has flying. (It can't be blocked except by creatures with flying or reach.)
//! Oracle: {U}: Return this Aura to its owner's hand.
//! Set: 10E #107 — Tenth Edition | Scryfall ID: 66c9aa9f-acd5-4908-9e11-1a152363ef42 | Oracle ID: 7d892d61-5a69-4500-9fb2-7cf50728aedd

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIMMERING_WINGS,
    oracle_id = "7d892d61-5a69-4500-9fb2-7cf50728aedd",
    scryfall_id = "66c9aa9f-acd5-4908-9e11-1a152363ef42",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shimmering Wings",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::AddKeyword(KeywordSet::FLYING),
        ),
        activated!(
            cost!("{U}"),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);
