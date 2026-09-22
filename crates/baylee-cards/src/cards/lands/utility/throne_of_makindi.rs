//! Throne of Makindi — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Put a charge counter on this land.
//! Oracle: {T}, Remove a charge counter from this land: Add two mana of any one color. Spend this mana only to cast kicked spells.
//! Set: ZNR #265 — Zendikar Rising | Scryfall ID: d5a0563e-c83b-40df-abf6-51c83bf6792d | Oracle ID: 7e8198e9-0f3b-420b-ab09-74f13f4fd548
// PARTIAL — the {C} mana ability and "{1}, {T}: Put a charge counter on this
// land" are built. The third ability is NOT SUPPORTED as a whole: its cost
// could be written, but the mana it makes is restricted to kicked spells and
// a `ManaRestriction` carries a `Filter` — nothing in `Filter` matches a
// kicked spell, so the mana would come out spendable on anything, which is a
// land strictly better than the printed one. The ability is left off rather
// than its restriction.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THRONE_OF_MAKINDI,
    oracle_id = "7e8198e9-0f3b-420b-ab09-74f13f4fd548",
    scryfall_id = "d5a0563e-c83b-40df-abf6-51c83bf6792d",
    coverage = Coverage::Partial(
        "no Filter matches a kicked spell, so ManaRestriction cannot say \"spend only to cast kicked spells\""
    ),
    faces = &[face!(name = "Throne of Makindi", types = TypeSet::LAND,)],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::Fixed(1),
            }],
        ),
        // NOT SUPPORTED: {T}, Remove a charge counter from this land: Add two mana of any one color. Spend this mana only to cast kicked spells.
    ],
);
