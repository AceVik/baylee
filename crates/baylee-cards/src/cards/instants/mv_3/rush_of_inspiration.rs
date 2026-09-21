//! Rush of Inspiration // Crackling Falls — {1}{U/R}{U/R} — Instant // Land
//! Oracle: Draw two cards. Then discard a card at random unless you pay {E}{E} (two energy counters).
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U} or {R}.
//! Set: MH3 #257 — Modern Horizons 3 | Scryfall ID: 70a25a3a-c12a-49d3-8a91-a108dfa9d3c5 | Oracle ID: bbd569cc-bc21-46df-b8eb-5b5bcd8fe762
//! Face: Rush of Inspiration — {1}{U/R}{U/R} — Instant
//! Face: Crackling Falls —  — Land
// PARTIAL — the instant's "draw two cards" and the whole land face are
// built: it enters tapped and taps for {U} or {R}. The energy payment and the
// random discard are not expressible.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUSH_OF_INSPIRATION,
    oracle_id = "bbd569cc-bc21-46df-b8eb-5b5bcd8fe762",
    scryfall_id = "70a25a3a-c12a-49d3-8a91-a108dfa9d3c5",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[
        face!(
            name = "Rush of Inspiration",
            mana_cost = mana!("{1}{U/R}{U/R}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Crackling Falls",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = &[mana_ability!(&[Effect::mana_choice(&[
                ManaColor::Blue,
                ManaColor::Red,
            ])])],
        ),
    ],
    coverage = Coverage::Partial(
        "the instant's \"Then discard a card at random unless you pay {E}{E}\" is not \
         expressible: nothing in the DSL charges an energy cost, and nothing discards \
         at random",
    ),
    abilities = &[
        // NOT SUPPORTED: "Then discard a card at random unless you pay {E}{E}
        // (two energy counters)." — Effect::PlayerMayPayCostOr charges a
        // CostPart a player answers by naming an object, and no effect or cost
        // pays energy counters; DiscardForPlayers is each player's own choice,
        // and nothing discards at random.
        spell!(&[Effect::draw(2)]),
    ],
);
