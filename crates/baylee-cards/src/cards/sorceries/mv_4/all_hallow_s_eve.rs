//! All Hallow's Eve — {2}{B}{B} — Sorcery
//! Oracle: Exile All Hallow's Eve with two scream counters on it.
//! Oracle: At the beginning of your upkeep, if this card is exiled with a scream counter on it, remove a scream counter from it. If there are no more scream counters on it, put it into your graveyard and each player returns all creature cards from their graveyard to the battlefield.
//! Set: ME3 #57 — Masters Edition III | Scryfall ID: af266eec-e61b-4bda-be7f-3a5a59eee864 | Oracle ID: ebed900f-6de3-4ae7-9795-fae34882298d
// PARTIAL — no clause is written: scream counters, an ability working from
// exile and the mass return each lack vocabulary.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALL_HALLOW_S_EVE,
    oracle_id = "ebed900f-6de3-4ae7-9795-fae34882298d",
    scryfall_id = "af266eec-e61b-4bda-be7f-3a5a59eee864",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "All Hallow's Eve",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "no scream counter id exists, no ability functions from exile, and the \
         mass return is only yours: AllGraveyardCreaturesToBattlefield puts \
         every graveyard's creatures under your control",
    ),
    // NOT SUPPORTED: "Exile All Hallow's Eve with two scream counters on it."
    // — no `counters::SCREAM` id is assigned in the DSL, `Effect::ExileSource`
    // exiles without counters, and nothing can put or read counters on a card
    // in exile.
    // NOT SUPPORTED: "At the beginning of your upkeep, if this card is exiled
    // with a scream counter on it, remove a scream counter from it. If there
    // are no more scream counters on it, put it into your graveyard and each
    // player returns all creature cards from their graveyard to the
    // battlefield." — `TriggerZone` is `Battlefield` or `Graveyard`, so no
    // ability functions from exile; no effect removes a counter from a card
    // in exile or moves the source from exile to a graveyard; and
    // `AllGraveyardCreaturesToBattlefield` puts every graveyard's creature
    // cards onto the battlefield under **your** control where this returns
    // each player's own.
    abilities = &[],
);
