//! Academy Ruins — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{U}, {T}: Put target artifact card from your graveyard on top of your library.
//! Set: 2XM #309 — Double Masters | Scryfall ID: a95b7645-154f-4904-bf71-db7eb24d4df2 | Oracle ID: a3da7d5b-2c2b-45fe-b9c5-413b8c8fc0a2
// IMPLEMENTED — {T}: Add {C}, and {1}{U}, {T}: Put target artifact card from your graveyard on top of your library.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ACADEMY_RUINS,
    oracle_id = "a3da7d5b-2c2b-45fe-b9c5-413b8c8fc0a2",
    scryfall_id = "a95b7645-154f-4904-bf71-db7eb24d4df2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Academy Ruins",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}{U}", TapSelf),
            &[Effect::GraveyardToTop {
                target: TargetSpec::CardInGraveyard(&Filter::ARTIFACT, PlayerRel::You),
            }],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::ARTIFACT,
                PlayerRel::You
            )),
        ),
    ],
);
