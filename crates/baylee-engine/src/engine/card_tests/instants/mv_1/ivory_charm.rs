//! `cards/instants/mv_1/ivory_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, IVORY_CHARM};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Ivory Charm (33f629de-1197-4121-9b85-31518d0d0ff6) — {W} Instant:
/// "Choose one —
/// • All creatures get -2/-0 until end of turn.
/// • Tap target creature.
/// • Prevent the next 1 damage that would be dealt to any target this turn."
#[test]
fn ivory_charm_shrinks_creatures_or_taps_target_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[IVORY_CHARM, IVORY_CHARM])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    // Mode 0: All creatures get -2/-0 until end of turn.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, IVORY_CHARM);
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
    cast_with_floating(&mut engine, P0, IVORY_CHARM);
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
