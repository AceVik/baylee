//! Word of Undoing — {U} — Instant
//! Oracle: Return target creature and all white Auras you own attached to it to their owners' hands.
//! Set: ME1 #56 — Masters Edition | Scryfall ID: c1b5d79e-7b96-445f-a36d-264082a4c6af | Oracle ID: 2597a1ee-fe9e-44b5-a5e3-d34332c6c123

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static WHITE_AURA_YOU_OWN_ON_TARGET: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    Filter::OwnedByYou,
    Filter::AttachedTo(&Filter::Reference),
]);

card!(
    index = index::WORD_OF_UNDOING,
    oracle_id = "2597a1ee-fe9e-44b5-a5e3-d34332c6c123",
    scryfall_id = "c1b5d79e-7b96-445f-a36d-264082a4c6af",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Word of Undoing",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::ReturnAllToHand {
                filter: &WHITE_AURA_YOU_OWN_ON_TARGET,
                opponents_only: false,
            },
            Effect::bounce(TargetSpec::Object(&Filter::CREATURE)),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
