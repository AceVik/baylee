//! `cards/creatures/mv_5/screaming_seahawk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Screaming Seahawk (`02b5ea0b-c0a3-4213-9064-e688190fca5e`) — {4}{U} Creature — Bird 2/2:
/// "Flying
/// When this creature enters, you may search your library for a card named Screaming Seahawk, reveal it, put it into your hand, then shuffle."
#[test]
fn screaming_seahawk_searches_library_for_another_copy_on_etb() {
    let seahawk_card = card_index("02b5ea0b-c0a3-4213-9064-e688190fca5e");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, seahawk_card)
        .hand(0, &[seahawk_card])
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, seahawk_card);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected SearchLibrary prompt");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let seahawk = on_battlefield(&engine, p0, seahawk_card).unwrap();
    assert!(keywords(&engine, seahawk).contains(KeywordSet::FLYING));
    assert!(in_hand(&engine, p0, seahawk_card).is_some());
}
