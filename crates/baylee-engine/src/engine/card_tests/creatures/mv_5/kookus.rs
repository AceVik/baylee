//! `cards/creatures/mv_5/kookus.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn kookus() -> CardIndex {
    card_index("4c8196e0-c392-4402-b77c-e3d009e9899c")
}

/// `Kookus` ({3}{R}{R}, 3/5 Djinn, `Coverage::Implemented`):
/// `Trample. At the beginning of your upkeep, if you don't control a creature
/// named Keeper of Kookus, this creature deals 3 damage to you and attacks this
/// turn if able. {R}: This creature gets +1/+0 until end of turn.`
#[test]
fn kookus_pumps_and_deals_three_damage_on_upkeep_without_keeper_of_kookus() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[kookus()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, kookus());
    pass_until(&mut engine, stack_is_empty);
    let djinn = on_battlefield(&engine, p0, kookus()).unwrap();
    assert!(keywords(&engine, djinn).contains(KeywordSet::TRAMPLE));

    activate(&mut engine, p0, kookus(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, djinn), (4, 5));

    // Advance to P0's next upkeep without Keeper of Kookus; Kookus deals 3 damage to P0.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().players[0].life, 17);
}
