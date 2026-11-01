//! Reviewed flowstone sculpture behavior.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn flowstone_sculpture() -> CardIndex {
    card_index("42b01697-8663-4b53-86d5-699063bd0778")
}

#[test]
fn flowstone_sculpture_pays_then_chooses_a_counter_or_permanent_keyword() {
    for (mode, keyword) in [
        (0, KeywordSet::EMPTY),
        (1, KeywordSet::FLYING),
        (2, KeywordSet::FIRST_STRIKE),
        (3, KeywordSet::TRAMPLE),
    ] {
        let mut e = lk::game(SEED, &[flowstone_sculpture()], &[], &[forest()], &[]);
        let source = lk::obj(&e, lk::P0, flowstone_sculpture());
        let discard = in_hand(&e, lk::P0, forest()).unwrap();
        lk::float(&mut e, lk::P0, &[(ManaColor::Colorless, 2)]);
        activate(&mut e, lk::P0, flowstone_sculpture(), 0);
        e.apply(
            lk::P0,
            PlayerAction::ChooseObjects {
                objects: vec![discard],
            },
        )
        .unwrap();
        assert!(in_graveyard(&e, lk::P0, forest()).is_some());
        assert_eq!(lk::pool_total(&e, lk::P0), 0);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        e.apply(lk::P0, PlayerAction::ChooseMode(mode)).unwrap();
        lk::settle(&mut e);
        assert_eq!(pt(&e, source), if mode == 0 { (5, 5) } else { (4, 4) });
        assert_eq!(keywords(&e, source), keyword);
        reach_their_main_phase(&mut e, lk::P1);
        assert_eq!(pt(&e, source), if mode == 0 { (5, 5) } else { (4, 4) });
        assert_eq!(keywords(&e, source), keyword);
        let mut empty = lk::game(SEED, &[flowstone_sculpture()], &[], &[], &[]);
        lk::float(&mut empty, lk::P0, &[(ManaColor::Colorless, 2)]);
        assert!(!lk::offered(&empty, flowstone_sculpture(), lk::P0, 0));
    }
}
