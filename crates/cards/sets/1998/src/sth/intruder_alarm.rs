//! Intruder Alarm — {2}{U} — Enchantment
//! Oracle: Creatures don't untap during their controllers' untap steps.
//! Oracle: Whenever a creature enters, untap all creatures.
//! Set: 8ED #86 — Eighth Edition | Scryfall ID: 8d62468e-ed9d-4932-bbbd-103b33b98b25 | Oracle ID: 1e943e04-e213-4781-b1a7-935aad8790e1
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INTRUDER_ALARM,
    oracle_id = "1e943e04-e213-4781-b1a7-935aad8790e1",
    scryfall_id = "8d62468e-ed9d-4932-bbbd-103b33b98b25",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Intruder Alarm",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(Filter::CREATURE, Modifier::DoesNotUntap),
        triggered!(
            Trigger::EntersBattlefield(&Filter::CREATURE),
            &[Effect::UntapAll {
                filter: &Filter::CREATURE,
            }],
        ),
    ],
);
