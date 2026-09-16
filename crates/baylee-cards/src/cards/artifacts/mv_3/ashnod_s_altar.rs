//! Ashnod's Altar — {3} — Artifact
//! Oracle: Sacrifice a creature: Add {C}{C}.
//! Set: CMM #368 — Commander Masters | Scryfall ID: 3c0f7157-a375-499c-92c7-d47d2e95dbad | Oracle ID: 4d18bcba-a346-445e-a182-6cc30b7e066d
// PARTIAL — a sacrifice outlet that is also a mana rock: eat a creature you
// control and get {C}{C} without using the stack (CR 605.1). No tap, so it
// eats as many creatures a turn as you can feed it.
// NOT SUPPORTED: `Sacrifice a creature` as the activation cost. The cost
// names a permanent to *choose* and an activation has nowhere to ask the
// question — `pay_cost` refuses `CostPart::Sacrifice` outright, so
// `can_afford` declines to offer the ability at all rather than offering it
// and taking it back. This is the card's only ability, so the Altar plays
// exactly as though the line were not printed, which is what `Partial`
// promises here (`docs/card-dsl.md`, "Four of those may not appear on an
// activated ability"). It goes to `Implemented` beside Recurring Nightmare
// the day an activation can suspend on a choice during cost payment.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASHNOD_S_ALTAR,
    oracle_id = "4d18bcba-a346-445e-a182-6cc30b7e066d",
    scryfall_id = "3c0f7157-a375-499c-92c7-d47d2e95dbad",
    faces = &[face!(
        name = "Ashnod's Altar",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial("a sacrifice cost cannot be chosen during an activation"),
    abilities = &[mana_ability!(
        cost!(Sacrifice(&Filter::YOUR_CREATURE)),
        &[Effect::mana(ManaColor::Colorless, 2)]
    ),],
);
