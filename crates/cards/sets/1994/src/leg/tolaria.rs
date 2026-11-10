//! Tolaria — (no cost) — Legendary Land
//! Oracle: {T}: Add {U}.
//! Oracle: {T}: Target creature loses banding and all "bands with other" abilities until end of turn. Activate only during any upkeep step.
//! Set: LEG #308 — Legends | Scryfall ID: d43c01b7-443d-4061-a934-6863d230c9b8 | Oracle ID: 9879a4f3-3b9c-45cf-af03-7f2ae4c689b4

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOLARIA,
    oracle_id = "9879a4f3-3b9c-45cf-af03-7f2ae4c689b4",
    scryfall_id = "d43c01b7-443d-4061-a934-6863d230c9b8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tolaria",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        activated!(
            Cost::TAP,
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::RemoveKeyword(KeywordSet::BANDING),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::LoseAllBandsWithOther,
                    Duration::UntilEndOfTurn,
                ),
            ],
            condition = Some(Condition::DuringStep(StepKind::Upkeep)),
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
