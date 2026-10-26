//! `cards/instants/mv_3/dromar_s_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::generated::index;

/// Dromar's Charm — {W}{U}{B} Instant
/// "Choose one —
/// • You gain 5 life.
/// • Counter target spell.
/// • Target creature gets -2/-2 until end of turn."
#[test]
fn dromars_charm_mode_zero_gains_five_life() {
    let charm = card_index("e9950393-8458-4132-9dc7-01246282a41a");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), island(), swamp(), index::GRIZZLY_BEARS])
        .hand(0, &[charm])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, charm);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 25);
}

#[test]
fn dromars_charm_mode_one_counters_target_spell() {
    let charm = card_index("e9950393-8458-4132-9dc7-01246282a41a");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .battlefield(1, &[plains(), island(), swamp()])
        .hand(1, &[charm])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::GRIZZLY_BEARS);
    let spell_id = on_stack(&engine, index::GRIZZLY_BEARS).expect("Grizzly Bears on stack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && !stack_is_empty(e)
    });
    cast_from_hand(&mut engine, p1, charm);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .unwrap();
    engine.apply(p1, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p0, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn dromars_charm_mode_two_shrinks_target_creature() {
    let charm = card_index("e9950393-8458-4132-9dc7-01246282a41a");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), island(), swamp()])
        .hand(0, &[charm])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant_id = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    assert_eq!(power_toughness(&engine, giant_id), (3, 3));

    cast_from_hand(&mut engine, p0, charm);
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(2)))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(power_toughness(&engine, giant_id), (1, 1));
}
