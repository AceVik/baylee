//! Island of Wak-Wak — (no cost) — Land
//! Oracle: {T}: Target creature with flying has base power 0 until end of turn.
//! Set: ME1 #176 — Masters Edition | Scryfall ID: 17f4a1aa-395f-423c-b70c-00cf178bb84d | Oracle ID: d427e61d-5b30-4d2a-bad2-2e7f016036ca
// PARTIAL — the base-power ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ISLAND_OF_WAK_WAK,
    oracle_id = "d427e61d-5b30-4d2a-bad2-2e7f016036ca",
    scryfall_id = "17f4a1aa-395f-423c-b70c-00cf178bb84d",
    faces = &[face!(name = "Island of Wak-Wak", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no Modifier sets base power alone: SetPT writes power and toughness together, and no Amount reads the target's current toughness"
    ),
    // NOT SUPPORTED: "{T}: Target creature with flying has base power 0 until
    // end of turn." — the target and duration are sayable
    // (`TargetSpec::Object` over `Filter::CREATURE` and
    // `Filter::HasKeyword(KeywordSet::FLYING)`, with `Effect::continuous`),
    // but the value is not: `Modifier::SetPT` sets power and toughness
    // outright, and the printed sentence leaves toughness alone — no `Amount`
    // reads the target's current toughness to write beside the 0
    // (`Amount::TargetPower` reads power). The ability comes off the card
    // rather than overwriting the toughness the card does not touch.
);
