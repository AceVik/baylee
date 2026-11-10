//! Oracle checklist: sacrifice this creature is the activation cost; draw exactly one on resolution; source is in its owner's graveyard and no longer on battlefield.
#[allow(clippy::wildcard_imports)]
use super::*;
fn alchemist_s_apprentice() -> CardIndex {
    card_index("de085c51-2555-450e-a479-ddbb3bea9e8c")
}

#[test]
fn apprentice_sacrifice_is_paid_before_its_draw_resolves() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[alchemist_s_apprentice()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let hand_before = library_size(&e, p0);
    activate(&mut e, p0, alchemist_s_apprentice(), 0);
    assert!(
        on_battlefield(&e, p0, alchemist_s_apprentice()).is_none(),
        "sacrifice is paid as the cost"
    );
    assert!(in_graveyard(&e, p0, alchemist_s_apprentice()).is_some());
    assert!(!stack_is_empty(&e), "the draw ability uses the stack");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library_size(&e, p0), hand_before - 1);
    assert!(in_graveyard(&e, p0, alchemist_s_apprentice()).is_some());
}
