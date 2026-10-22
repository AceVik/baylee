//! `cards/instants/mv_1/choking_vines.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CHOKING_VINES, HILL_GIANT};

/// `Choking Vines` ({X}{G}, Instant) can only be cast during the declare
/// blockers step, makes X target attacking creatures blocked, and deals 1
/// damage to each of them.
#[test]
fn choking_vines_stops_attacker_combat_damage_and_deals_1_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[CHOKING_VINES])
        .start();
    keep_mulligans(&mut engine);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, CHOKING_VINES);
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().object(giant).unwrap().damage, 1);
}
