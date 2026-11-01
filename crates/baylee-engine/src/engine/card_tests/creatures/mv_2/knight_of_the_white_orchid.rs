//! `cards/creatures/mv_2/knight_of_the_white_orchid.rs`, played.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("7d0efc64-151f-45b4-8c0b-c32609f9d862")
}

#[test]
fn knight_conditionally_offers_a_plains_onto_the_battlefield_untapped() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[subject()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, |g| matches!(g.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, |g| {
        matches!(
            g.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    let fetched_plains = options[0];
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![fetched_plains],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let knight = on_battlefield(&e, p0, subject()).unwrap();
    assert!(keywords(&e, knight).contains(KeywordSet::FIRST_STRIKE));
    let lands = all_on_battlefield(&e, p0, plains());
    assert_eq!(lands.len(), 3);
    assert!(!is_tapped(&e, fetched_plains));
}
