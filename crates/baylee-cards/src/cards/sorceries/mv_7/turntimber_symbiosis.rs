//! Turntimber Symbiosis // Turntimber, Serpentine Wood — {4}{G}{G}{G} — Sorcery // Land
//! Oracle: Look at the top seven cards of your library. You may put a creature card from among them onto the battlefield. If that card has mana value 3 or less, it enters with three additional +1/+1 counters on it. Put the rest on the bottom of your library in a random order.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {G}.
//! Set: ZNR #215 — Zendikar Rising | Scryfall ID: 61bd69ea-1e9e-46b0-b1a1-ed7fdbe3deb6 | Oracle ID: 403b59f3-7ade-4bc2-a3e6-de0c3c700f18
//! Face: Turntimber Symbiosis — {4}{G}{G}{G} — Sorcery
//! Face: Turntimber, Serpentine Wood —  — Land
// PARTIAL — the land face is whole: it enters tapped unless its controller pays
// 3 life, and taps for {G}. The sorcery face has nothing to build:
// NOT SUPPORTED: "Look at the top seven cards of your library. You may put a
// creature card from among them onto the battlefield. If that card has mana
// value 3 or less, it enters with three additional +1/+1 counters on it. Put
// the rest on the bottom of your library in a random order." — no `Effect` looks
// at the top of a library and moves a filtered card from it onto the
// battlefield (`LookAtTopPick` keeps to hand; `SearchLibrary` searches — and
// shuffles), and nothing hangs extra counters off the chosen card's mana value.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TURNTIMBER_SYMBIOSIS,
    oracle_id = "403b59f3-7ade-4bc2-a3e6-de0c3c700f18",
    scryfall_id = "61bd69ea-1e9e-46b0-b1a1-ed7fdbe3deb6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Turntimber Symbiosis",
            mana_cost = mana!("{4}{G}{G}{G}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Turntimber, Serpentine Wood",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
        ),
    ],
    coverage = Coverage::Partial(
        "the sorcery face — look at the top seven, put a creature card onto the \
         battlefield, three +1/+1 counters if its mana value is 3 or less, rest on \
         the bottom — has no effect variant; the land face is implemented in full"
    ),
);
