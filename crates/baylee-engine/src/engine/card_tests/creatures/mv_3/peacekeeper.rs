//! `cards/creatures/mv_3/peacekeeper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, PEACEKEEPER};

/// `Peacekeeper` ({2}{W}, Creature — Human 1/1) prevents creatures from
/// attacking and sacrifices itself at your upkeep unless you pay `{1}{W}`.
#[test]
fn peacekeeper_prevents_creatures_from_attacking() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[PEACEKEEPER, GRIZZLY_BEARS, plains(), plains()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareAttackers
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(
        attackers.is_empty(),
        "Peacekeeper prevents creatures from attacking"
    );
}
