//! Cephalid Coliseum — (no cost) — Land
//! Oracle: {T}: Add {U}. This land deals 1 damage to you.
//! Oracle: Threshold — {U}, {T}, Sacrifice this land: Target player draws three cards, then discards three cards. Activate only if there are seven or more cards in your graveyard.
//! Set: TDC #349 — Tarkir: Dragonstorm Commander | Scryfall ID: 03b9c9ed-fb6f-4f8d-bb1d-7999dec4245c | Oracle ID: c733873e-77db-471f-8061-139db24f7e7c
// PARTIAL — the mana ability is built whole ({T}: Add {U}, then this land
// deals 1 damage to you); the threshold half is left off, see the
// NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CEPHALID_COLISEUM,
    oracle_id = "c733873e-77db-471f-8061-139db24f7e7c",
    scryfall_id = "03b9c9ed-fb6f-4f8d-bb1d-7999dec4245c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(name = "Cephalid Coliseum", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "threshold gate: no Condition states \"seven or more cards in your graveyard\""
    ),
    abilities = &[
        // NOT SUPPORTED: "Threshold — {U}, {T}, Sacrifice this land: Target
        // player draws three cards, then discards three cards. Activate only
        // if there are seven or more cards in your graveyard." The effects are
        // sayable (AnyPlayer target, DrawCardsFor / DiscardForPlayers on
        // PlayerRel::Chosen), but the gate is not: no Condition counts cards in
        // your own graveyard — ControlCount counts permanents you control,
        // OpponentGraveyardCountAtLeast counts an opponent's. Ungated, the
        // ability would be activatable with an empty graveyard, i.e. strictly
        // better than the printed card, so it comes off rather than ships.
        mana_ability!(&[
            Effect::mana(ManaColor::Blue, 1),
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ]),
    ],
);
