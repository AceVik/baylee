//! Gaseous Form — {2}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Prevent all combat damage that would be dealt to and dealt by enchanted creature.
//! Set: EMA #51 — Eternal Masters | Scryfall ID: c5464250-a6a9-4399-a67f-2488073794e9 | Oracle ID: 896af9a9-12a2-4dd6-957c-e52150b5f3a2
// IMPLEMENTED — a combat-damage shield in both directions on the enchanted
// creature (`Modifier::PreventDamageToIt` and `PreventDamageFromIt`).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GASEOUS_FORM,
    oracle_id = "896af9a9-12a2-4dd6-957c-e52150b5f3a2",
    scryfall_id = "c5464250-a6a9-4399-a67f-2488073794e9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gaseous Form",
        mana_cost = mana!("{2}{U}"),
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
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::PreventDamageToIt
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::PreventDamageFromIt
        ),
    ],
);

// Engine-level coverage belongs in `card_tests`: attach Gaseous Form to a
// blocking creature and prove combat damage is prevented in both directions
// while noncombat damage is still dealt.
