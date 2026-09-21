//! Eumidian Hatchery — (no cost) — Land
//! Oracle: {T}, Pay 1 life: Add {B}. Put a hatchling counter on this land.
//! Oracle: When this land is put into a graveyard from the battlefield, for each hatchling counter on it, create a 1/1 black Insect creature token with flying.
//! Set: EOC #20 — Edge of Eternities Commander | Scryfall ID: 25b57aaf-04fa-463d-8516-40fccd24d6ed | Oracle ID: 5b6d933e-2830-4f5a-b244-a421aa9615dc
// PARTIAL — the mana ability is built ("{T}, Pay 1 life: Add {B}" plus the
// hatchling counter); the graveyard trigger is not expressible, see the
// NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

/// Hatchling counters.
///
/// The id is assigned in `baylee_cards_dsl::counters`, which is the one place
/// a card file cannot reach; it is named here so the use site is not a bare
/// number, and it has to move there before a second card prints the word.
const HATCHLING: CounterKind = CounterKind::Custom(5);

// NOT SUPPORTED: "When this land is put into a graveyard from the battlefield, for each hatchling counter on it, create a 1/1 black Insect creature token with flying." — `Amount` has no variant that counts counters on the source (`CountOf` counts objects in a zone, not counters), and no 1/1 black flying Insect token exists in `crate::tokens`, which a card file may not define for itself.

card!(
    index = index::EUMIDIAN_HATCHERY,
    oracle_id = "5b6d933e-2830-4f5a-b244-a421aa9615dc",
    scryfall_id = "25b57aaf-04fa-463d-8516-40fccd24d6ed",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(name = "Eumidian Hatchery", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the graveyard trigger needs an amount that counts counters on the \
         source, which Amount does not carry, and a 1/1 black flying Insect \
         token, which crate::tokens does not have",
    ),
    abilities = &[mana_ability!(
        cost!(TapSelf, PayLife(1)),
        &[
            Effect::mana(ManaColor::Black, 1),
            Effect::AddCounter {
                kind: HATCHLING,
                amount: Amount::Fixed(1),
            },
        ],
    )],
);
