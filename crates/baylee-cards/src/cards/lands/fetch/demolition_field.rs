//! Demolition Field — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Destroy target nonbasic land an opponent controls. That land's controller may search their library for a basic land card, put it onto the battlefield, then shuffle. You may search your library for a basic land card, put it onto the battlefield, then shuffle.
//! Set: FDN #687 — Foundations | Scryfall ID: 0c7e51b6-4898-4632-b39c-3ce438caa882 | Oracle ID: 93953926-a644-49bb-9b5a-4c8f19114c7e
// PARTIAL — {T} for {C}; {2}, {T}, Sacrifice this land destroys a target
// nonbasic land an opponent controls, its controller may then search a basic
// land up, and you may search one up too. The only search this DSL can hand
// another player puts the land onto the battlefield tapped — see the NOT
// SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

// What the destroy half points at, read twice: once as the ability's target
// requirement and once by the effect. Noun first, like every named filter.
static OPPONENT_NONBASIC_LAND: Filter = f!(opponents NONBASIC_LAND);

card!(
    index = index::DEMOLITION_FIELD,
    oracle_id = "93953926-a644-49bb-9b5a-4c8f19114c7e",
    scryfall_id = "0c7e51b6-4898-4632-b39c-3ce438caa882",
    faces = &[face!(name = "Demolition Field", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the destroyed land's controller's search puts the basic land onto the \
         battlefield tapped — OptionalBasicLandSearchFor is Path to Exile's \
         wording, and the DSL has no untapped search for another player",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf, SacrificeSelf),
            &[
                Effect::destroy(TargetSpec::Object(&OPPONENT_NONBASIC_LAND)),
                // NOT SUPPORTED: "put it onto the battlefield" — untapped.
                // The only effect that makes another player search is
                // Path to Exile's, and the land it finds enters tapped.
                Effect::OptionalBasicLandSearchFor {
                    player: PlayerRel::ControllerOfTarget,
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
