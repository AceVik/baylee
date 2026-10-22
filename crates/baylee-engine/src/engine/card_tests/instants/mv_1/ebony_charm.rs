//! `cards/instants/mv_1/ebony_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{EBONY_CHARM, GRIZZLY_BEARS};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Ebony Charm (2f3237fe-9e07-4aad-8bc8-7ef4b510dd36) — {B} Instant:
/// "Choose one —
/// • Target opponent loses 1 life and you gain 1 life.
/// • Exile up to three target cards from a single graveyard.
/// • Target creature gains fear until end of turn."
#[test]
fn ebony_charm_drains_one_life_or_grants_fear() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), GRIZZLY_BEARS])
        .hand(0, &[EBONY_CHARM, EBONY_CHARM])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();

    // Mode 0: Target opponent loses 1 life and you gain 1 life.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, EBONY_CHARM);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot0 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseMode(slot0)).unwrap();
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().players[0].life, 21);

    // Mode 2: Target creature gains fear until end of turn.
    cast_with_floating(&mut engine, P0, EBONY_CHARM);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot2 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(2)))
        .unwrap();
    engine.apply(P0, PlayerAction::ChooseMode(slot2)).unwrap();
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FEAR)
    );
}
