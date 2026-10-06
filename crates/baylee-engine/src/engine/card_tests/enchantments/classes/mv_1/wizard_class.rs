//! `cards/enchantments/classes/mv_1/wizard_class.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wizard Class ({U}, Class): `{2}{U}: Level 2`, and "when this Class
/// becomes level 2, draw two cards".
///
/// The level-up is an `ActivatedConditional` — it may only be activated at
/// level 1 — and that is the whole point of the test. The condition is a
/// restriction on *activating* the ability (CR 602.5), spent the moment the
/// activation is allowed to begin; what goes on the stack afterwards is an
/// ordinary ability. The engine's resolver did not know that: its match over
/// the ability being resolved listed `Activated` and four others and left
/// `ActivatedConditional` out, so levelling a Class put an ability on the
/// stack that panicked the process as it resolved. Two implemented cards
/// print one of these — this and Luminarch Ascension — and neither had ever
/// had its ability resolved by a test.
#[test]
fn a_class_can_be_levelled_and_the_level_up_resolves() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(919, forest())
        .battlefield(0, &[wizard_class(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, wizard_class()).expect("the Class is on the table");
    tap_mana_except(&mut engine, p0, class);
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: class,
                ability_index: 1,
            },
        )
        .expect("level 2 is affordable at level 1");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(class)
            .expect("the Class survived its own ability")
            .counters
            .get(CounterKind::Level),
        1,
        "one level counter, so the Class is level 2"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 2,
        "becoming level 2 drew two cards"
    );
}
