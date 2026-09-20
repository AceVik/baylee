//! Leechridden Swamp — (no cost) — Land — Swamp
//! Oracle: ({T}: Add {B}.)
//! Oracle: This land enters tapped.
//! Oracle: {B}, {T}: Each opponent loses 1 life. Activate only if you control two or more black permanents.
//! Set: DSC #286 — Duskmourn: House of Horror Commander | Scryfall ID: a07a0e31-ace6-40ad-8700-2d58135b5320 | Oracle ID: d83c86c1-126d-49e9-9b13-9e55784c49c5
// IMPLEMENTED — enters tapped; tap-for-{B} from its Swamp type; and a {B},
// {T} drain of each opponent for 1, gated on two or more black permanents
// (Condition::ControlCount, so the ability is an ActivatedConditional).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLACK_PERMANENTS_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
    Filter::ControlledByYou,
]);

card!(
    index = index::LEECHRIDDEN_SWAMP,
    oracle_id = "d83c86c1-126d-49e9-9b13-9e55784c49c5",
    scryfall_id = "a07a0e31-ace6-40ad-8700-2d58135b5320",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Leechridden Swamp",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::SWAMP],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            cost!("{B}", TapSelf),
            &[Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::EachOpponent,
            }],
            condition = Some(Condition::ControlCount(&BLACK_PERMANENTS_YOU_CONTROL, 2)),
        ),
    ],
);
