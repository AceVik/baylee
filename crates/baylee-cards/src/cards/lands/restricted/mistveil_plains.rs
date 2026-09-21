//! Mistveil Plains — (no cost) — Land — Plains
//! Oracle: ({T}: Add {W}.)
//! Oracle: This land enters tapped.
//! Oracle: {W}, {T}: Put target card from your graveyard on the bottom of your library. Activate only if you control two or more white permanents.
//! Set: SOC #386 — Secrets of Strixhaven Commander | Scryfall ID: c53028dd-efb5-486c-a7d3-45f2f6050c1d | Oracle ID: bb5c1817-ac22-4779-9005-251bc354f181
// IMPLEMENTED — enters tapped; tap-for-{W} from its Plains type; and a {W},
// {T} activation putting target card from your graveyard on bottom of library,
// gated on two or more white permanents.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static WHITE_PERMANENTS_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    Filter::ControlledByYou,
]);

card!(
    index = index::MISTVEIL_PLAINS,
    oracle_id = "bb5c1817-ac22-4779-9005-251bc354f181",
    scryfall_id = "c53028dd-efb5-486c-a7d3-45f2f6050c1d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mistveil Plains",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::PLAINS],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{W}", TapSelf),
            &[Effect::PutTargetOnBottomOfLibrary],
            target = Some(TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::You)),
            condition = Some(Condition::ControlCount(&WHITE_PERMANENTS_YOU_CONTROL, 2)),
        ),
    ],
);
