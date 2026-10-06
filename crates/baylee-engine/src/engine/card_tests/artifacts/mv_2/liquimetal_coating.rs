//! `cards/artifacts/mv_2/liquimetal_coating.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Liquimetal Coating: "{T}: **Target** permanent becomes an artifact in
/// addition to its other types until end of turn."
///
/// The same mistake Karn's `+1` made, found by the lint written for it and
/// worse: the filter reused here was `Filter::Any`, so one tap turned *every
/// permanent in the game* into an artifact — both battlefields, lands
/// included. That is the shape the owner asked about from the other side
/// ("an effect that should only appear for my field"), so the bystander here
/// is the **opponent's** land: an effect pointed at one permanent may not
/// cross the table.
#[test]
fn liquimetal_coating_plates_its_target_and_nobody_elses_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[liquimetal_coating(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let coating = on_battlefield(&engine, p0, liquimetal_coating()).expect("the coating is out");
    let mine = on_battlefield(&engine, p0, forest()).expect("my forest");
    let theirs = on_battlefield(&engine, p1, forest()).expect("their forest");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: coating,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target permanent\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(mine)
            .is_some_and(|o| o.characteristics().types.intersects(TypeSet::ARTIFACT))
    });

    // The target is plated, and the land across the table is a plain Forest.
    assert!(
        !engine
            .state()
            .object(theirs)
            .expect("their forest is still there")
            .characteristics()
            .types
            .intersects(TypeSet::ARTIFACT),
        "the ability reached across the table and plated the opponent's land"
    );
}
