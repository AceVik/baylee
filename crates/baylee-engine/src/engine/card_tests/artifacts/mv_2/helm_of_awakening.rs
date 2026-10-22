//! `cards/artifacts/mv_2/helm_of_awakening.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn helm_of_awakening() -> CardIndex {
    card_index("f2fcef65-4873-4b1f-a257-b8648097c990")
}

/// `Helm of Awakening` ({2}, Artifact, `Coverage::Implemented`) makes all
/// spells cost {1} less to cast.
#[test]
fn helm_of_awakening_reduces_spell_costs_by_one_generic() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[helm_of_awakening(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 3 Forests total: pay {2} for Helm of Awakening, then only {G} (1 Forest left) for Grizzly Bears ({1}{G}).
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, helm_of_awakening());
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, p0, GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
}
