//! Viscera Seer — {B} — Creature — Vampire Wizard
//! Oracle: Sacrifice a creature: Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
//! Set: SOC #229 — Secrets of Strixhaven Commander | Scryfall ID: f511830b-1c1f-4d30-aa5d-4314726d142e | Oracle ID: f82a4e85-526d-4456-b700-7760043a31be
// PARTIAL — a one-drop sacrifice outlet: eat a creature you control to look
// at the top card of your library and send it to the bottom (scry 1).
// NOT SUPPORTED: `Sacrifice a creature` as the activation cost. The cost
// names a permanent to *choose* and an activation has nowhere to ask the
// question, so `abilities::choice_cost_unpayable` has `can_afford` decline
// the ability rather than let `pay_cost` refuse it once the rest is paid.
// It is the Seer's only ability, so the card plays exactly as though the
// line were not printed — a vanilla 1/1 — which is what `Partial` promises
// here (`docs/card-dsl.md`, "Four of those may not appear on an activated
// ability"). It joins Ashnod's Altar and Recurring Nightmare at
// `Implemented` the day an activation can suspend on a choice during cost
// payment.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VISCERA_SEER,
    oracle_id = "f82a4e85-526d-4456-b700-7760043a31be",
    scryfall_id = "f511830b-1c1f-4d30-aa5d-4314726d142e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Viscera Seer",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial("a sacrifice cost cannot be chosen during an activation"),
    abilities = &[activated!(
        cost!(Sacrifice(&Filter::YOUR_CREATURE)),
        &[Effect::scry(1)]
    ),],
);
