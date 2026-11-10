//! `cards/lands/dual/savannah.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Krosan Verge prints `This land enters tapped`, `{T}: Add {C}`, and `{2}, {T}, Sacrifice this
/// land: Search your library for a Forest card and a Plains card, put them onto the battlefield tapped, then shuffle.`
/// The card is marked `Coverage::Implemented`.
/// Backed by a library of dual lands possessing both Forest and Plains subtypes and floating two mana,
/// activating ability index 1 sacrifices Krosan Verge and conducts two successive searches, placing both
/// found lands onto the battlefield tapped.
#[test]
fn krosan_verge_searches_forest_and_plains_onto_battlefield_tapped() {
    let p0 = PlayerId::new(0);
    let savannah_id = card_index("703243f0-8cb3-420f-958f-5fd4bde30293");
    let mut engine = Duel::new(SEED, savannah_id)
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[krosan_verge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let verge = play_land(&mut engine, p0, krosan_verge());
    assert!(entered_tapped(&engine, verge));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, verge));

    tap_all_mana_but(&mut engine, p0, Some(krosan_verge()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2
    );

    activate(&mut engine, p0, krosan_verge(), 1);
    assert!(in_graveyard(&engine, p0, krosan_verge()).is_some());

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
        panic!("expected first search");
    };
    let first = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first],
            },
        )
        .unwrap();

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
        panic!("expected second search");
    };
    let second = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(first).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(second).unwrap().zone,
        Zone::Battlefield
    );
    assert!(is_tapped(&engine, first));
    assert!(is_tapped(&engine, second));
}
