//! `cards/instants/mv_2/mana_drain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "74d3277a-38e5-4732-afed-084a56148f20"

/// Mana Drain is `{U}{U}` and prints two sentences: "Counter target spell",
/// and "At the beginning of your next main phase, add an amount of {C} equal
/// to that spell's mana value."
///
/// The victim is Tidings — `{3}{U}{U}`, five mana and no target — so the
/// delayed mana has to be **five** colourless. That is the whole second
/// sentence: the amount is read off the spell that was countered, which a
/// one-mana victim could not tell from the Drain's own cost or from a card
/// that always adds one. Five Islands pay Tidings' price to the last mana, so
/// the five that arrive later are not leftovers, and reading them at p1's
/// *next* first main phase is the first sentence too: mana made during p0's
/// main phase would have emptied with it (CR 500.5) and the walk would never
/// see it at all.
#[test]
fn mana_drain_counters_a_five_mana_spell_and_pays_five_colorless_a_turn_later() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[tidings()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[mana_drain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a first main phase of its own"
    );

    let library_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, tidings());
    let victim = on_stack(&engine, tidings()).expect("Tidings went on the stack");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five Islands are exactly {{3}}{{U}}{{U}}, so nothing is left floating"
    );

    // p0 passes, and the opponent gets the window an instant needs.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
            && on_stack(e, tidings()).is_some()
    });
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "the two Islands are {{U}}{{U}}, and the Drain asks for no more"
    );
    cast_with_floating(&mut engine, p1, mana_drain());
    let menu = aim_at(&mut engine, p1, victim);
    assert!(
        menu.contains(&victim),
        "the only spell on the stack is the one being countered: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, tidings()).is_some(),
        "\"Counter target spell\": the sorcery is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, mana_drain()).is_some(),
        "and the counterspell itself resolved rather than fizzling"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "Tidings draws four cards only if it resolves — a countered spell does nothing"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the Drain's price was all p1 had, so the pool starts this walk blank"
    );

    // The second sentence, at the only moment it can be read: p1's own next
    // first main phase, with the mana already made.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().players[1].mana_pool.total() > 0
    });

    let pool = &engine.state().players[1].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        5,
        "the countered spell's mana value is five, so five {{C}} — not one, \
         and not the two the Drain itself cost"
    );
    assert_eq!(
        pool.total(),
        5,
        "and nothing else is floating: the two Islands paid for the Drain"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "\"add an amount of {{C}}\": the mana is colourless, and no blue land \
         on this board made any of it"
    );
}
