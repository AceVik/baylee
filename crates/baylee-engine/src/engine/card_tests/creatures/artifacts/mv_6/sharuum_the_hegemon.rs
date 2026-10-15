//! `cards/creatures/artifacts/mv_6/sharuum_the_hegemon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sharuum_the_hegemon() -> CardIndex {
    card_index("037e7fc9-3aa6-484c-a2c8-43009e45f1d8")
}

fn sharuum_graveyard() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sharuum_the_hegemon(), sol_ring(), gray_ogre()])
        .hand(1, &[sol_ring()])
        .battlefield(
            0,
            &[plains(), island(), swamp(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hand_to_graveyard(&mut engine, p0, sol_ring());
    hand_to_graveyard(&mut engine, p0, gray_ogre());
    hand_to_graveyard(&mut engine, PlayerId::new(1), sol_ring());
    engine
}

/// Sharuum the Hegemon — flying; "When Sharuum enters, you may return target
/// artifact card from your graveyard to the battlefield." Only my artifact
/// card is offered, not my creature card nor their artifact.
#[test]
fn sharuum_the_hegemon_returns_an_artifact_card_from_my_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = sharuum_graveyard();
    let mine = in_graveyard(&engine, p0, sol_ring()).expect("my ring");
    let ogre = in_graveyard(&engine, p0, gray_ogre()).expect("my ogre");
    let theirs = in_graveyard(&engine, p1, sol_ring()).expect("their ring");

    cast_from_hand(&mut engine, p0, sharuum_the_hegemon());
    let options = pass_until_targets(&mut engine, p0);
    assert_eq!(
        options,
        vec![mine],
        "{ogre:?} and {theirs:?} are not artifacts of mine"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, sol_ring()).is_some(),
        "returned to the battlefield"
    );
    assert!(in_graveyard(&engine, p0, sol_ring()).is_none());
    let sharuum = on_battlefield(&engine, p0, sharuum_the_hegemon()).expect("resolved");
    assert_eq!(pt(&engine, sharuum), (5, 5));
    assert!(keywords(&engine, sharuum).contains(KeywordSet::FLYING));
}

/// It is a "may": saying no leaves the card where it lies.
#[test]
fn sharuum_the_hegemon_may_decline_to_return_it() {
    let p0 = PlayerId::new(0);
    let mut engine = sharuum_graveyard();
    let mine = in_graveyard(&engine, p0, sol_ring()).expect("my ring");
    cast_from_hand(&mut engine, p0, sharuum_the_hegemon());
    pass_until_targets(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("a legal target");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).expect("may");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, sol_ring()).is_none());
    assert!(in_graveyard(&engine, p0, sol_ring()).is_some());
}
