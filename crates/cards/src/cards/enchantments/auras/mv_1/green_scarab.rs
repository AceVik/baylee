//! Green Scarab — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't be blocked by green creatures.
//! Oracle: Enchanted creature gets +2/+2 as long as an opponent controls a green permanent.
//! Set: ICE #28 — Ice Age | Scryfall ID: 0fbf9266-c97e-4666-b0fa-1802a69a62cc | Oracle ID: c5ca2df3-6fd9-478d-9bc6-48695a4600fb
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GREEN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
]);

static GREEN_PERMANENT: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
    Filter::ControlledByOpponent,
]);

card!(
    index = index::GREEN_SCARAB,
    oracle_id = "c5ca2df3-6fd9-478d-9bc6-48695a4600fb",
    scryfall_id = "0fbf9266-c97e-4666-b0fa-1802a69a62cc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Green Scarab",
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
            Modifier::CantBeBlockedBy(&GREEN_CREATURE)
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPT(2, 2),
            condition = Some(Condition::OpponentControlCount(&GREEN_PERMANENT, 1))
        ),
    ],
);
