//! Oracle: This creature gets +2/+1 as long as you control no untapped lands.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;
use baylee_core::generated::index;

fn subject() -> CardIndex {
    card_index("42784364-719a-49ca-b515-64146a8f82e8")
}

// Checklist: zero lands qualifies; any own untapped land suppresses +2/+1;
// opponent untapped land irrelevant; tapping last own land enables exact bonus;
// next own untap removes it. Other creatures never receive the bonus.
#[test]
fn no_untapped_lands_includes_zero_and_tracks_real_taps_and_untap() {
    let p0 = PlayerId::new(0);
    for own_land in [false, true] {
        let board = if own_land {
            vec![subject(), island(), index::GRAY_OGRE]
        } else {
            vec![subject(), index::GRAY_OGRE]
        };
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(1, &[forest()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let source = on_battlefield(&e, p0, subject()).unwrap();
        assert_eq!(pt(&e, source), if own_land { (2, 1) } else { (4, 2) });
        if own_land {
            tap_all_mana(&mut e, p0);
            assert_eq!(pt(&e, source), (4, 2));
            let turn = e.state().turn.number;
            pass_until(&mut e, |e| {
                e.state().turn.number > turn
                    && e.state().turn.active == p0
                    && matches!(e.state().turn.phase, Phase::FirstMain)
            });
            assert!(!is_tapped(&e, on_battlefield(&e, p0, island()).unwrap()));
            assert_eq!(pt(&e, source), (2, 1));
        }
        assert_eq!(
            pt(&e, on_battlefield(&e, p0, index::GRAY_OGRE).unwrap()),
            (2, 2)
        );
    }
}
