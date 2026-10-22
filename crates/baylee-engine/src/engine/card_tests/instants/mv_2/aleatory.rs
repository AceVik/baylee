//! `cards/instants/mv_2/aleatory.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ALEATORY, GRIZZLY_BEARS};

#[test]
fn aleatory_flips_coin_after_blockers_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[ALEATORY])
        .battlefield(0, &[GRIZZLY_BEARS, mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    cast_from_hand(&mut engine, p0, ALEATORY);
    aim_at(&mut engine, p0, attacker);
    pass_until(&mut engine, stack_is_empty);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "Aleatory should draw a card at the beginning of the next turn's upkeep"
    );
}
