//! `cards/instants/mv_2/heroic_intervention.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The game the owner played, and the first rules hole a player walked into.
///
/// An opponent cast Banishing Stroke at Katara, the Fearless; the owner
/// answered with Heroic Intervention; Katara went to the bottom of the
/// library anyway. All three cards read correctly — the printed removal, the
/// printed protection and Katara's own replacement rule — and the engine
/// simply never asked CR 608.2b as the removal began to resolve.
///
/// The rule itself is tested in `rules`, with a synthetic-sized board and
/// both directions of it. This one is the scenario: the three printings that
/// were in front of a person, played in the order they were played in, so
/// that what he reported is what goes red if it comes back.
#[test]
fn heroic_intervention_answers_banishing_stroke() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(76, forest())
        .battlefield(0, &[forest(), forest(), katara_the_fearless()])
        .hand(0, &[heroic_intervention()])
        .battlefield(
            1,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(1, &[banishing_stroke()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let katara = on_battlefield(&engine, p0, katara_the_fearless()).expect("Katara is out");
    let library_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p1, banishing_stroke());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the stroke's aim, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![katara],
        "\"target artifact, creature, or enchantment\": Katara is the only \
         one on the table"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![katara],
            },
        )
        .unwrap();

    // The answer, cast in response — which is the half that was broken. A
    // creature that *already* had hexproof could not have been chosen at
    // all, and that path was tested and worked.
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_from_hand(&mut engine, p0, heroic_intervention());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(katara).map(|o| o.zone),
        Some(Zone::Battlefield),
        "Katara is on the bottom of the library with hexproof — the owner's \
         bug, exactly as he reported it"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "and nothing was put under the library either: the count is the \
         other half of the same claim, because a creature that left the \
         battlefield and a creature that arrived in the library are two \
         readings a single zone check cannot tell apart"
    );
    assert!(
        in_graveyard(&engine, p1, banishing_stroke()).is_some(),
        "the spell that did not resolve went to its owner's graveyard \
         (CR 608.2b)"
    );
}
