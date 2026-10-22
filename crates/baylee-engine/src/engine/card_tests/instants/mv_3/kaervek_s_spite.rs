//! `cards/instants/mv_3/kaervek_s_spite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn kaervek_s_spite() -> CardIndex {
    card_index("7d69be1e-701a-4bcb-a83e-38b4174f7828")
}

/// `Kaervek's Spite` ({B}{B}{B}, Instant, `Coverage::Partial`) discards your
/// hand as an additional cost and sacrifices all permanents you control and
/// causes target player to lose 5 life on resolution.
#[test]
fn kaervek_s_spite_sacrifices_all_permanents_and_discards_hand_for_five_life_loss() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), grizzly_bears()])
        .hand(0, &[kaervek_s_spite(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, kaervek_s_spite());
    match engine.pending().clone() {
        Pending::ChoosePlayer { .. } => {
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        Pending::ChooseTargets { .. } => {
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .unwrap();
        }
        other => panic!("expected target player choice, got {other:?}"),
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .all(|&id| engine.state().object(id).unwrap().controller != p0)
    );
    assert!(engine.state().zones.list(ZoneLocation::Hand(p0)).is_empty());
    assert_eq!(engine.state().players[1].life, 15);
}
