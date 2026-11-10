//! Demolition Field — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Destroy target nonbasic land an opponent controls. That land's controller may search their library for a basic land card, put it onto the battlefield, then shuffle. You may search your library for a basic land card, put it onto the battlefield, then shuffle.
//! Set: FDN #687 — Foundations | Scryfall ID: 0c7e51b6-4898-4632-b39c-3ce438caa882 | Oracle ID: 93953926-a644-49bb-9b5a-4c8f19114c7e

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static OPPONENT_NONBASIC_LAND: Filter = f!(opponents NONBASIC_LAND);

card!(
    index = index::DEMOLITION_FIELD,
    oracle_id = "93953926-a644-49bb-9b5a-4c8f19114c7e",
    scryfall_id = "0c7e51b6-4898-4632-b39c-3ce438caa882",
    faces = &[face!(name = "Demolition Field", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf, SacrificeSelf),
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
                Effect::SearchLibrary {
                    filter: &Filter::BASIC_LAND,
                    finds: &[Find::BATTLEFIELD],
                    optional: true,
                },
            ],
            target = Some(TargetSpec::Object(&OPPONENT_NONBASIC_LAND)),
        ),
    ],
);
