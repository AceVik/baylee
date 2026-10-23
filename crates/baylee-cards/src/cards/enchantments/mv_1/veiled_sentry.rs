//! Veiled Sentry — {U} — Enchantment
//! Oracle: When an opponent casts a spell, if this permanent is an enchantment, it becomes an Illusion creature with power and toughness each equal to that spell's mana value.
//! Set: USG #109 — Urza's Saga | Scryfall ID: de458a9d-6c09-42bb-b470-c2691e95345a | Oracle ID: 0b9b8e60-51eb-4d4a-95c1-48ca5b8153df
// PARTIAL: setting power and toughness equal to the cast spell's mana value is not in Modifier; when an opponent casts a spell, if this permanent is an enchantment, it becomes a 1/1 Illusion creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VEILED_SENTRY,
    oracle_id = "0b9b8e60-51eb-4d4a-95c1-48ca5b8153df",
    scryfall_id = "de458a9d-6c09-42bb-b470-c2691e95345a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "setting power and toughness equal to the cast spell's mana value is not in Modifier; when an opponent casts a spell, if this permanent is an enchantment, it becomes a 1/1 Illusion creature",
    ),
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
            Effect::continuous(&Filter::This, Modifier::SetPT(1, 1), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
