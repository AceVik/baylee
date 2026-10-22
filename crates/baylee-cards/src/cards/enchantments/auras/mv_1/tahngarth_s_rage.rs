//! Tahngarth's Rage — {R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +3/+0 as long as it's attacking. Otherwise, it gets -2/-1.
//! Set: TMP #209 — Tempest | Scryfall ID: 36658368-88c8-4c55-8147-f6e581f6af36 | Oracle ID: 5b0cb089-087d-4b91-9e63-8186dc737a00

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TAHNGARTH_S_RAGE,
    oracle_id = "5b0cb089-087d-4b91-9e63-8186dc737a00",
    scryfall_id = "36658368-88c8-4c55-8147-f6e581f6af36",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tahngarth's Rage",
        mana_cost = mana!("{R}"),
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
            Filter::And(&[
                Filter::CREATURE,
                Filter::AttachedToBySource,
                Filter::Attacking,
            ]),
            Modifier::ModifyPT(3, 0),
        ),
        static_ability!(
            Filter::And(&[
                Filter::CREATURE,
                Filter::AttachedToBySource,
                Filter::Not(&Filter::Attacking),
            ]),
            Modifier::ModifyPT(-2, -1),
        ),
    ],
);
