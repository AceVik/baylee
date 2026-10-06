//! `cards/creatures/mv_4/skyshroud_troopers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skyshroud Troopers — {3}{G} — Creature — Elf Druid Warrior, 3/3, and its
/// whole printed text is one line: "{T}: Add {G}".
///
/// Both halves are the engine's answer rather than the card's, so both are
/// played. Four Forests are exactly {3}{G} and are spent down to an empty
/// pool, which is what makes the body the cast left behind a reading rather
/// than a label: three and a green is a real price, and (3, 3) is the body
/// the card prints. The mana line is a *printed* mana ability, so it is an
/// ordinary `(source, index)` entry in `LegalActions::abilities` and never the
/// CR 305.6 shortcut a Forest uses; its whole price is its own {T}, so it is
/// offered on an empty pool (CR 601.2h reads the pool, not the untapped
/// lands), and leaving every Forest standing is what makes "one green and
/// nothing else" exact — nothing on the board but the creature could have made
/// it. A creature's {T} is unpayable while it is sick (CR 302.6), so the turn
/// is walked once before the tap symbol is worth anything.
#[test]
fn skyshroud_troopers_lands_as_a_three_three_and_taps_for_one_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[skyshroud_troopers()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Forests into the pool: {3}{G} is the whole cost, and CR 500.5
    // keeps a pool across a cast while the scenario stays in one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, and the creature is still in hand"
    );
    cast_with_floating(&mut engine, p0, skyshroud_troopers());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let troopers =
        on_battlefield(&engine, p0, skyshroud_troopers()).expect("the Troopers resolved");
    assert_eq!(
        pt(&engine, troopers),
        (3, 3),
        "the body the card prints, for the {{3}}{{G}} the cast really paid"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the four mana were the whole cost"
    );
    let kinds = types(&engine, troopers);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "and what arrived is a creature: {kinds:?}"
    );

    // CR 302.6: a creature that has just entered has no payable {T}, so the
    // untap step its controller's next turn brings is what turns the printed
    // line into an ability at all.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, troopers),
        "the untap step stood the creature back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5), so a green \
         read below has no other source on this board"
    );

    // The whole price is the tap symbol, so `can_afford` is satisfied by an
    // empty pool — and the ability is a printed one, which is why it carries
    // an index to name instead of arriving as the CR 305.6 shortcut.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(troopers, 0)),
        "an untapped creature is a paid {{T}}, so \"{{T}}: Add {{G}}\" is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, skyshroud_troopers(), 0);

    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{G}}` is fixed, so there is nothing to name on the way (CR 605.1), \
         got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "\"{{T}}: Add {{G}}\" — one green, off one tap"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        is_tapped(&engine, troopers),
        "the creature paid its own {{T}}"
    );
    assert!(
        lands_of(&engine, p0)
            .iter()
            .all(|id| !is_tapped(&engine, *id)),
        "and no land on the board moved, so the green has no other source on it"
    );

    // The tap is spent, so the line is no longer one the seat may take — read
    // off the offer, which is where an unpayable cost goes.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(troopers, 0)),
        "a tapped creature has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
