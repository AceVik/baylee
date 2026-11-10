//! Retaliation — {2}{G} — Enchantment
//! Oracle: Creatures you control have "Whenever this creature becomes blocked by a creature, this creature gets +1/+1 until end of turn."
//! Set: USG #272 — Urza's Saga | Scryfall ID: 7a6e3814-5b9f-40da-8205-263fc49294da | Oracle ID: 51571fa8-fd52-4439-8331-4eb4f651e764
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RETALIATION,
    oracle_id = "51571fa8-fd52-4439-8331-4eb4f651e764",
    scryfall_id = "7a6e3814-5b9f-40da-8205-263fc49294da",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Retaliation",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::GrantTriggered {
            trigger: Trigger::BecomesBlockedBy(&Filter::CREATURE),
            effects: &[Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(1, 1),
                Duration::UntilEndOfTurn,
            )],
            target: None,
        },
    ),],
);
