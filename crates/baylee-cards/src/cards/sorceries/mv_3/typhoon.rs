//! Typhoon — {2}{G} — Sorcery
//! Oracle: Typhoon deals damage to each opponent equal to the number of Islands that player controls.
//! Set: LEG #209 — Legends | Scryfall ID: 254e0403-67d8-4e73-8d89-c901ebeba49f | Oracle ID: ef295a34-0325-49dd-87a6-546dde395082
// PARTIAL — the per-opponent damage is not written: no `Amount` reads a board
// per damaged player.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TYPHOON,
    oracle_id = "ef295a34-0325-49dd-87a6-546dde395082",
    scryfall_id = "254e0403-67d8-4e73-8d89-c901ebeba49f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Typhoon",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "no amount reads the board of each damaged player: DealDamage evaluates \
         one amount for the whole batch, and CountOf over ControlledByOpponent \
         totals every opponent's Islands at a table of three or more",
    ),
    // NOT SUPPORTED: "Typhoon deals damage to each opponent equal to the
    // number of Islands that player controls." — `Effect::DealDamage {
    // target: TargetSpec::Player(PlayerRel::EachOpponent) }` evaluates its
    // `amount` once for the whole batch, so it cannot count each opponent's
    // Islands separately; `Amount::CountOf` over `Filter::ControlledByOpponent`
    // sums every opponent's Islands at a table of three or more. There is no
    // per-target `Amount` variant to write.
    abilities = &[],
);
