//! Red Scarab — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked by red creatures.
//! Oracle: Enchanted creature gets +2/+2 as long as an opponent controls a red permanent.
//! Set: ICE #49 — Ice Age | Scryfall ID: 9a734154-5944-42f4-a02e-c426a45847f3 | Oracle ID: 380b6a48-c364-4939-be0b-df542c63a5ff
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static RED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
]);

static RED_PERMANENT: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
    Filter::ControlledByOpponent,
]);

card!(
    index = index::RED_SCARAB,
    oracle_id = "380b6a48-c364-4939-be0b-df542c63a5ff",
    scryfall_id = "9a734154-5944-42f4-a02e-c426a45847f3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Red Scarab",
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
            Modifier::CantBeBlockedBy(&RED_CREATURE)
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPT(2, 2),
            condition = Some(Condition::OpponentControlCount(&RED_PERMANENT, 1))
        ),
    ],
);
