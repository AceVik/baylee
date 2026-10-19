//! Brainwash — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't attack unless its controller pays {3}.
//! Set: 5ED #13 — Fifth Edition | Scryfall ID: 0afcbd6d-2b83-4393-9fc8-fe91f86117fe | Oracle ID: 5a81755f-bc44-47ac-8039-8847a5e37922

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRAINWASH,
    oracle_id = "5a81755f-bc44-47ac-8039-8847a5e37922",
    scryfall_id = "0afcbd6d-2b83-4393-9fc8-fe91f86117fe",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brainwash",
        mana_cost = mana!("{W}"),
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
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::CantAttackUnlessPayMana(3)
        ),
    ],
);
