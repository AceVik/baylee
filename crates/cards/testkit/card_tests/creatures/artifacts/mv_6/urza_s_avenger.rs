//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn avenger_chooses_one_keyword_on_resolution_and_always_shrinks() {
    let p0 = PlayerId::new(0);
    for (mode, keyword) in [
        KeywordSet::BANDING,
        KeywordSet::FLYING,
        KeywordSet::FIRST_STRIKE,
        KeywordSet::TRAMPLE,
    ]
    .into_iter()
    .enumerate()
    {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::URZA_S_AVENGER])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let avenger = on_battlefield(&e, p0, index::URZA_S_AVENGER).unwrap();
        for n in 1..=2 {
            activate(&mut e, p0, index::URZA_S_AVENGER, 0);
            assert!(matches!(e.pending(), Pending::Priority { .. }));
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseCastMode { .. })
            });
            let hash = e.state().snapshot_hash();
            assert!(e.apply(p0, PlayerAction::ChooseMode(4)).is_err());
            assert_eq!(e.state().snapshot_hash(), hash);
            e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(pt(&e, avenger), (4 - n, 4 - n));
            assert_eq!(keywords(&e, avenger), keyword);
        }
        reach_their_main_phase(&mut e, PlayerId::new(1));
        assert_eq!(pt(&e, avenger), (4, 4));
        assert_eq!(keywords(&e, avenger), KeywordSet::EMPTY);
    }
}
