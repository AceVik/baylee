//! Veiled Sentry — {U} — Enchantment
//! Oracle: When an opponent casts a spell, if this permanent is an enchantment, it becomes an Illusion creature with power and toughness each equal to that spell's mana value.
//! Set: USG #109 — Urza's Saga | Scryfall ID: de458a9d-6c09-42bb-b470-c2691e95345a | Oracle ID: 0b9b8e60-51eb-4d4a-95c1-48ca5b8153df
// IMPLEMENTED — animates using the triggering spell mana value.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VEILED_SENTRY,
    oracle_id = "0b9b8e60-51eb-4d4a-95c1-48ca5b8153df",
    scryfall_id = "de458a9d-6c09-42bb-b470-c2691e95345a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Veiled Sentry",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::ControlledByOpponent),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::ILLUSION,
                },
                Duration::Indefinitely,
            ),
            Effect::SetPTFilter {
                filter: &Filter::This,
                power: Amount::EventManaValue,
                toughness: Amount::EventManaValue,
                duration: Duration::Indefinitely,
            },
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
