//! `cards/instants/mv_2/rebound.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn rebound() -> CardIndex {
    card_index("9769ed80-b546-49f5-9329-689e619dabdd")
}

/// Rebound (`9769ed80-b546-49f5-9329-689e619dabdd`) — {1}{U} Instant (Partial):
/// "Change the target of target spell that targets only a player. The new target must be a player."
#[test]
fn rebound_changes_target_of_single_target_spell() {
    let mut engine = lk::game(SEED, &[], &[], &[index::LIGHTNING_BOLT], &[rebound()]);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[], &[P1]);
    let bolt_spell = on_stack(&engine, index::LIGHTNING_BOLT).expect("Lightning Bolt on stack");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(
        &mut engine,
        P1,
        &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P1, rebound());
    lk::aim(&mut engine, &[bolt_spell], &[]);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        lk::aim(&mut engine, &[], &[P0]);
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(engine.state().players[1].life, 20);
}
