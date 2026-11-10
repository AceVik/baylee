//! Veil of Birds — {U} — Enchantment
//! Oracle: When an opponent casts a spell, if this permanent is an enchantment, it becomes a 1/1 Bird creature with flying.
//! Set: USG #106 — Urza's Saga | Scryfall ID: 55bd3fac-44b6-4078-a096-8d47b01ea979 | Oracle ID: fc6278e5-7eb1-4590-880e-fe9efa0ec4d9
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VEIL_OF_BIRDS,
    oracle_id = "fc6278e5-7eb1-4590-880e-fe9efa0ec4d9",
    scryfall_id = "55bd3fac-44b6-4078-a096-8d47b01ea979",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Veil of Birds",
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
                    subtype: subtypes::creature::BIRD,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::FLYING),
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(1, 1), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
