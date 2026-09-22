//! Midgar, City of Mako // Reactor Raid — (no cost) — Land — Town // Sorcery — Adventure
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: You may sacrifice an artifact or creature. If you do, draw two cards. (Then exile this card. You may play the land later from exile.)
//! Set: FIN #286 — Final Fantasy | Scryfall ID: 8a837256-6bb4-4a60-962d-d2793548d26c | Oracle ID: 4e34a49d-f031-48ac-a458-97b79124b76c
//! Face: Midgar, City of Mako —  — Land — Town
//! Face: Reactor Raid — {2}{B} — Sorcery — Adventure
// PARTIAL — the land half is built: it enters tapped (EnterModifier::Tapped)
// and taps for {B}. Reactor Raid's reflexive "If you do" draw has no variant,
// so that spell ability is off the card — see the NOT SUPPORTED note below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIDGAR_CITY_OF_MAKO,
    oracle_id = "4e34a49d-f031-48ac-a458-97b79124b76c",
    scryfall_id = "8a837256-6bb4-4a60-962d-d2793548d26c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Reactor Raid's \"If you do, draw two cards\" has no variant — \
         Effect::MayDo would draw them with nothing to sacrifice — so the \
         spell ability is off the card; the land half is implemented",
    ),
    faces = &[
        face!(
            name = "Midgar, City of Mako",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::TOWN],
            enter_modifiers = &[EnterModifier::Tapped],
        ),
        face!(
            name = "Reactor Raid",
            mana_cost = mana!("{2}{B}"),
            types = TypeSet::SORCERY,
            subtypes = &[subtypes::spell::ADVENTURE],
        ),
    ],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);

// NOT SUPPORTED: "You may sacrifice an artifact or creature. If you do, draw
// two cards." — the draw is conditioned on whether the optional sacrifice
// happened, and no variant says that. `Effect::PlayerMayPayCostOr` is the
// other direction ("… unless you <pay>": its effect runs when the price is
// *not* paid), and an `Effect::MayDo` over `Effect::SacrificeFilter { who:
// PlayerRel::You, filter: &Filter::ARTIFACT_OR_CREATURE }` plus
// `Effect::draw(2)` would hand out the two cards on a board with nothing to
// sacrifice, where the printed card hands out none.
