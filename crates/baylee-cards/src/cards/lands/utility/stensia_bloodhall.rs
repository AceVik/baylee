//! Stensia Bloodhall — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.
//! Set: ISD #247 — Innistrad | Scryfall ID: cc2741d8-2c02-4acd-8ca2-55b4bf6aef1c | Oracle ID: 8220c5fa-28dc-40d0-a38a-d8eefc2795d6
// PARTIAL — the {T} mana ability is built; the damage ability is not, because
// its target set is a player *or* a planeswalker.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STENSIA_BLOODHALL,
    oracle_id = "8220c5fa-28dc-40d0-a38a-d8eefc2795d6",
    scryfall_id = "cc2741d8-2c02-4acd-8ca2-55b4bf6aef1c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(name = "Stensia Bloodhall", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the activated ability's target is \"target player or planeswalker\", and \
         TargetSpec has no variant spanning players with a restricted object set",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{3}{B}{R}, {T}: This land deals 2 damage to target player
        // or planeswalker." The cost and the effect are both sayable — cost!("{3}{B}{R}",
        // TapSelf) and Effect::DealDamage { amount: 2, .. } — but the target is not:
        // TargetSpec::AnyTarget is the only variant that spans objects and players and
        // it also offers creatures and battles (CR 115.4), and TargetSpec::AnyPlayer
        // reaches no planeswalker at all. What is missing is a target spec that spans
        // the seats with an object filter beside them.
    ],
);
