//! Heartwood Dryad's body remains playable without an unprinted actual shadow ability.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;
fn heartwood_dryad() -> CardIndex {
    card_index("976968c5-7ef4-4451-86ff-03a011bbcca7")
}

#[test]
fn heartwood_dryad_casts_and_never_gains_unprinted_shadow() {
    let mut e = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[heartwood_dryad()],
        &[],
    );
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut e, lk::P0, heartwood_dryad(), &[], &[]);
    let dryad = lk::obj(&e, lk::P0, heartwood_dryad());
    assert_eq!(pt(&e, dryad), (2, 1));
    reach_their_main_phase(&mut e, lk::P1);
    assert!(!keywords(&e, dryad).contains(KeywordSet::SHADOW));
    reach_their_main_phase(&mut e, lk::P0);
    let bears = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let menu = lk::attack(&mut e, &[dryad]);
    assert!(lk::may_block(&menu, bears, dryad));
    lk::block(&mut e, &[(bears, dryad)]);
    lk::settle(&mut e);
}
