//! Keeper of the Flame — {R}{R} — Creature — Human Wizard
//! Oracle: {R}, {T}: Choose target opponent who has more life than you do as you activate this ability. This creature deals 2 damage to that player.
//! Set: EXO #85 — Exodus | Scryfall ID: 9bf246ca-9dfc-400f-8883-acc80ac016e1 | Oracle ID: 8cf12b01-7c0a-488f-9527-404a545c70c4
// PARTIAL — Targeting only an opponent with more life is unsupported; the activated ability is omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KEEPER_OF_THE_FLAME,
    oracle_id = "8cf12b01-7c0a-488f-9527-404a545c70c4",
    scryfall_id = "9bf246ca-9dfc-400f-8883-acc80ac016e1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Targeting only an opponent with more life is unsupported; the activated ability is omitted."
    ),
    faces = &[face!(
        name = "Keeper of the Flame",
        mana_cost = mana!("{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: Targeting only an opponent with more life is unsupported; the activated ability is omitted.
    abilities = &[],
);
