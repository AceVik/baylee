//! Ill-Gotten Gains — {2}{B}{B} — Sorcery
//! Oracle: Exile Ill-Gotten Gains. Each player discards their hand, then returns up to three cards from their graveyard to their hand.
//! Set: USG #138 — Urza's Saga | Scryfall ID: 826230ad-6b2b-42a0-9d6f-ed07d3554efd | Oracle ID: c88eff74-def5-42ea-873c-b3b479f6fe18
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ILL_GOTTEN_GAINS,
    oracle_id = "c88eff74-def5-42ea-873c-b3b479f6fe18",
    scryfall_id = "826230ad-6b2b-42a0-9d6f-ed07d3554efd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "each player choosing up to three cards in their graveyard to return to their hand is not in Effect; exiles itself and causes each player to discard their hand",
    ),
    faces = &[face!(
        name = "Ill-Gotten Gains",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::Exile {
            target: TargetSpec::ThisObject,
        },
        Effect::DiscardHand {
            who: PlayerRel::EachPlayer,
        },
    ])],
);
