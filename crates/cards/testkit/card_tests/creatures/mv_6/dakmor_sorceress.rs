//! `cards/creatures/mv_6/dakmor_sorceress.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dakmor_sorceress() -> CardIndex {
    card_index("be5d9522-40b4-4f45-978b-d78bd00f9491")
}

/// Dakmor Sorceress's power equals the number of Swamps you control (ignoring
/// non-Swamp lands and opponent's Swamps), while toughness stays fixed at 4.
#[test]
fn dakmor_sorceress_power_tracks_controlled_swamps_with_fixed_four_toughness() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dakmor_sorceress(), forest(), island()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(0, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sorceress = on_battlefield(&engine, p0, dakmor_sorceress()).unwrap();
    assert_eq!(
        pt(&engine, sorceress),
        (0, 4),
        "0 controlled Swamps -> 0/4; opponent's Swamps do not count"
    );

    let hand_swamp = in_hand(&engine, p0, swamp()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: hand_swamp })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, sorceress), (1, 4));

    let _ = attack_and_collect_blocks(&mut engine, sorceress, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 19);
}

/// Casting Dakmor Sorceress off six Swamps gives it 6/4 on arrival.
#[test]
fn dakmor_sorceress_cast_off_six_swamps_enters_as_a_six_four() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 6])
        .hand(0, &[dakmor_sorceress()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, dakmor_sorceress());
    pass_until(&mut engine, stack_is_empty);
    let sorceress = on_battlefield(&engine, p0, dakmor_sorceress()).expect("resolved");
    assert_eq!(pt(&engine, sorceress), (6, 4));
}
