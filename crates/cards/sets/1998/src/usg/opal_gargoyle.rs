//! Opal Gargoyle — {1}{W} — Enchantment
//! Oracle: When an opponent casts a creature spell, if this permanent is an enchantment, it becomes a 2/2 Gargoyle creature with flying.
//! Set: USG #25 — Urza's Saga | Scryfall ID: a8a67943-8f07-445b-a84c-893879dae7ca | Oracle ID: efef2d33-5354-4fce-880e-cb91fa60f730
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent]);

card!(
    index = index::OPAL_GARGOYLE,
    oracle_id = "efef2d33-5354-4fce-880e-cb91fa60f730",
    scryfall_id = "a8a67943-8f07-445b-a84c-893879dae7ca",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Opal Gargoyle",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&OPPONENT_CREATURE_SPELL),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::GARGOYLE,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::FLYING),
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(2, 2), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
