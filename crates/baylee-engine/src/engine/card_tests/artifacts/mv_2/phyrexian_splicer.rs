//! Each keyword transfer observes both target restrictions and cleanup.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;
fn phyrexian_splicer() -> CardIndex {
    card_index("4433a5fd-2daa-440a-b9cc-4ad028358cbc")
}

#[test]
fn splicer_moves_each_chosen_keyword_only_until_cleanup() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (ability, donor, keyword) in [
        (0, index::AIR_ELEMENTAL, KeywordSet::FLYING),
        (1, index::WHITE_KNIGHT, KeywordSet::FIRST_STRIKE),
        (2, index::WAR_MAMMOTH, KeywordSet::TRAMPLE),
        (3, index::DAUTHI_HORROR, KeywordSet::SHADOW),
    ] {
        let mut e = lk::game(
            SEED,
            &[phyrexian_splicer(), donor],
            &[index::GRIZZLY_BEARS],
            &[],
            &[],
        );
        let donor = on_battlefield(&e, p0, donor).unwrap();
        let recipient = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
        lk::float(&mut e, p0, &[(ManaColor::Colorless, 2)]);
        activate(&mut e, p0, phyrexian_splicer(), ability);
        assert!(
            e.apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![recipient],
                    players: vec![]
                }
            )
            .is_err(),
            "the donor must already have the chosen ability"
        );
        lk::aim(&mut e, &[donor], &[]);
        assert!(
            e.apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![donor],
                    players: vec![]
                }
            )
            .is_err(),
            "another target creature"
        );
        lk::aim(&mut e, &[recipient], &[]);
        lk::settle(&mut e);
        assert!(
            !e.state()
                .object(donor)
                .unwrap()
                .characteristics()
                .keywords
                .contains(keyword)
        );
        assert!(
            e.state()
                .object(recipient)
                .unwrap()
                .characteristics()
                .keywords
                .contains(keyword)
        );
        reach_their_main_phase(&mut e, p1);
        assert!(
            e.state()
                .object(donor)
                .unwrap()
                .characteristics()
                .keywords
                .contains(keyword)
        );
        assert!(
            !e.state()
                .object(recipient)
                .unwrap()
                .characteristics()
                .keywords
                .contains(keyword)
        );
    }
}
