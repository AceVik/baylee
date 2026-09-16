//! Sorceries, the door `cards/sorceries/` puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A creature cast after a board-wide debuff resolved is not on its list.
///
/// CR 611.2c: a continuous effect created by a resolving spell or ability
/// that modifies characteristics affects the objects it found when it began,
/// and the set does not change afterwards. Toxic Deluge registered a *live
/// filter* instead — "every creature", asked again on every projection —
/// so a creature cast one priority later entered as a 0/0 and was swept up
/// by the next state-based check, killed by a spell that had already
/// finished resolving.
///
/// Both halves are asserted, because either alone can be passed by a
/// mistake: the opponent's Llanowar Elves was there when the Deluge
/// resolved and dies, and the one cast afterwards stands there at 1/1.
#[test]
fn a_creature_cast_after_a_mass_debuff_is_not_shrunk_by_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), forest()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[toxic_deluge(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Only the swamps: the forest is kept back for the creature, so the
    // test does not depend on which land the payment happens to spend.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        if engine
            .state()
            .object(source)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == swamp()))
        {
            engine
                .apply(p0, PlayerAction::ActivateManaAbility { source })
                .unwrap();
        }
    }
    let deluge = in_hand(&engine, p0, toxic_deluge()).expect("the Deluge is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: deluge })
        .unwrap();
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("expected the X choice, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_none(),
        "the 1/1 that was there when the Deluge resolved took -1/-1 and died",
    );

    cast_from_hand(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, quiet_creature()).is_some()
    });
    let mine = on_battlefield(&engine, p0, quiet_creature())
        .expect("a creature cast after the Deluge survives its arrival");
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the Deluge had already resolved, so it is not one of its creatures",
    );
}
