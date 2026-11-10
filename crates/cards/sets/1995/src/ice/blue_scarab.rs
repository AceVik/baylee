//! Blue Scarab — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked by blue creatures.
//! Oracle: Enchanted creature gets +2/+2 as long as an opponent controls a blue permanent.
//! Set: ICE #9 — Ice Age | Scryfall ID: b423bb5a-eaac-4c1d-981a-1c635001fc5a | Oracle ID: 47c12d2f-b458-4848-9a01-0fc37ef3d942

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLUE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);

static BLUE_PERMANENT: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
    Filter::ControlledByOpponent,
]);

card!(
    index = index::BLUE_SCARAB,
    oracle_id = "47c12d2f-b458-4848-9a01-0fc37ef3d942",
    scryfall_id = "b423bb5a-eaac-4c1d-981a-1c635001fc5a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blue Scarab",
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
            Modifier::CantBeBlockedBy(&BLUE_CREATURE)
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPT(2, 2),
            condition = Some(Condition::OpponentControlCount(&BLUE_PERMANENT, 1))
        ),
    ],
);
