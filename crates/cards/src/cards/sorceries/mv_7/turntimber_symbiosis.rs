//! Turntimber Symbiosis // Turntimber, Serpentine Wood — {4}{G}{G}{G} — Sorcery // Land
//! Oracle: Look at the top seven cards of your library. You may put a creature card from among them onto the battlefield. If that card has mana value 3 or less, it enters with three additional +1/+1 counters on it. Put the rest on the bottom of your library in a random order.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {G}.
//! Set: ZNR #215 — Zendikar Rising | Scryfall ID: 61bd69ea-1e9e-46b0-b1a1-ed7fdbe3deb6 | Oracle ID: 403b59f3-7ade-4bc2-a3e6-de0c3c700f18
//! Face: Turntimber Symbiosis — {4}{G}{G}{G} — Sorcery
//! Face: Turntimber, Serpentine Wood —  — Land
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
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::LookAtTopFind {
        found: Find::BATTLEFIELD.when_matching(
            &Filter::CmcAtMost(3),
            &Find::BATTLEFIELD.with_counter(CounterKind::P1P1, 3),
        ),
        count: 7,
        filter: &Filter::CREATURE,
        random: true,
    }])],
);
