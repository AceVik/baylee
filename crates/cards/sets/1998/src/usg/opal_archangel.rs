//! Opal Archangel — {4}{W} — Enchantment
//! Oracle: When an opponent casts a creature spell, if this permanent is an enchantment, it becomes a 5/5 Angel creature with flying and vigilance.
//! Set: USG #23 — Urza's Saga | Scryfall ID: a75fca33-fa06-4385-866c-5d463ae6aaf6 | Oracle ID: 888d12d4-aaa5-406f-9dc8-8c4e856eb2fe
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent]);

card!(
    index = index::OPAL_ARCHANGEL,
    oracle_id = "888d12d4-aaa5-406f-9dc8-8c4e856eb2fe",
    scryfall_id = "a75fca33-fa06-4385-866c-5d463ae6aaf6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Opal Archangel",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&OPPONENT_CREATURE_SPELL),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::ANGEL,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::FLYING.union(KeywordSet::VIGILANCE)),
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(5, 5), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
