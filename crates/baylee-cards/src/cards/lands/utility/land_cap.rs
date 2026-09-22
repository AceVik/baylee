//! Land Cap — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step if it has a depletion counter on it.
//! Oracle: At the beginning of your upkeep, remove a depletion counter from this land.
//! Oracle: {T}: Add {W} or {U}. Put a depletion counter on this land.
//! Set: ICE #357 — Ice Age | Scryfall ID: c4806c02-7a4d-42e3-affd-0338084bd3ab | Oracle ID: bfec4d0a-3792-4bc3-bae1-e639da5bb9a6
// PARTIAL — built: the mana line, as one untargeted mana ability — "{T}: Add
// {W} or {U}. Put a depletion counter on this land." The other two sentences
// have no vocabulary; see the // NOT SUPPORTED: lines below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAND_CAP,
    oracle_id = "bfec4d0a-3792-4bc3-bae1-e639da5bb9a6",
    scryfall_id = "c4806c02-7a4d-42e3-affd-0338084bd3ab",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(name = "Land Cap", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the untap lock is conditional on a counter on the source, which no \
         Modifier can say, and the upkeep clause removes a counter, which no \
         Effect does"
    ),
    // NOT SUPPORTED: "This land doesn't untap during your untap step if it has
    // a depletion counter on it." — Modifier::DoesNotUntap is unconditional, no
    // Modifier carries a Condition (only AddTypeIfCountersAtLeast and
    // AddKeywordIfCountersAtLeast read counters), and static_ability! takes no
    // condition either.
    // NOT SUPPORTED: "At the beginning of your upkeep, remove a depletion
    // counter from this land." — the only counter removal in the DSL is the cost
    // part CostPart::RemoveCounterSelf, and a cost is not an effect.
    abilities = &[mana_ability!(&[
        Effect::mana_choice(&[ManaColor::White, ManaColor::Blue]),
        Effect::AddCounter {
            kind: counters::DEPLETION,
            amount: Amount::Fixed(1),
        },
    ])],
);
