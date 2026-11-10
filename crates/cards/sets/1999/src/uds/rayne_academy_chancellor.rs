//! Rayne, Academy Chancellor — {2}{U} — Legendary Creature — Human Wizard
//! Oracle: Whenever you or a permanent you control becomes the target of a spell or ability an opponent controls, you may draw a card. You may draw an additional card if Rayne is enchanted.
//! Set: UDS #43 — Urza's Destiny | Scryfall ID: 8ee6480c-7697-47e2-893b-ca88c0ab3376 | Oracle ID: 6933a7bf-10ed-46c8-a49c-ce200bce3eea
// NOT SUPPORTED: drawing an additional card if Rayne is enchanted is omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAYNE_ACADEMY_CHANCELLOR,
    oracle_id = "6933a7bf-10ed-46c8-a49c-ce200bce3eea",
    scryfall_id = "8ee6480c-7697-47e2-893b-ca88c0ab3376",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "triggers whenever you or a permanent you control becomes the target of a spell or ability an opponent controls to let you draw a card; drawing an additional card if Rayne is enchanted is omitted"
    ),
    faces = &[face!(
        name = "Rayne, Academy Chancellor",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::TargetedByOpponent {
            filter: &Filter::ControlledByYou,
            you: true,
        },
        &[Effect::MayDo {
            effects: &[Effect::draw(1)],
        }],
    )],
);
