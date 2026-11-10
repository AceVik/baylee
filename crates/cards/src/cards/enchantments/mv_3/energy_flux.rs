//! Energy Flux — {2}{U} — Enchantment
//! Oracle: All artifacts have "At the beginning of your upkeep, sacrifice this artifact unless you pay {2}."
//! Set: ME4 #48 — Masters Edition IV | Scryfall ID: 78ae772e-cd54-4860-b8a8-94c934853772 | Oracle ID: 7a756cd1-29a8-4edf-bb74-fbb5b4020022
// IMPLEMENTED — one static grant to every artifact (Modifier::GrantTriggered,
// layer 6): its controller's upkeep, pay {2} or sacrifice it.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENERGY_FLUX,
    oracle_id = "7a756cd1-29a8-4edf-bb74-fbb5b4020022",
    scryfall_id = "78ae772e-cd54-4860-b8a8-94c934853772",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Energy Flux",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::ARTIFACT,
        Modifier::GrantTriggered {
            // "your upkeep" and "you pay" are the granted ability's
            // controller — the artifact's controller, not Energy Flux's.
            trigger: Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            effects: &[Effect::PlayerMayPayOr {
                player: PlayerRel::You,
                mana: Amount::Fixed(2),
                effect: &Effect::SacrificeSelf,
            }],
            target: None,
        }
    )],
);
