//! `cards/creatures/mv_2/loyal_warhound.rs`, played.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("cc6a83c7-e645-4a53-9550-be79b42cd851")
}

#[test]
fn loyal_warhound_fetches_a_basic_plains_tapped_when_behind_on_lands() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[subject()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, subject());
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
    let plains_card = options[0];
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![plains_card],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let hound = on_battlefield(&e, p0, subject()).unwrap();
    assert!(keywords(&e, hound).contains(KeywordSet::VIGILANCE));
    assert_eq!(
        e.state().object(plains_card).unwrap().zone,
        Zone::Battlefield
    );
    assert!(is_tapped(&e, plains_card));
}
