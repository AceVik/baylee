//! `cards/creatures/mv_3/gliding_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn gliding_licid() -> CardIndex {
    card_index("a07b8619-224d-46d0-a17a-38d04aba574e")
}

/// Gliding Licid (`a07b8619-224d-46d0-a17a-38d04aba574e`) — {2}{U} 2/2 Creature — Licid (Partial):
/// "{U}, {T}: Grants target creature flying until end of turn."
#[test]
fn gliding_licid_grants_target_creature_flying_until_end_of_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gliding_licid(), index::GRIZZLY_BEARS, island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, gliding_licid(), 0);
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bear).contains(KeywordSet::FLYING));
}
