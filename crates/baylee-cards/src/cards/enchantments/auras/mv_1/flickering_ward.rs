//! Flickering Ward — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: As this Aura enters, choose a color.
//! Oracle: Enchanted creature has protection from the chosen color. This effect doesn't remove this Aura.
//! Oracle: {W}: Return this Aura to its owner's hand.
//! Set: SOC #109 — Secrets of Strixhaven Commander | Scryfall ID: 23fcd6a8-be83-4304-ae32-d3dfc855f596 | Oracle ID: 3e2ae59a-40a1-46ca-8f24-3231ebd4e4eb
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const CHOSEN_COLOR_NOT_THIS: Filter =
    Filter::And(&[Filter::OfSourceChosenColor, Filter::Not(&Filter::This)]);

card!(
    index = index::FLICKERING_WARD,
    oracle_id = "3e2ae59a-40a1-46ca-8f24-3231ebd4e4eb",
    scryfall_id = "23fcd6a8-be83-4304-ae32-d3dfc855f596",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Flickering Ward",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        enter_modifiers = &[EnterModifier::ChooseColor],
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
            Modifier::ProtectionFrom(&CHOSEN_COLOR_NOT_THIS),
        ),
        activated!(cost!("{W}"), &[Effect::bounce(TargetSpec::ThisObject)]),
    ],
);
