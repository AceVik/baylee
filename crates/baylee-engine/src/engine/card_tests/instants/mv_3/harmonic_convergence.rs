#[allow(clippy::wildcard_imports)]
use super::*;

fn harmonic_convergence() -> CardIndex {
    card_index("400adeec-11b4-4a9c-9466-dcab524d87ea")
}

#[test]
fn harmonic_convergence_returns_all_enchantments_to_owners_hands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[harmonic_convergence()])
        .battlefield(0, &[wizard_class(), forest(), forest(), forest()])
        .battlefield(1, &[wizard_class()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, harmonic_convergence());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, wizard_class()).is_none());
    assert!(on_battlefield(&engine, p1, wizard_class()).is_none());
    assert!(in_hand(&engine, p0, wizard_class()).is_some());
    assert!(in_hand(&engine, p1, wizard_class()).is_some());
}
