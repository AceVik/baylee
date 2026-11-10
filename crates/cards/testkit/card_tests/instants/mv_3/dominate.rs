//! `cards/instants/mv_3/dominate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Dominate ({X}{1}{U}{U} Instant):
/// "Gain control of target creature with mana value X or less."
#[test]
fn dominate_gains_permanent_control_of_target_creature_with_mana_value_x_or_less() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[card_index("77db8db4-9fe4-478e-b919-5348b0b87b0d")])
        .battlefield(1, &[grizzly_bears(), index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    tap_mana_where(&mut engine, P0, |_| true);
    cast_with_floating(
        &mut engine,
        P0,
        card_index("77db8db4-9fe4-478e-b919-5348b0b87b0d"),
    );
    // Announce X = 2: Grizzly Bears (MV 2) is legal, Hill Giant (MV 3) is not.
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice");
    };
    assert!(options.contains(&bears));
    assert!(!options.contains(&giant));

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bears).unwrap().controller, P0);
    // Control persists past end of turn.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(engine.state().object(bears).unwrap().controller, P0);
}
