//! `cards/lands/utility/wasteland.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wasteland destroys a nonbasic land and cannot be pointed at a basic.
///
/// The refusal is the half worth a test. `Filter::NONBASIC_LAND` stands on
/// both sides of this ability — once as the target spec the offer enumerates
/// from and once inside the effect — so a filter that widened to every land
/// would read as correct from the card and hand a player a Forest to blow
/// up. What proves it is the *menu*, not the outcome: the engine has already
/// decided by the time the options arrive.
#[test]
fn wasteland_destroys_a_nonbasic_land_and_is_not_offered_a_basic() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4_700, forest())
        .battlefield(0, &[wasteland(), forest()])
        .battlefield(1, &[academy_ruins()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let waste = on_battlefield(&engine, p0, wasteland()).expect("Wasteland is on the table");
    let ruins = on_battlefield(&engine, p1, academy_ruins()).expect("their Ruins are too");
    let basic = on_battlefield(&engine, p0, forest()).expect("and my own Forest");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: waste,
                ability_index: 1,
            },
        )
        .expect("the ability is offered on an untapped Wasteland");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&ruins),
        "a nonbasic land is what it destroys"
    );
    assert!(
        !options.contains(&basic),
        "a basic Forest was offered to Wasteland: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ruins],
            },
        )
        .expect("a land the menu named is a legal answer");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, academy_ruins()).is_none(),
        "the targeted nonbasic land survived"
    );
    assert!(
        on_battlefield(&engine, p0, wasteland()).is_none(),
        "Wasteland sacrifices itself as part of the cost"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "and the Forest it was never offered is still there"
    );
}
