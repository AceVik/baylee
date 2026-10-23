//! Hidden Guerrillas — {G} — Enchantment
//! Oracle: When an opponent casts an artifact spell, if this permanent is an enchantment, it becomes a 5/3 Soldier creature with trample.
//! Set: USG #261 — Urza's Saga | Scryfall ID: 1ab16fbb-1542-4ba6-9a06-30806176572c | Oracle ID: 4337f35b-baab-4640-9e1c-05f61c49bbdd
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIDDEN_GUERRILLAS,
    oracle_id = "4337f35b-baab-4640-9e1c-05f61c49bbdd",
    scryfall_id = "1ab16fbb-1542-4ba6-9a06-30806176572c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hidden Guerrillas",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::And(&[
            Filter::ARTIFACT,
            Filter::ControlledByOpponent
        ])),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::SOLDIER,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::TRAMPLE),
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(5, 3), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
