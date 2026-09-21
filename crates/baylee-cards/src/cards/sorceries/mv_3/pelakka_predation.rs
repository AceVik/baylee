//! Pelakka Predation // Pelakka Caverns — {2}{B} — Sorcery // Land
//! Oracle: Target opponent reveals their hand. You choose a card from it with mana value 3 or greater. That player discards that card.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: ZNR #120 — Zendikar Rising | Scryfall ID: e63f8b20-f45b-4293-9aac-cdc021939be6 | Oracle ID: b0fd6889-20b4-439b-aa97-2e90aca1675a
//! Face: Pelakka Predation — {2}{B} — Sorcery
//! Face: Pelakka Caverns —  — Land
// PARTIAL — the land half is complete (enters tapped, taps for {B}); the
// sorcery half has no shape in the DSL, see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PELAKKA_PREDATION,
    oracle_id = "b0fd6889-20b4-439b-aa97-2e90aca1675a",
    scryfall_id = "e63f8b20-f45b-4293-9aac-cdc021939be6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        // NOT SUPPORTED: "Target opponent reveals their hand. You choose a
        // card from it with mana value 3 or greater. That player discards
        // that card." — no Effect reveals a hand, and none lets the caster
        // pick which card its owner discards. `Effect::DiscardForPlayers`
        // is the discarding player's own choice and carries no mana-value
        // filter, and `Effect::BottomCardFromHand` moves the card to a
        // library rather than to a graveyard.
        face!(
            name = "Pelakka Predation",
            mana_cost = mana!("{2}{B}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Pelakka Caverns",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
        ),
    ],
    coverage = Coverage::Partial(
        "Pelakka Predation: the DSL has no effect that reveals a hand and \
         lets the caster choose which card its owner discards"
    ),
);
