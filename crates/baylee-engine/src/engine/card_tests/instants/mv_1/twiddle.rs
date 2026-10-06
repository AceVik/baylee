//! `cards/instants/mv_1/twiddle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Twiddle: "You may tap or untap target artifact, creature, or land."
/// Targets an untapped land and accepts the "may": it comes back tapped.
#[test]
fn twiddle_toggles_the_tapped_state_of_its_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[twiddle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let islands = all_on_battlefield(&engine, p0, island());
    let (payer, target) = (islands[0], islands[1]);
    assert!(!is_tapped(&engine, target), "untapped before");
    tap_mana_where(&mut engine, p0, |id| id == payer);
    cast_with_floating(&mut engine, p0, twiddle());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("an untapped land is a legal target");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, target),
        "\"tap or untap\" — tapped, since it started untapped"
    );
}

/// Twiddle declined: "you may" answered no leaves the target exactly as it
/// was.
#[test]
fn twiddle_declined_leaves_its_target_unchanged() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[twiddle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let islands = all_on_battlefield(&engine, p0, island());
    let (payer, target) = (islands[0], islands[1]);
    tap_mana_where(&mut engine, p0, |id| id == payer);
    cast_with_floating(&mut engine, p0, twiddle());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("an untapped land is a legal target");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, target),
        "declined \"you may\" — nothing happens"
    );
}
