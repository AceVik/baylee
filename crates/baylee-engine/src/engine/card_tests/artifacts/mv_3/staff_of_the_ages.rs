//! Every family of landwalk is ignored for blocking without removing the abilities.

#[allow(clippy::wildcard_imports)] // The shared canonical-card harness.
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn staff_of_the_ages() -> CardIndex {
    card_index("86af21ba-58ad-45e8-a316-5411c870e415")
}

#[test]
fn staff_allows_basic_snow_and_nonbasic_landwalkers_to_be_blocked_only_while_present() {
    let cases = [
        (index::SHANODIN_DRYADS, index::FOREST),
        (index::RIME_DRYAD, index::SNOW_COVERED_FOREST),
        (index::DESERT_NOMADS, index::DESERT),
    ];
    for (walker, land) in cases {
        for (cast_staff, remove_staff) in [(false, false), (true, false), (true, true)] {
            let mut e = lk::game(
                SEED,
                &[walker],
                &[land, index::GRIZZLY_BEARS],
                &[staff_of_the_ages(), index::DISENCHANT],
                &[],
            );
            let attacker = lk::obj(&e, P0, walker);
            let blocker = lk::obj(&e, P1, index::GRIZZLY_BEARS);
            if cast_staff {
                lk::float(&mut e, P0, &[(ManaColor::Colorless, 3)]);
                lk::cast_at(&mut e, P0, staff_of_the_ages(), &[], &[]);
                if walker == index::SHANODIN_DRYADS {
                    assert!(
                        keywords(&e, attacker).contains(KeywordSet::FORESTWALK),
                        "Staff allows blocking without removing forestwalk"
                    );
                }
                if remove_staff {
                    let staff = lk::obj(&e, P0, staff_of_the_ages());
                    lk::float(
                        &mut e,
                        P0,
                        &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
                    );
                    lk::cast_at(&mut e, P0, index::DISENCHANT, &[staff], &[]);
                    assert!(in_graveyard(&e, P0, staff_of_the_ages()).is_some());
                }
            }
            let blocks = lk::attack(&mut e, &[attacker]);
            assert_eq!(
                lk::may_block(&blocks, blocker, attacker),
                cast_staff && !remove_staff,
                "landwalker {walker:?}: Staff on battlefield={}",
                cast_staff && !remove_staff
            );
        }
    }
}
