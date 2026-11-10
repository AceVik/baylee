//! Vhati il-Dal — {2}{B}{G} — Legendary Creature — Human Warrior
//! Oracle: {T}: Until end of turn, target creature has base power 1 or base toughness 1.
//! Set: TPR #214 — Tempest Remastered | Scryfall ID: a8564611-b678-4336-87e0-5a00a930de48 | Oracle ID: 1759547a-52b3-4bae-817a-658a6fd047e3
// IMPLEMENTED — {T} activated ability that sets target creature's base power to 1 or base toughness to 1 until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CHOICES: &[Effect] = &[
    Effect::ModifyTargets {
        modifier: Modifier::SetPower(1),
        duration: Duration::UntilEndOfTurn,
    },
    Effect::ModifyTargets {
        modifier: Modifier::SetToughness(1),
        duration: Duration::UntilEndOfTurn,
    },
];

card!(
    index = index::VHATI_IL_DAL,
    oracle_id = "1759547a-52b3-4bae-817a-658a6fd047e3",
    scryfall_id = "a8564611-b678-4336-87e0-5a00a930de48",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vhati il-Dal",
        mana_cost = mana!("{2}{B}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::ChooseOne { choices: CHOICES }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
