//! `cards/creatures/artifacts/mv_4/sanctum_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;

fn sanctum_gargoyle() -> CardIndex {
    card_index("6ee9802a-7a44-49c7-93d9-f75c4d42e71e")
}

/// The Gargoyle cast with a Sol Ring in the graveyard, stopped at "you may".
fn cast_with_a_ring_in_the_graveyard() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, sol_ring())
        .battlefield(0, &[plains(), forest(), forest(), forest()])
        .hand(0, &[sanctum_gargoyle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    let ring = in_graveyard(&engine, p0, sol_ring()).expect("a Sol Ring is in the graveyard");
    cast_from_hand(&mut engine, p0, sanctum_gargoyle());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for this");
    };
    assert_eq!(
        options,
        vec![ring],
        "the one artifact card in the graveyard"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .expect("the Ring is a legal target");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
}

/// Sanctum Gargoyle — {3}{W} 2/3 with flying: "When this creature enters,
/// you may return target artifact card from your graveyard to your hand."
/// Yes returns the Sol Ring.
#[test]
fn sanctum_gargoyle_returns_an_artifact_card_when_you_say_yes() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_with_a_ring_in_the_graveyard();
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the may is answered");
    pass_until(&mut engine, stack_is_empty);
    let gargoyle = on_battlefield(&engine, p0, sanctum_gargoyle()).expect("it resolved");
    assert_eq!(pt(&engine, gargoyle), (2, 3));
    assert!(keywords(&engine, gargoyle).contains(KeywordSet::FLYING));
    assert!(in_hand(&engine, p0, sol_ring()).is_some(), "back in hand");
    assert!(in_graveyard(&engine, p0, sol_ring()).is_none());
}

/// "May": no leaves the Sol Ring in the graveyard.
#[test]
fn sanctum_gargoyle_leaves_the_graveyard_alone_when_you_say_no() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_with_a_ring_in_the_graveyard();
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("the may is answered");
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, sol_ring()).is_some());
    assert!(in_hand(&engine, p0, sol_ring()).is_none());
}
