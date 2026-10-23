//! Donate — {2}{U} — Sorcery
//! Oracle: Target player gains control of target permanent you control.
//! Set: UDS #31 — Urza's Destiny | Scryfall ID: 7f6d8ce9-f8c8-45ad-b74c-97fba0e2982e | Oracle ID: 049d1fb2-cce7-40e7-a633-e84cf9782d89
// PARTIAL: targets a permanent you control and a player, and gives your opponent control of that permanent; giving control to a targeted player other than your opponent requires second-target player support in GainControlAll.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DONATE,
    oracle_id = "049d1fb2-cce7-40e7-a633-e84cf9782d89",
    scryfall_id = "7f6d8ce9-f8c8-45ad-b74c-97fba0e2982e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "targets a permanent you control and a player, and gives your opponent control of that permanent; giving control to a targeted player other than your opponent requires second-target player support in GainControlAll",
    ),
    faces = &[face!(
        name = "Donate",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::GainControlAll {
            filter: &Filter::Reference,
            to: PlayerRel::Opponent,
        }],
        targets = Some(TargetReq::one(
            TargetSpec::Object(&Filter::ControlledByYou,)
        )),
        second_targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
