//! Ghost Quarter — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
//! Set: CM2 #253 — Commander Anthology Volume II | Scryfall ID: 12f8071c-8955-4aa2-889c-6043df047223 | Oracle ID: 2ec4288e-34c6-4831-a2c0-ba1ca1d9d1dc
// PARTIAL — {T} for {C}; {T}, Sacrifice this land destroys target land and its controller may search up a basic land (tapped, see NOT SUPPORTED).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GHOST_QUARTER,
    oracle_id = "2ec4288e-34c6-4831-a2c0-ba1ca1d9d1dc",
    scryfall_id = "12f8071c-8955-4aa2-889c-6043df047223",
    faces = &[face!(name = "Ghost Quarter", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the destroyed land's controller's search puts the basic land onto the battlefield tapped — OptionalBasicLandSearchFor is Path to Exile's wording, and the DSL has no untapped search for another player"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!(TapSelf, SacrificeSelf),
            &[
                Effect::destroy(TargetSpec::Object(&Filter::LAND)),
                // NOT SUPPORTED: "put it onto the battlefield" — untapped.
                // The only effect that makes another player search is
                // Path to Exile's, and the land it finds enters tapped.
                Effect::OptionalBasicLandSearchFor {
                    player: PlayerRel::ControllerOfTarget,
                },
            ],
            target = Some(TargetSpec::Object(&Filter::LAND)),
        ),
    ],
);
