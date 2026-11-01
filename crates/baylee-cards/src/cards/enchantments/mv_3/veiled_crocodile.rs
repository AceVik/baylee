//! Veiled Crocodile — {2}{U} — Enchantment
//! Oracle: When a player has no cards in hand, if this permanent is an enchantment, it becomes a 4/4 Crocodile creature.
//! Set: USG #108 — Urza's Saga | Scryfall ID: 9be15ae1-5262-40ba-937c-217a33d131da | Oracle ID: bdc3ff35-f8d8-477d-b042-94a9f8ed7243
// IMPLEMENTED — animates when any player has an empty hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VEILED_CROCODILE,
    oracle_id = "bdc3ff35-f8d8-477d-b042-94a9f8ed7243",
    scryfall_id = "9be15ae1-5262-40ba-937c-217a33d131da",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Veiled Crocodile",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::State(&Condition::APlayerHandSizeAtMost(0)),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::CROCODILE,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(4, 4), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
