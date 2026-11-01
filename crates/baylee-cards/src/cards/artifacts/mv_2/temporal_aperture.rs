//! Temporal Aperture — {2} — Artifact
//! Oracle: {5}, {T}: Shuffle your library, then reveal the top card. Until end of turn, for as long as that card remains on top of your library, play with the top card of your library revealed and you may play that card without paying its mana cost. (If it has X in its mana cost, X is 0.)
//! Set: USG #310 — Urza's Saga | Scryfall ID: 5b5c2555-c707-4814-933a-c275b9ebc0a3 | Oracle ID: 549fe3ff-70aa-4543-b427-40b814b29894
// PARTIAL: playing with the top card of your library revealed and playing it without paying its mana cost until end of turn is not in Effect; activates {5}, {T} to shuffle your library.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEMPORAL_APERTURE,
    oracle_id = "549fe3ff-70aa-4543-b427-40b814b29894",
    scryfall_id = "5b5c2555-c707-4814-933a-c275b9ebc0a3",
    coverage = Coverage::Partial(
        "playing with the top card of your library revealed and playing it without paying its mana cost until end of turn is not in Effect; activates {5}, {T} to shuffle your library",
    ),
    faces = &[face!(
        name = "Temporal Aperture",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    // NOT SUPPORTED: playing with the top card of your library revealed and playing it without paying its mana cost until end of turn is not in Effect; activates {5}, {T} to shuffle your library.
    abilities = &[activated!(
        cost!("{5}", TapSelf),
        &[Effect::ShuffleLibrary {
            who: PlayerRel::You,
        }],
    ),],
);
