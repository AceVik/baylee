//! Field of Ruin — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Destroy target nonbasic land an opponent controls. Each player searches their library for a basic land card, puts it onto the battlefield, then shuffles.
//! Set: MOC #400 — March of the Machine Commander | Scryfall ID: 143147d2-2eec-41e7-b78a-592288b38630 | Oracle ID: f825c98f-a327-440b-8c0d-ebe02e23bfb7
// PARTIAL — the mana ability and the destroy are built; the each-player
// search is not expressible and is marked below.

use baylee_cards_dsl::prelude::*;

/// "A nonbasic land an opponent controls" — named once because the card
/// refers to it twice: as the target requirement and as the effect's target.
static OPPONENT_NONBASIC_LAND: Filter = f!(opponents NONBASIC_LAND);

card!(
    index = index::FIELD_OF_RUIN,
    oracle_id = "f825c98f-a327-440b-8c0d-ebe02e23bfb7",
    scryfall_id = "143147d2-2eec-41e7-b78a-592288b38630",
    faces = &[face!(name = "Field of Ruin", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the each-player search for a basic land card onto the battlefield, \
         untapped and mandatory, has no form: SearchLibrary reads only its \
         controller's library, and OptionalBasicLandSearchFor is a may that \
         enters tapped"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf, SacrificeSelf),
            &[Effect::destroy(TargetSpec::Object(&OPPONENT_NONBASIC_LAND))],
            target = Some(TargetSpec::Object(&OPPONENT_NONBASIC_LAND))
        ),
        // NOT SUPPORTED: "Each player searches their library for a basic land
        // card, puts it onto the battlefield, then shuffles."
    ],
);
