//! `cards/creatures/mv_7/delraich.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn subject_card() -> CardIndex {
    card_index("38f9e766-a2ad-437b-a38c-d6f3ffd261fb")
}

/// Delraich — {6}{B} 6/6 Creature — Horror:
/// "Trample." (Partial: sacrifice-three-black-creatures alternative cost unimplemented).
#[test]
fn delraich_casts_for_mana_and_tramples_over_a_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[subject_card()], &[]);
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 6), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, p0, subject_card());
    lk::settle(&mut engine);
    let delraich = lk::obj(&engine, p0, subject_card());
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bears = lk::obj(&engine, p1, index::GRIZZLY_BEARS);
    let blocks = lk::attack(&mut engine, &[delraich]);
    assert!(lk::may_block(&blocks, bears, delraich));
    lk::block(&mut engine, &[(bears, delraich)]);
    lk::through_combat(&mut engine);
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(
        lk::life(&engine, p1),
        16,
        "two damage kills the blocker; four tramples over"
    );
    assert!(on_battlefield(&engine, p0, subject_card()).is_some());
}
