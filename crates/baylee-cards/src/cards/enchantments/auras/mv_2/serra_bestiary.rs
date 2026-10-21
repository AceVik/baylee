//! Serra Bestiary — {W}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of your upkeep, sacrifice this Aura unless you pay {W}{W}.
//! Oracle: Enchanted creature can't attack or block, and its activated abilities with {T} in their costs can't be activated.
//! Set: ME4 #27 — Masters Edition IV | Scryfall ID: fdc856b2-c84a-41a1-835e-b8261f0a3fe8 | Oracle ID: 320d8f2b-005d-4769-80d2-f46d96afffe7

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

// NOT SUPPORTED: Modifier::CantActivate forbids all activated abilities rather than only activated abilities with {T} in their costs.
card!(
    index = index::SERRA_BESTIARY,
    oracle_id = "320d8f2b-005d-4769-80d2-f46d96afffe7",
    scryfall_id = "fdc856b2-c84a-41a1-835e-b8261f0a3fe8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "restricting only activated abilities with {T} in their costs on the enchanted creature is not in Modifier; forbids all activated abilities of the enchanted creature along with attacking and blocking"
    ),
    faces = &[face!(
        name = "Serra Bestiary",
        mana_cost = mana!("{W}{W}"),
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
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{W}{W}"),
                effect: &Effect::SacrificeSelf,
            }]
        ),
        static_ability!(
            ENCHANTED_CREATURE,
            Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK))
        ),
        static_ability!(ENCHANTED_CREATURE, Modifier::CantActivate),
    ],
);
