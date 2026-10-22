//! Favorable Destiny — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+2 as long as it's white.
//! Oracle: Enchanted creature has shroud as long as its controller controls another creature. (It can't be the target of spells or abilities.)
//! Set: MIR #16 — Mirage | Scryfall ID: e5657403-7c86-4eb6-84b0-75eedb04a5a2 | Oracle ID: ee04afb0-184f-4a13-8d44-0fbe121b1eff
// PARTIAL — Condition::ControlCount checks whether the Aura's controller controls at least two creatures rather than whether the enchanted creature's controller controls another creature when those controllers differ.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::AttachedToBySource,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

card!(
    index = index::FAVORABLE_DESTINY,
    oracle_id = "ee04afb0-184f-4a13-8d44-0fbe121b1eff",
    scryfall_id = "e5657403-7c86-4eb6-84b0-75eedb04a5a2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "gives enchanted white creature +1/+2 and grants enchanted creature shroud while you control at least two creatures; checking whether the enchanted creature's controller controls another creature when controlled by another player is not in Condition",
    ),
    faces = &[face!(
        name = "Favorable Destiny",
        mana_cost = mana!("{1}{W}"),
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
        static_ability!(ENCHANTED_WHITE_CREATURE, Modifier::ModifyPT(1, 2)),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::AddKeyword(KeywordSet::SHROUD),
            condition = Some(Condition::ControlCount(&Filter::CREATURE, 2)),
        ),
    ],
);
