//! Access Tunnel — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}: Target creature with power 3 or less can't be blocked this turn.
//! Set: TDC #337 — Tarkir: Dragonstorm Commander | Scryfall ID: 4bef5957-71a4-4fe0-b2ce-dff8e8690bd9 | Oracle ID: ed9cc560-f30b-4b60-a094-ccf93ed656a7
// IMPLEMENTED — {T}: Add {C}, and the {3} activation targeting through the
// power restriction the card prints.

use baylee_cards_dsl::prelude::*;

/// The restriction is on the **target**, so it is a `TargetSpec` filter and
/// not a check on resolution: CR 115.3 makes an object that does not match
/// an illegal target, so a 4/4 is never offered, and CR 608.2b takes the
/// ability off the stack if the creature grows out of the restriction
/// before it resolves. Reading it on resolution instead would let the
/// player point the ability at anything and be told afterwards.
static SMALL_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::PowerAtMost(3)]);

card!(
    index = index::ACCESS_TUNNEL,
    oracle_id = "ed9cc560-f30b-4b60-a094-ccf93ed656a7",
    scryfall_id = "4bef5957-71a4-4fe0-b2ce-dff8e8690bd9",
    faces = &[face!(name = "Access Tunnel", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::UNBLOCKABLE,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&SMALL_CREATURE)),
        ),
    ],
);
