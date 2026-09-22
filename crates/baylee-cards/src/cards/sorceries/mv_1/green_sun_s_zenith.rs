//! Green Sun's Zenith — {X}{G} — Sorcery
//! Oracle: Search your library for a green creature card with mana value X or less, put it onto the battlefield, then shuffle. Shuffle Green Sun's Zenith into its owner's library.
//! Set: 2X2 #150 — Double Masters 2022 | Scryfall ID: 70291c7b-a86f-4466-8502-c28765a89b2a | Oracle ID: 0d96b60b-a060-48ee-bb83-93f1c4a10669
// PARTIAL — the search is built and bounded by the announced X; where the
// spell itself goes afterwards is not sayable (see below).

use baylee_cards_dsl::prelude::*;

/// "A green creature card with mana value X or less", X being the value
/// announced for this spell.
static GREEN_CREATURE_WITHIN_X: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
    Filter::CmcAtMostX,
]);

card!(
    index = index::GREEN_SUN_S_ZENITH,
    oracle_id = "0d96b60b-a060-48ee-bb83-93f1c4a10669",
    scryfall_id = "70291c7b-a86f-4466-8502-c28765a89b2a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "\"Shuffle Green Sun's Zenith into its owner's library\" — no effect \
         moves the resolving spell anywhere but the graveyard"
    ),
    faces = &[face!(
        name = "Green Sun's Zenith",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::SORCERY,
    ),],
    // NOT SUPPORTED: "Shuffle Green Sun's Zenith into its owner's library."
    // CR 608.2m puts a resolved spell into its owner's graveyard, and the
    // card replaces that — but the only self-moving effect in the DSL is
    // `ExileSelfReturnAsFace`, which is a transform, and
    // `ShuffleGraveyardIntoLibrary` shuffles the whole graveyard rather than
    // this one card. So the Zenith resolves and goes to the graveyard, where
    // the printing recycles it.
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &GREEN_CREATURE_WITHIN_X,
        finds: &[Find::BATTLEFIELD],
        optional: false,
    }])],
);
