//! Jester's Mask — {5} — Artifact
//! Oracle: This artifact enters tapped.
//! Oracle: {1}, {T}, Sacrifice this artifact: Target opponent puts the cards from their hand on top of their library. Search that player's library for that many cards. That player puts those cards into their hand, then shuffles.
//! Set: ME2 #211 — Masters Edition II | Scryfall ID: 69d74d40-5642-4ffb-a65e-4a5c48ff8254 | Oracle ID: bcc797b9-d2e5-4fde-8d80-c890c871f344
// NOT SUPPORTED: the activated ability shuffles target opponent's hand into their library without letting you search their library for that many cards to put into their hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JESTER_S_MASK,
    oracle_id = "bcc797b9-d2e5-4fde-8d80-c890c871f344",
    scryfall_id = "69d74d40-5642-4ffb-a65e-4a5c48ff8254",
    coverage = Coverage::Partial(
        "the activated ability shuffles target opponent's hand into their library without letting you search their library for that many cards to put into their hand",
    ),
    faces = &[face!(
        name = "Jester's Mask",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[activated!(
        cost!("{1}", TapSelf, SacrificeSelf),
        &[Effect::ShuffleIntoLibrary {
            who: PlayerRel::Chosen,
            hand: true,
            graveyard: false,
        }],
        target = Some(TargetSpec::AnyOpponent),
    )],
);
