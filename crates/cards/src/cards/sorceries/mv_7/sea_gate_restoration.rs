//! Sea Gate Restoration // Sea Gate, Reborn — {4}{U}{U}{U} — Sorcery // Land
//! Oracle: Draw cards equal to the number of cards in your hand plus one. You have no maximum hand size for the rest of the game.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {U}.
//! Set: ZNR #76 — Zendikar Rising | Scryfall ID: 193071fe-180b-4d35-ba78-9c16675c29fc | Oracle ID: 4a8d41fe-e04d-484b-a7d1-19be311e6ca7
//! Face: Sea Gate Restoration — {4}{U}{U}{U} — Sorcery
//! Face: Sea Gate, Reborn —  — Land

use baylee_cards_dsl::prelude::*;

/// "…equal to the number of cards in your hand plus one", counted once as
/// the spell resolves: the spell itself is on the stack and not in the hand,
/// and the cards it draws arrive after the number is read.
static HAND_PLUS_ONE: Amount = Amount::Plus {
    base: &Amount::CountOf {
        filter: &Filter::Any,
        zone: ZoneSel::HandYou,
    },
    offset: 1,
};

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
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[
        Effect::DrawCards {
            amount: HAND_PLUS_ONE,
        },
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
