//! Black Scarab — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked by black creatures.
//! Oracle: Enchanted creature gets +2/+2 as long as an opponent controls a black permanent.
//! Set: ICE #6 — Ice Age | Scryfall ID: 5bfd4ee1-05f9-45ae-a31d-1225b271dbe6 | Oracle ID: 2774d8b3-dc5f-4281-b4c5-cb003fef0ab2

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

static BLACK_PERMANENT: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
    Filter::ControlledByOpponent,
]);

card!(
    index = index::BLACK_SCARAB,
    oracle_id = "2774d8b3-dc5f-4281-b4c5-cb003fef0ab2",
    scryfall_id = "5bfd4ee1-05f9-45ae-a31d-1225b271dbe6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Black Scarab",
        mana_cost = mana!("{W}"),
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
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::CantBeBlockedBy(&BLACK_CREATURE)
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPT(2, 2),
            condition = Some(Condition::OpponentControlCount(&BLACK_PERMANENT, 1))
        ),
    ],
);
