//! Mogg Infestation — {3}{R}{R} — Sorcery
//! Oracle: Destroy all creatures target player controls. For each creature that died this way, that player creates two 1/1 red Goblin creature tokens.
//! Set: TPR #146 — Tempest Remastered | Scryfall ID: bfeb1145-3729-481e-a314-c325ed2f2a35 | Oracle ID: cfd61b53-83c6-4886-8503-ddcb92dc7f85
// PARTIAL: targets a player and destroys all creatures an opponent controls; filtering by the targeted player and creating two 1/1 red Goblin creature tokens for each creature that died this way are not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOGG_INFESTATION,
    oracle_id = "cfd61b53-83c6-4886-8503-ddcb92dc7f85",
    scryfall_id = "bfeb1145-3729-481e-a314-c325ed2f2a35",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "targets a player and destroys all creatures an opponent controls; filtering by the targeted player and creating two 1/1 red Goblin creature tokens for each creature that died this way are not supported"
    ),
    faces = &[face!(
        name = "Mogg Infestation",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DestroyAll {
            filter: &Filter::OPPONENT_CREATURE,
            no_regen: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
