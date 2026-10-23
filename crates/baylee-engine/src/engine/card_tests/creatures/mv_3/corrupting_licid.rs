//! `cards/creatures/mv_3/corrupting_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn corrupting_licid() -> CardIndex {
    card_index("503ddd47-4b0c-4a2a-b77a-fb321044b438")
}

/// Corrupting Licid (`503ddd47-4b0c-4a2a-b77a-fb321044b438`) — {2}{B} 2/2 Creature — Licid (Partial):
/// "{B}, {T}: Target creature gains fear until end of turn."
#[test]
fn corrupting_licid_grants_fear_to_target_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), corrupting_licid(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, corrupting_licid(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::FEAR));
}
