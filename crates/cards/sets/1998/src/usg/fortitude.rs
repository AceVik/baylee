//! Fortitude — {1}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Sacrifice a Forest: Regenerate enchanted creature.
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: USG #253 — Urza's Saga | Scryfall ID: d54d5240-8afc-4c61-aaf6-a78d2b92e5c9 | Oracle ID: 5017c07d-8ad5-4ee2-91d3-974811cdff75
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FORTITUDE,
    oracle_id = "5017c07d-8ad5-4ee2-91d3-974811cdff75",
    scryfall_id = "d54d5240-8afc-4c61-aaf6-a78d2b92e5c9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fortitude",
        mana_cost = mana!("{1}{G}"),
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
        activated!(
            cost!(Sacrifice(&Filter::And(&[
                Filter::ControlledByYou,
                Filter::HasSubtype(subtypes::land::FOREST),
            ]))),
            &[Effect::RegenerateAll {
                filter: &Filter::AttachedToBySource,
            }],
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);
