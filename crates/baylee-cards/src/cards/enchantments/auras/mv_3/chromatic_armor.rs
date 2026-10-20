//! Chromatic Armor — {1}{W}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: As this Aura enters, choose a color.
//! Oracle: This Aura enters with a sleight counter on it.
//! Oracle: Prevent all damage that would be dealt to enchanted creature by sources of the last chosen color.
//! Oracle: {X}: Put a sleight counter on this Aura and choose a color. X is the number of sleight counters on this Aura.
//! Set: ICE #283 — Ice Age | Scryfall ID: 2657e85b-8f77-41fa-9df2-233443efef43 | Oracle ID: 65b3a40e-1d2b-4b24-996f-c935414ded7d
// PARTIAL: activated {X} sleight-counter color re-choice not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: {X}: Put a sleight counter on this Aura and choose a color. X is the number of sleight counters on this Aura.
card!(
    index = index::CHROMATIC_ARMOR,
    oracle_id = "65b3a40e-1d2b-4b24-996f-c935414ded7d",
    scryfall_id = "2657e85b-8f77-41fa-9df2-233443efef43",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Partial("activated {X} sleight-counter color re-choice not supported"),
    faces = &[face!(
        name = "Chromatic Armor",
        mana_cost = mana!("{1}{W}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        enter_modifiers = &[
            EnterModifier::ChooseColor,
            EnterModifier::WithCounters {
                kind: counters::SLEIGHT,
                amount: Amount::Fixed(1),
            },
        ],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::PreventDamageFrom(&Filter::OfSourceChosenColor)
        ),
    ],
);
