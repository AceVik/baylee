//! Stunted Growth — {3}{G}{G} — Sorcery
//! Oracle: Target player chooses three cards from their hand and puts them on top of their library in any order.
//! Set: ME2 #179 — Masters Edition II | Scryfall ID: 413bebf6-d47c-4bd2-ad2d-9b1ae70ef066 | Oracle ID: e3480c8f-15ad-451b-bb34-b129805569e1
// NOT SUPPORTED: putting cards from a targeted player's hand on top of their library is not supported; always instructs you to put three cards from your hand on top of your library instead of the targeted player.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STUNTED_GROWTH,
    oracle_id = "e3480c8f-15ad-451b-bb34-b129805569e1",
    scryfall_id = "413bebf6-d47c-4bd2-ad2d-9b1ae70ef066",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "putting cards from a targeted player's hand on top of their library is not supported; always instructs you to put three cards from your hand on top of your library instead of the targeted player",
    ),
    faces = &[face!(
        name = "Stunted Growth",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::PutFromHandOnTop { count: 3 }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
