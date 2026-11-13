//! Parapet — {1}{W} — Enchantment
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Creatures you control get +0/+1.
//! Set: VIS #14 — Visions | Scryfall ID: a7bbcaa9-edbf-48ad-bcd2-65e8fb9bb938 | Oracle ID: 4e73634a-9df8-4c36-b1d2-f366eae902fc
// PARTIAL — creatures you control get +0/+1, cast at sorcery speed only; the flash and its sacrifice rider are not supported.
// NOT SUPPORTED: "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step." Flash is left off on purpose: the DSL cannot say the sacrifice rider (the cast timing is not remembered), and flash without it would be stronger than printed. The enchantment casts at sorcery speed only, where the printed rider never applies.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PARAPET,
    oracle_id = "4e73634a-9df8-4c36-b1d2-f366eae902fc",
    scryfall_id = "a7bbcaa9-edbf-48ad-bcd2-65e8fb9bb938",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "creatures you control get +0/+1, cast at sorcery speed only; casting it as though it had flash, with the sacrifice at the next cleanup step when it is cast at a time a sorcery couldn't have been cast, is not supported (flash is left off, since without the sacrifice it would be stronger than printed)"
    ),
    faces = &[face!(
        name = "Parapet",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::ModifyPT(0, 1)
    )],
);
