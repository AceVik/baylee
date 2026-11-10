//! `cards/creatures/artifacts/mv_2/aegis_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn aegis_automaton() -> CardIndex {
    card_index("727f4734-53cc-4279-ba51-dfa26c4ceb1d")
}

/// Aegis Automaton: a 0/3 with "{4}{W}: Return another target creature you
/// control to its owner's hand". The Bears go back to hand; the Automaton
/// itself is not on the menu.
#[test]
fn aegis_automaton_returns_another_creature_you_control_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[],
        &[
            aegis_automaton(),
            grizzly_bears(),
            plains(),
            forest(),
            forest(),
            forest(),
            forest(),
        ],
        &[],
    );
    let aegis = on_battlefield(&engine, p0, aegis_automaton()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, aegis), (0, 3));
    assert!(
        !priority_offer(&engine).abilities.contains(&(aegis, 0)),
        "no mana floating"
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, aegis_automaton(), 0);
    let options = aim_at(&mut engine, p0, bears);
    assert!(options.contains(&bears));
    assert!(!options.contains(&aegis), "\"another\" creature");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(in_hand(&engine, p0, grizzly_bears()).is_some(), "in hand");
    assert!(on_battlefield(&engine, p0, aegis_automaton()).is_some());
}

/// Only creatures you control: the opponent's Bears are not on the menu.
#[test]
fn aegis_automaton_cannot_target_the_opponents_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[
            aegis_automaton(),
            grizzly_bears(),
            plains(),
            forest(),
            forest(),
            forest(),
            forest(),
        ],
        &[wall_of_stone()],
    );
    let wall = on_battlefield(&engine, p1, wall_of_stone()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, aegis_automaton(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target question")
    };
    assert!(!options.contains(&wall));
}
