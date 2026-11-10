//! `cards/creatures/artifacts/mv_2/myr_retriever.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Myr Retriever ({2}, 1/1): "When this creature dies, return **another**
/// target artifact card from your graveyard to your hand."
///
/// The word the whole card turns on is `another`, and it is load-bearing in a
/// way no other dies trigger's is: by the time the ability is put on the
/// stack the Myr is itself an artifact card lying in that same graveyard
/// (CR 603.6c, CR 400.7), so a trigger that read "target artifact card" would
/// offer the Myr its own corpse and return it to hand every time — a
/// two-mana artifact that recurs itself forever, which is not the card.
///
/// Both halves are struck, and on the ids the cards have **after** the move:
/// comparing against the Myr's battlefield id would pass however wrong the
/// filter was, because an object changes id when it changes zone.
///
/// The library is filled with an artifact rather than a basic land, which is
/// what gives `seed_graveyard` an artifact card to put there — the Myr needs
/// something legal to point at or the trigger would be removed from the stack
/// for having no legal target, and the interesting assertion would never be
/// reached.
#[test]
fn a_dying_myr_returns_another_artifact_card_and_never_its_own_corpse() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, quiet_artifact())
        .battlefield(0, &[myr_retriever()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "an artifact card is waiting in the graveyard"
    );

    reach_their_main_phase(&mut engine, p1);
    let myr = on_battlefield(&engine, p0, myr_retriever()).expect("the Myr is out");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![myr] })
        .expect("their removal may point at an ordinary creature");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the loop above waited for exactly this")
    };
    let corpse = in_graveyard(&engine, p0, myr_retriever()).expect("the Myr died");
    let other = in_graveyard(&engine, p0, quiet_artifact()).expect("and it is not alone");
    assert!(
        !options.contains(&corpse),
        "`another` keeps the Myr from targeting itself in the graveyard it is \
         now lying in: {options:?}"
    );
    assert!(
        options.contains(&other),
        "and the other artifact card is offered: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![other],
            },
        )
        .expect("the one legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_some(),
        "the artifact card came back to hand"
    );
    assert!(
        in_graveyard(&engine, p0, myr_retriever()).is_some(),
        "and the Myr stayed where it fell"
    );
}
