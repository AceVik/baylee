//! `cards/instants/mv_6/fireblast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fireblast() -> CardIndex {
    card_index("9dd7f27a-e862-47d7-9158-034cf4d353b8")
}

/// `Fireblast` ({4}{R}{R}, Instant, `Coverage::Partial`) deals 4 damage to any
/// target.
#[test]
fn fireblast_deals_four_damage_to_any_target() {
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
        .hand(0, &[fireblast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, fireblast());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 16);
}
