//! `cards/instants/mv_1/ivory_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, LIGHTNING_BOLT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ivory_charm() -> CardIndex {
    card_index("dded2874-e80f-401a-b700-52c54dd6da60")
}

/// Ivory Charm (dded2874-e80f-401a-b700-52c54dd6da60) — {W} Instant:
/// "Choose one —
/// • All creatures get -2/-0 until end of turn.
/// • Tap target creature.
/// • Prevent the next 1 damage that would be dealt to any target this turn."
#[test]
fn ivory_charm_shrinks_creatures_or_taps_target_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[ivory_charm(), ivory_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    // Mode 0: All creatures get -2/-0 until end of turn.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ivory_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot0 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseMode(slot0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let chars = engine.state().object(bears).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(0), Some(2)));

    // Mode 1: Tap target creature.
    cast_with_floating(&mut engine, P0, ivory_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot1 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseMode(slot1)).unwrap();
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

/// Casts `ivory_charm()` with floating mana and picks the mode `n`.
fn cast_charm_mode(engine: &mut Engine<RegistryLookup>, n: usize) {
    cast_with_floating(engine, P0, ivory_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(m) if m == n))
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
}

/// "All creatures get -2/-0 until end of turn": the caster's own Grizzly
/// Bears shrinks too, and both are 2/2 again on the opponent's turn.
#[test]
fn ivory_charm_shrinks_every_creature_until_end_of_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), GRIZZLY_BEARS])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[ivory_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mine = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    let theirs = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_charm_mode(&mut engine, 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, mine), (0, 2));
    assert_eq!(pt(&engine, theirs), (0, 2));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, mine), (2, 2));
    assert_eq!(pt(&engine, theirs), (2, 2));
}

/// Mode 3: "Prevent the next 1 damage that would be dealt to any target this
/// turn." Aimed at the opponent, the first Lightning Bolt deals 2, and the
/// second deals its full 3.
#[test]
fn ivory_charm_prevents_only_the_next_one_damage() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), mountain(), mountain()])
        .hand(0, &[ivory_charm(), LIGHTNING_BOLT, LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_charm_mode(&mut engine, 2);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    for expected in [18, 15] {
        cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().players[1].life, expected);
    }
}
