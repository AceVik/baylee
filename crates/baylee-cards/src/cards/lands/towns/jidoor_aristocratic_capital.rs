//! Jidoor, Aristocratic Capital // Overture — (no cost) — Land — Town // Sorcery — Adventure
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: Target opponent mills half their library, rounded down. (Then exile this card. You may play the land later from exile.)
//! Set: FIN #284 — Final Fantasy | Scryfall ID: 98b2d5b5-f85b-4c42-a0f5-a76f6af304ba | Oracle ID: bd513d9d-5aa2-4860-bd86-8b5d9430f133
//! Face: Jidoor, Aristocratic Capital —  — Land — Town
//! Face: Overture — {4}{U}{U} — Sorcery — Adventure
// PARTIAL — the land half is whole (it enters tapped and taps for {U}); the
// adventure's mill has no `Amount` that can say "half their library".

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JIDOOR_ARISTOCRATIC_CAPITAL,
    oracle_id = "bd513d9d-5aa2-4860-bd86-8b5d9430f133",
    scryfall_id = "98b2d5b5-f85b-4c42-a0f5-a76f6af304ba",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Jidoor, Aristocratic Capital",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::TOWN],
            enter_modifiers = &[EnterModifier::Tapped],
        ),
        // NOT SUPPORTED: "Target opponent mills half their library, rounded
        // down." — no `Amount` halves anything (`Amount::CountOf` is a whole
        // count of objects) and `ZoneSel` reaches only your own library, so
        // the Overture face carries no spell ability at all.
        face!(
            name = "Overture",
            mana_cost = mana!("{4}{U}{U}"),
            types = TypeSet::SORCERY,
            subtypes = &[subtypes::spell::ADVENTURE],
        ),
    ],
    coverage = Coverage::Partial(
        "Overture's mill — half the target opponent's library, rounded down — \
         has no Amount: nothing halves, and ZoneSel cannot count an opponent's library",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
);
