//! Dizzying Gaze — {R} — Enchantment — Aura
//! Oracle: Enchant creature you control
//! Oracle: {R}: Enchanted creature deals 1 damage to target creature with flying.
//! Set: EXO #81 — Exodus | Scryfall ID: 71a482cf-a1cd-47b5-a76a-08e03965c679 | Oracle ID: e04f46d9-43e7-45ef-b59c-1810f30bec72

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURE_WITH_FLYING: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::DIZZYING_GAZE,
    oracle_id = "e04f46d9-43e7-45ef-b59c-1810f30bec72",
    scryfall_id = "71a482cf-a1cd-47b5-a76a-08e03965c679",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Having the enchanted creature be the source of damage for this Aura's activated ability is not supported; this Aura deals 1 damage to target creature with flying instead."
    ),
    faces = &[face!(
        name = "Dizzying Gaze",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        activated!(
            cost!("{R}"),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Object(&CREATURE_WITH_FLYING),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&CREATURE_WITH_FLYING))),
        ),
    ],
);
