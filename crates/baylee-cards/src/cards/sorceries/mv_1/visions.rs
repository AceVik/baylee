//! Visions — {W} — Sorcery
//! Oracle: Look at the top five cards of target player's library. You may then have that player shuffle that library.
//! Set: 4ED #54 — Fourth Edition | Scryfall ID: b0fed0e6-0e56-4987-b0dd-b294156c0233 | Oracle ID: 448fa4e3-7269-446b-8089-675bc1bff5f9
// PARTIAL — the optional shuffle is written; "look at the top five" is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VISIONS,
    oracle_id = "448fa4e3-7269-446b-8089-675bc1bff5f9",
    scryfall_id = "b0fed0e6-0e56-4987-b0dd-b294156c0233",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "no effect only looks at the top cards of a library: every LookAt* \
         effect moves cards, and ReorderTopLibraryOf forces an order this \
         card does not print"
    ),
    faces = &[face!(
        name = "Visions",
        mana_cost = mana!("{W}"),
        types = TypeSet::SORCERY,
    ),],
    // NOT SUPPORTED: "Look at the top five cards of target player's library."
    // — `Effect::ReorderTopLibraryOf` is the nearest piece, but it makes the
    // controller put the cards back in any order, which this card does not
    // say; `Effect::LookAtTopPick`, `Effect::LookAtTopKeepBottomPlay` and
    // `Effect::LookAtTopMayPut` all move cards out of the library, and
    // `Effect::LookAtChosenHand` reads a hand. No effect looks at a library's
    // top cards and leaves them there.
    abilities = &[spell!(
        &[Effect::MayDo {
            effects: &[Effect::ShuffleLibrary {
                who: PlayerRel::Chosen
            }]
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
