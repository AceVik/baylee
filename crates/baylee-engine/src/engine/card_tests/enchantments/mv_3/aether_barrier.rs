//! `cards/enchantments/mv_3/aether_barrier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Aether Barrier ({2}{U} Enchantment):
/// "Whenever a player casts a creature spell, that player sacrifices a permanent of their choice unless they pay {1}."
#[test]
fn aether_barrier_taxes_creature_spells_or_forces_caster_to_sacrifice_a_permanent() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card_index("0fe602b7-9f88-4d3d-af24-7790df867ed5"),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[grizzly_bears()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // P0 casts Grizzly Bears with 3 Islands (1 extra mana floating) and pays {1}.
    cast_from_hand(&mut engine, P0, grizzly_bears());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| engine.state().object(**id).unwrap().controller == P0)
            .count(),
        5,
        "paying {{1}} spared all of P0's permanents"
    );

    // P1 casts Grizzly Bears with only 2 Forests (no mana left to pay {1}), so P1 declines to pay and must sacrifice a Forest.
    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, grizzly_bears());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected permanent sacrifice prompt");
    };
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, forest()).is_some());
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
}
