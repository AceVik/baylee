//! `cards/creatures/mv_3/calming_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn calming_licid() -> CardIndex {
    card_index("93d20859-249f-47ae-8f6d-095228e7b6e9")
}

/// Calming Licid (`93d20859-249f-47ae-8f6d-095228e7b6e9`) — {2}{W} 2/2 Creature — Licid (Partial):
/// "{W}, {T}: Target creature can't attack this turn."
#[test]
fn calming_licid_prevents_target_creature_from_attacking() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), calming_licid(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, calming_licid(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(!attackers.contains(&bears));
}
