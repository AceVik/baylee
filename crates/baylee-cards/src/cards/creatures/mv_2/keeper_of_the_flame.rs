//! Keeper of the Flame — {R}{R} — Creature — Human Wizard
//! Oracle: {R}, {T}: Choose target opponent who has more life than you do as you activate this ability. This creature deals 2 damage to that player.
//! Set: EXO #85 — Exodus | Scryfall ID: 9bf246ca-9dfc-400f-8883-acc80ac016e1 | Oracle ID: 8cf12b01-7c0a-488f-9527-404a545c70c4

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KEEPER_OF_THE_FLAME,
    oracle_id = "8cf12b01-7c0a-488f-9527-404a545c70c4",
    scryfall_id = "9bf246ca-9dfc-400f-8883-acc80ac016e1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Restricting the target opponent to one who has more life than you do is not supported; any target opponent can be chosen to be dealt 2 damage."
    ),
    faces = &[face!(
        name = "Keeper of the Flame",
        mana_cost = mana!("{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{R}", TapSelf),
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::AnyOpponent,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
