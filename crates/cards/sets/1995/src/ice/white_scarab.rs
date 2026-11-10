//! White Scarab — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked by white creatures.
//! Oracle: Enchanted creature gets +2/+2 as long as an opponent controls a white permanent.
//! Set: ICE #56 — Ice Age | Scryfall ID: c57726b5-dfdd-4e47-bc52-ebf6eedbf3bd | Oracle ID: ec9c3906-8628-48c6-96a7-8c6748cba816

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

static WHITE_PERMANENT: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    Filter::ControlledByOpponent,
]);

card!(
    index = index::WHITE_SCARAB,
    oracle_id = "ec9c3906-8628-48c6-96a7-8c6748cba816",
    scryfall_id = "c57726b5-dfdd-4e47-bc52-ebf6eedbf3bd",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "White Scarab",
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
            Modifier::CantBeBlockedBy(&WHITE_CREATURE)
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPT(2, 2),
            condition = Some(Condition::OpponentControlCount(&WHITE_PERMANENT, 1))
        ),
    ],
);
