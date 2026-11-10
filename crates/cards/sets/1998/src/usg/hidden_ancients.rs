//! Hidden Ancients — {1}{G} — Enchantment
//! Oracle: When an opponent casts an enchantment spell, if this permanent is an enchantment, it becomes a 5/5 Treefolk creature.
//! Set: USG #260 — Urza's Saga | Scryfall ID: 45f4f257-c04e-489f-8ffd-814bf075e7d1 | Oracle ID: dc430e1b-d57f-4035-b518-9bc2e26a6c04
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIDDEN_ANCIENTS,
    oracle_id = "dc430e1b-d57f-4035-b518-9bc2e26a6c04",
    scryfall_id = "45f4f257-c04e-489f-8ffd-814bf075e7d1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hidden Ancients",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::And(&[
            Filter::ENCHANTMENT,
            Filter::ControlledByOpponent
        ])),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::TREEFOLK,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(5, 5), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
