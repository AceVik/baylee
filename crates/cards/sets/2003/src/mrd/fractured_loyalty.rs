//! Fractured Loyalty — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature becomes the target of a spell or ability, that spell or ability's controller gains control of that creature. (This effect lasts indefinitely.)
//! Set: MRD #93 — Mirrodin | Scryfall ID: 5c7ce5d5-e51a-4dbc-82f2-b79c88769a7b | Oracle ID: ccc168ef-b93b-4bff-89a3-80984d3625de
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FRACTURED_LOYALTY,
    oracle_id = "ccc168ef-b93b-4bff-89a3-80984d3625de",
    scryfall_id = "5c7ce5d5-e51a-4dbc-82f2-b79c88769a7b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Fractured Loyalty",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::EnchantedBecomesTarget,
            &[Effect::AttachedGainControlBy {
                new_controller: PlayerRel::EventPlayer,
            }],
        ),
    ],
);
