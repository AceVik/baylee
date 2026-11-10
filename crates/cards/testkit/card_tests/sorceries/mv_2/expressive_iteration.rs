//! `cards/sorceries/mv_2/expressive_iteration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Look at the top three cards of your library. Put one of them into your
/// hand, put one of them on the bottom of your library, and exile one of
/// them. You may play the exiled card this turn." — the exiled Elf is cast
/// from exile, paid for like any spell.
#[test]
fn expressive_iteration_keeps_one_bottoms_one_and_lets_the_third_be_cast() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[island(), mountain(), forest()])
        .hand(0, &[expressive_iteration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let [keep, bottom, exiled] = iterate(&mut engine, p0);
    let state = engine.state();
    assert!(state.zones.list(ZoneLocation::Hand(p0)).contains(&keep));
    assert_eq!(
        state.zones.list(ZoneLocation::Library(p0)).first(),
        Some(&bottom),
        "on the bottom"
    );
    assert!(state.zones.list(ZoneLocation::Exile(p0)).contains(&exiled));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&exiled),
        "a permission to play it, not a free cast: nothing is floating yet"
    );
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(legal.castable.contains(&exiled), "with {{G}} floating");
    engine
        .apply(p0, PlayerAction::CastSpell { card: exiled })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}

/// A spell cast from exile under the permission was not cast from a hand,
/// so an exiled Ephemerate has no rebound (CR 702.88a) and goes to the
/// graveyard. `finish_cast` stamped every paid cast "from hand", so this
/// Ephemerate was exiled again to be cast a second time for free.
#[test]
fn a_rebound_spell_cast_from_expressive_iterations_exile_does_not_rebound() {
    let p0 = PlayerId::new(0);
    let ephemerate = card_index("0fd57894-b917-41c8-a394-360d1d31b236");
    let mut engine = Duel::new(SEED, ephemerate)
        .battlefield(0, &[island(), mountain(), plains(), llanowar_elves()])
        .hand(0, &[expressive_iteration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let [_, _, exiled] = iterate(&mut engine, p0);
    let plains = on_battlefield(&engine, p0, plains()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == plains);
    engine
        .apply(p0, PlayerAction::CastSpell { card: exiled })
        .unwrap();
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&exiled),
        "into the graveyard, not back into exile on a rebound"
    );
}
