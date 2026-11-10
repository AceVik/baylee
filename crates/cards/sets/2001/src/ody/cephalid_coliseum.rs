//! Cephalid Coliseum — (no cost) — Land
//! Oracle: {T}: Add {U}. This land deals 1 damage to you.
//! Oracle: Threshold — {U}, {T}, Sacrifice this land: Target player draws three cards, then discards three cards. Activate only if there are seven or more cards in your graveyard.
//! Set: TDC #349 — Tarkir: Dragonstorm Commander | Scryfall ID: 03b9c9ed-fb6f-4f8d-bb1d-7999dec4245c | Oracle ID: c733873e-77db-471f-8061-139db24f7e7c
// IMPLEMENTED — the pain mana line, and draw-three-discard-three gated
// on threshold.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CEPHALID_COLISEUM,
    oracle_id = "c733873e-77db-471f-8061-139db24f7e7c",
    scryfall_id = "03b9c9ed-fb6f-4f8d-bb1d-7999dec4245c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(name = "Cephalid Coliseum", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[
            Effect::mana(ManaColor::Blue, 1),
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ]),
        // "Target player draws three cards, then discards three cards."
        // `PlayerRel::Chosen` is the seat the target named, so both halves
        // read the same answer — and the order matters: a player who draws
        // into seven cards discards from the eight they now hold.
        activated!(
            cost!("{U}", TapSelf, SacrificeSelf),
            &[
                Effect::DrawCardsFor {
                    amount: Amount::Fixed(3),
                    who: PlayerRel::Chosen,
                },
                Effect::DiscardForPlayers {
                    who: PlayerRel::Chosen,
                    count: 3,
                },
            ],
            target = Some(TargetSpec::AnyPlayer),
            condition = Some(Condition::GraveyardCountAtLeast(7)),
        ),
    ],
);
