//! Lady Evangela — {W}{U}{B} — Legendary Creature — Human Cleric
//! Oracle: {W}{B}, {T}: Prevent all combat damage that would be dealt by target creature this turn.
//! Set: ME3 #158 — Masters Edition III | Scryfall ID: a01adcbd-0ebd-4f18-9942-4638fa0de358 | Oracle ID: 8800d672-424b-4a7b-886f-7eb9d7a56cfe
// IMPLEMENTED — {W}{B}, {T}: prevent all combat damage the target creature
// would deal this turn (`Modifier::PreventDamageFromIt` until end of turn).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LADY_EVANGELA,
    oracle_id = "8800d672-424b-4a7b-886f-7eb9d7a56cfe",
    scryfall_id = "a01adcbd-0ebd-4f18-9942-4638fa0de358",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Lady Evangela",
        mana_cost = mana!("{W}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{W}{B}", TapSelf),
        &[Effect::continuous(
            &Filter::This,
            Modifier::PreventDamageFromIt,
            Duration::UntilEndOfTurn
        )],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    )],
);
