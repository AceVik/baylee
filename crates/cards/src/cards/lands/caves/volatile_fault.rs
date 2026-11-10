//! Volatile Fault — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: Destroy target nonbasic land an opponent controls. That player may search their library for a basic land card, put it onto the battlefield, then shuffle. You create a Treasure token.
//! Set: LCI #286 — The Lost Caverns of Ixalan | Scryfall ID: 9385abf3-b067-4586-bf3d-175526cf8f0a | Oracle ID: 95c44f28-f7fa-4785-83b9-0d81be0db0c8

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use crate::tokens::TREASURE;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OPPONENT_NONBASIC_LAND: Filter = f!(opponents NONBASIC_LAND);

card!(
    index = index::VOLATILE_FAULT,
    oracle_id = "95c44f28-f7fa-4785-83b9-0d81be0db0c8",
    scryfall_id = "9385abf3-b067-4586-bf3d-175526cf8f0a",
    faces = &[face!(
        name = "Volatile Fault",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf, SacrificeSelf),
            &[
                Effect::destroy(TargetSpec::Object(&OPPONENT_NONBASIC_LAND)),
                Effect::SearchLibraryOf {
                    library: PlayerRel::ControllerOfTarget,
                    owner_searches: true,
                    filter: &Filter::BASIC_LAND,
                    mana_value: None,
                    finds: &[Find::BATTLEFIELD],
                    optional: true,
                },
                Effect::CreateToken { token: &TREASURE },
            ],
            target = Some(TargetSpec::Object(&OPPONENT_NONBASIC_LAND)),
        ),
    ],
);
