//! Dream Coat — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {0}: Enchanted creature becomes the color or colors of your choice. Activate only once each turn.
//! Set: LEG #51 — Legends | Scryfall ID: 07edbbf4-c3d6-4ec1-ae9b-4ae202fb6998 | Oracle ID: 988aaf54-2e46-4afc-ae37-b0a01191a1f1

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DREAM_COAT,
    oracle_id = "988aaf54-2e46-4afc-ae37-b0a01191a1f1",
    scryfall_id = "07edbbf4-c3d6-4ec1-ae9b-4ae202fb6998",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dream Coat",
        mana_cost = mana!("{U}"),
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
        activated!(
            Cost::FREE,
            &[Effect::ChooseColors {
                subject: ColorSubject::Attached,
                max: 5,
                duration: Duration::Indefinitely,
            }],
            limit = ActivationLimit::PerTurn(1)
        ),
    ],
);
