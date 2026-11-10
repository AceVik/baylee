//! `cards/instants/mv_3/eerie_interlude.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Exile **any number of** target creatures you control", with none to exile.
///
/// The spell half of what the Apparition's trigger settles above, and it
/// arrives by a different door: `min` is 0 and `max` is 255, so the cast
/// wizard's `max == 0` branch never sees this spell and the *board* is what
/// leaves it with nothing to choose. One legal answer is not a choice, so the
/// spell is cast with no targets rather than the caster being held at
/// `ChooseTargets { options: [], min: 0, max: 255 }` — a stop that can only
/// be answered one way.
#[test]
fn a_spell_that_may_target_any_number_is_not_asked_with_nothing_to_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(202, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[eerie_interlude()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a main phase hands priority back");
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = in_hand(&engine, p0, eerie_interlude()).expect("the Interlude is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("three Plains pay {2}{W}");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no creatures to exile, so no question: {:?}",
        engine.pending()
    );
    assert!(!stack_is_empty(&engine), "and the spell is on the stack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });
}
