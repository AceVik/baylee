//! Hermetic Study — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has "{T}: This creature deals 1 damage to any target."
//! Set: USG #78 — Urza's Saga | Scryfall ID: 8321888a-a450-4c15-9461-255cfaa05367 | Oracle ID: 792d7818-ee5c-4254-b969-7b50f475c629
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static HERMETIC_STUDY_PING: AbilityDef = activated!(
    Cost::TAP,
    &[Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::AnyTarget,
    }],
    targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
);

card!(
    index = index::HERMETIC_STUDY,
    oracle_id = "792d7818-ee5c-4254-b969-7b50f475c629",
    scryfall_id = "8321888a-a450-4c15-9461-255cfaa05367",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hermetic Study",
        mana_cost = mana!("{1}{U}"),
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
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&HERMETIC_STUDY_PING)
        ),
    ],
);
