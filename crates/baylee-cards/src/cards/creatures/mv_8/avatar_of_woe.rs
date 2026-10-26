//! Avatar of Woe — {6}{B}{B} — Creature — Avatar
//! Oracle: If there are ten or more creature cards total in all graveyards, this spell costs {6} less to cast.
//! Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.)
//! Oracle: {T}: Destroy target creature. It can't be regenerated.
//! Set: CM2 #57 — Commander Anthology Volume II | Scryfall ID: 7e8e34de-4b0a-48fe-bb13-c7e181d37e6c | Oracle ID: 2da5aa33-3794-4f83-936c-9d8640833d86
// PARTIAL — 6/5 fear with {T}: destroy target creature (it can't be regenerated).
// NOT SUPPORTED: conditional {6} self-cost reduction when ten or more creature cards total are in all graveyards.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FILTER_FEAR: Filter = Filter::And(&[
    Filter::Not(&Filter::ARTIFACT),
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

card!(
    index = index::AVATAR_OF_WOE,
    oracle_id = "2da5aa33-3794-4f83-936c-9d8640833d86",
    scryfall_id = "7e8e34de-4b0a-48fe-bb13-c7e181d37e6c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FEAR,
    coverage = Coverage::Partial(
        "self-cost reduction when ten or more creature cards are in all graveyards is not supported",
    ),
    faces = &[face!(
        name = "Avatar of Woe",
        mana_cost = mana!("{6}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(6),
        toughness = Some(5),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::CantBeBlockedBy(&FILTER_FEAR)),
        activated!(
            cost!(TapSelf),
            &[Effect::Destroy {
                target: TargetSpec::Object(&Filter::CREATURE),
                no_regen: true
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
    ],
);
