//! Ghost Quarter — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
//! Set: CM2 #253 — Commander Anthology Volume II | Scryfall ID: 12f8071c-8955-4aa2-889c-6043df047223 | Oracle ID: 2ec4288e-34c6-4831-a2c0-ba1ca1d9d1dc

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GHOST_QUARTER,
    oracle_id = "2ec4288e-34c6-4831-a2c0-ba1ca1d9d1dc",
    scryfall_id = "12f8071c-8955-4aa2-889c-6043df047223",
    faces = &[face!(name = "Ghost Quarter", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!(TapSelf, SacrificeSelf),
            &[
                Effect::destroy(TargetSpec::Object(&Filter::LAND)),
                Effect::SearchLibraryOf {
                    library: PlayerRel::ControllerOfTarget,
                    owner_searches: true,
                    filter: &Filter::BASIC_LAND,
                    mana_value: None,
                    finds: &[Find::BATTLEFIELD],
                    optional: true,
                },
            ],
            target = Some(TargetSpec::Object(&Filter::LAND)),
        ),
    ],
);
