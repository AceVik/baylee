//! Barbarian Ring — (no cost) — Land
//! Oracle: {T}: Add {R}. This land deals 1 damage to you.
//! Oracle: Threshold — {R}, {T}, Sacrifice this land: It deals 2 damage to any target. Activate only if there are seven or more cards in your graveyard.
//! Set: MH3 #299 — Modern Horizons 3 | Scryfall ID: c184406c-b22c-4b9b-9d3a-3e7b17efd8a0 | Oracle ID: eeb9377b-72c1-4214-9a66-0f55577c17d1
// IMPLEMENTED — the pain mana line, and 2 damage gated on threshold.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BARBARIAN_RING,
    oracle_id = "eeb9377b-72c1-4214-9a66-0f55577c17d1",
    scryfall_id = "c184406c-b22c-4b9b-9d3a-3e7b17efd8a0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(name = "Barbarian Ring", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[
            Effect::mana(ManaColor::Red, 1),
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ]),
        // "It deals 2 damage to any target" — the *land* is the source, which
        // is why the damage is written on the ability rather than as a
        // player-dealt effect, and why sacrificing the land first does not
        // take the damage with it (CR 608.2g: the ability resolves whatever
        // happened to its source).
        activated!(
            cost!("{R}", TapSelf, SacrificeSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget,
            }],
            target = Some(TargetSpec::AnyTarget),
            condition = Some(Condition::GraveyardCountAtLeast(7)),
        ),
    ],
);
