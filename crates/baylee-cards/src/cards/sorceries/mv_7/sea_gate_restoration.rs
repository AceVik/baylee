//! Sea Gate Restoration // Sea Gate, Reborn — {4}{U}{U}{U} — Sorcery // Land
//! Oracle: Draw cards equal to the number of cards in your hand plus one. You have no maximum hand size for the rest of the game.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {U}.
//! Set: ZNR #76 — Zendikar Rising | Scryfall ID: 193071fe-180b-4d35-ba78-9c16675c29fc | Oracle ID: 4a8d41fe-e04d-484b-a7d1-19be311e6ca7
//! Face: Sea Gate Restoration — {4}{U}{U}{U} — Sorcery
//! Face: Sea Gate, Reborn —  — Land
// PARTIAL — "no maximum hand size for the rest of the game" (a created
// indefinite effect) plus the whole land face: pay 3 life or enter tapped,
// and {T}: Add {U}. The draw itself has no spelling in the DSL.

use baylee_cards_dsl::prelude::*;

/// `{T}: Add {U}.` — the back face's entire rules text.
static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

card!(
    index = index::SEA_GATE_RESTORATION,
    oracle_id = "4a8d41fe-e04d-484b-a7d1-19be311e6ca7",
    scryfall_id = "193071fe-180b-4d35-ba78-9c16675c29fc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Sea Gate Restoration",
            mana_cost = mana!("{4}{U}{U}{U}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Sea Gate, Reborn",
            types = TypeSet::LAND,
            abilities = BACK_MANA,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
        ),
    ],
    coverage = Coverage::Partial(
        "the draw: no Amount adds a constant to a count, so \"equal to the number of cards \
         in your hand plus one\" is not sayable — CountOf would read the hand and nothing adds one"
    ),
    abilities = &[spell!(&[
        // NOT SUPPORTED: "Draw cards equal to the number of cards in your
        // hand plus one." — Amount::CountOf answers the count itself and no
        // variant of Amount adds a constant to it.
        Effect::continuous(
            &Filter::Any,
            Modifier::NoMaxHandSize,
            Duration::Indefinitely
        ),
    ])],
);

// Engine-level coverage: the back face is an MDFC land face, castable from
// hand because it is a land, that enters tapped unless its controller pays
// 3 life and taps for {U}.
