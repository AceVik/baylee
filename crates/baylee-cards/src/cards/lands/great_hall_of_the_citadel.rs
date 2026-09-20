//! Great Hall of the Citadel — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add two mana in any combination of colors. Spend this mana only to cast legendary spells.
//! Set: LTR #254 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 219c7b57-b62b-42d1-85d9-4b57624a3f54 | Oracle ID: b3e28bcf-0ed0-4406-b615-68ddc55b349a
// IMPLEMENTED — {C} from a bare tap; two mana in any combination of the five
// colors for {1} and a tap, restricted to legendary spells.

use baylee_cards_dsl::prelude::*;

/// "A legendary spell" — the spend restriction on the second ability's mana.
static LEGENDARY_SPELL: Filter = Filter::HasSupertype(SupertypeSet::LEGENDARY);

card!(
    index = index::GREAT_HALL_OF_THE_CITADEL,
    oracle_id = "b3e28bcf-0ed0-4406-b615-68ddc55b349a",
    scryfall_id = "219c7b57-b62b-42d1-85d9-4b57624a3f54",
    faces = &[face!(
        name = "Great Hall of the Citadel",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!("{1}", TapSelf),
            &[Effect::mana_combination(ALL_MANA_COLORS, Amount::Fixed(2))
                .restricted(&LEGENDARY_SPELL, SpendRider::None)]
        ),
    ],
);
