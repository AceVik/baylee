//! Agility — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+1 and has flanking. (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Set: MIR #154 — Mirage | Scryfall ID: 5694cb42-7489-40c5-b21a-aeb36636015f | Oracle ID: 0f5f8b47-d104-4e29-b052-02d730d76ec0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::AGILITY,
    oracle_id = "0f5f8b47-d104-4e29-b052-02d730d76ec0",
    scryfall_id = "5694cb42-7489-40c5-b21a-aeb36636015f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Agility",
        mana_cost = mana!("{1}{R}"),
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
        static_ability!(ENCHANTED_CREATURE, Modifier::ModifyPT(1, 1)),
        static_ability!(
            ENCHANTED_CREATURE,
            Modifier::AddKeyword(KeywordSet::FLANKING)
        ),
    ],
);
