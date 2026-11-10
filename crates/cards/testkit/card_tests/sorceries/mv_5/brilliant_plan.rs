//! `cards/sorceries/mv_5/brilliant_plan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Brilliant Plan is a `{4}{U}` sorcery printing one word: "Draw three cards."
/// The whole card is therefore a count only a game can make, so the scenario
/// watches the spell wait on the stack, the five Islands pay the full `{4}{U}`
/// out of a pool that was empty beforehand, and then the three cards land in
/// hand while the library shrinks by exactly those three and the card itself
/// is in its owner's graveyard. Nothing else on this board draws, discards or
/// shuffles anything, so the three and the emptied pool are each other's
/// control — and the hand is read as *net* growth, because a test that only
/// counted the library would pass for a spell that exiled its own top three.
#[test]
fn brilliant_plan_draws_three_cards_for_the_five_mana_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[brilliant_plan()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats: the {{4}}{{U}} has to come out of the five Islands"
    );

    cast_from_hand(&mut engine, p0, brilliant_plan());
    assert!(
        on_stack(&engine, brilliant_plan()).is_some(),
        "a sorcery waits on the stack rather than resolving as it is announced"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five Islands and five blue: the whole {{4}}{{U}} is spent on the cast"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 3,
        "\"Draw three cards\": three cards left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "and the hand holds three more than the spell left behind — three \
         drawn, one resolved away. A library that merely emptied would not \
         move this number"
    );
    assert!(
        in_graveyard(&engine, p0, brilliant_plan()).is_some(),
        "the sorcery itself went to its owner's graveyard once it resolved"
    );
}
