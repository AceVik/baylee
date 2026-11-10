//! `cards/creatures/artifacts/mv_4/rusted_slasher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn rusted_slasher() -> CardIndex {
    card_index("098cc111-493b-470b-929c-67deb981240c")
}

/// Rusted Slasher — {4} 4/1: "Sacrifice an artifact: Regenerate this
/// creature." A Lightning Bolt aimed at it is answered by sacrificing a Sol
/// Ring: the Slasher survives, tapped and with the damage removed. (The
/// question offers the Slasher itself too, being an artifact; the Ring is
/// the one chosen.)
#[test]
fn rusted_slasher_sacrifices_an_artifact_to_regenerate() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rusted_slasher(), sol_ring(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let slasher = on_battlefield(&engine, p0, rusted_slasher()).expect("seated");
    let ring = on_battlefield(&engine, p0, sol_ring()).expect("seated");
    assert_eq!(pt(&engine, slasher), (4, 1));

    let mountains = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| mountains.contains(&id));
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![slasher],
                players: vec![],
            },
        )
        .expect("the Slasher is a legal target");

    activate(&mut engine, p0, rusted_slasher(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("which artifact: {:?}", engine.pending());
    };
    assert!(options.contains(&ring));
    assert!(options.contains(&slasher), "it is an artifact itself");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the Ring is on the menu");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, sol_ring()).is_some(),
        "sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_some(),
        "the Bolt resolved"
    );
    assert!(
        on_battlefield(&engine, p0, rusted_slasher()).is_some(),
        "and the Slasher regenerated"
    );
    assert!(is_tapped(&engine, slasher), "regenerating taps it");
    assert_eq!(engine.state().object(slasher).expect("slasher").damage, 0);
}

/// The control: the same Bolt with no answer kills the 4/1.
#[test]
fn rusted_slasher_dies_to_a_bolt_without_the_sacrifice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rusted_slasher(), sol_ring(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let slasher = on_battlefield(&engine, p0, rusted_slasher()).expect("seated");
    bolt_to_death(&mut engine, p0, slasher);
    assert!(in_graveyard(&engine, p0, rusted_slasher()).is_some());
}
