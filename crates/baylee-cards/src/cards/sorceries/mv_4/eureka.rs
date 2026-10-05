//! Eureka — {2}{G}{G} — Sorcery
//! Oracle: Starting with you, each player may put a permanent card from their hand onto the battlefield. Repeat this process until no one puts a card onto the battlefield.
//! Set: VMA #208 — Vintage Masters | Scryfall ID: 71725234-14fa-4a29-9efc-6dd2366d9798 | Oracle ID: 917f2b4b-79ec-4ded-b4c8-cbcf363cb23b
// PARTIAL — the whole spell is off the card: each player's hidden hand and the
// repetition until no one acts have no vocabulary.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EUREKA,
    oracle_id = "917f2b4b-79ec-4ded-b4c8-cbcf363cb23b",
    scryfall_id = "71725234-14fa-4a29-9efc-6dd2366d9798",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Eureka",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "PutFromHandOntoBattlefield reads only the resolving controller's hand, \
         and nothing repeats an instruction until a full round of players \
         produces no action",
    ),
    // NOT SUPPORTED: "Starting with you, each player may put a permanent card
    // from their hand onto the battlefield. Repeat this process until no one
    // puts a card onto the battlefield." — `Effect::PutFromHandOntoBattlefield`
    // offers only the resolving controller's hand (`optional: true` says the
    // "may" for that one player), no effect offers another player's hidden
    // hand for a put, and no effect repeats an instruction until a full round
    // produces no action (`Effect::Sequence` is a fixed list).
    abilities = &[],
);
