//! Krark-Clan Ironworks — {4} — Artifact
//! Oracle: Sacrifice an artifact: Add {C}{C}.
//! Set: 5DN #134 — Fifth Dawn | Scryfall ID: c60174d6-1f9d-4870-b3db-34d6fcb3f6ab | Oracle ID: 68e1f7e0-a9b3-437f-8086-0c0cb85f2880
// PARTIAL — an artifact sacrifice outlet that turns each one into {C}{C}
// without using the stack (CR 605.1). No tap, so it eats the whole board a
// turn, and "an artifact" includes the Ironworks itself (CR 701.17a limits
// the choice to permanents you control, which `Filter::YOUR_ARTIFACT` says).
// NOT SUPPORTED: `Sacrifice an artifact` as the activation cost. The cost
// names a permanent to *choose* and an activation has nowhere to ask the
// question, so `abilities::choice_cost_unpayable` puts `CostPart::Sacrifice`
// out of `can_afford`'s reach and the ability is never offered — `pay_cost`
// would refuse it after paying the parts before it. This is the card's only
// ability, so the Ironworks plays exactly as though the line were not
// printed, which is what `Partial` promises here (`docs/card-dsl.md`, "Four
// of those may not appear on an activated ability"). It goes to
// `Implemented` beside Ashnod's Altar and Recurring Nightmare the day an
// activation can suspend on a choice during cost payment.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KRARK_CLAN_IRONWORKS,
    oracle_id = "68e1f7e0-a9b3-437f-8086-0c0cb85f2880",
    scryfall_id = "c60174d6-1f9d-4870-b3db-34d6fcb3f6ab",
    faces = &[face!(
        name = "Krark-Clan Ironworks",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial("a sacrifice cost cannot be chosen during an activation"),
    abilities = &[mana_ability!(
        cost!(Sacrifice(&Filter::YOUR_ARTIFACT)),
        &[Effect::mana(ManaColor::Colorless, 2)]
    ),],
);
