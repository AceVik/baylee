//! `cards/creatures/mv_2/wirewood_elf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wirewood Elf is `{1}{G}` for a 1/2 Elf Druid whose entire printed
/// text line is `{T}: Add {G}` — both halves need the game, neither
/// of them is in the card def. The turn change before it is the cost
/// of the second: a creature that arrived this turn has summoning
/// sickness (CR 302.6), so its mana ability is not even offered on the
/// turn it arrives, although it stands untapped — and only after p0's
/// next untap step is the `{T}` something it may pay. Before that,
/// **nothing** is tapped: the whole cost is its own tap symbol, the
/// empty pool makes "exactly one green mana" a statement about the
/// Elf and not about the two Forests that remain untapped next to it.
#[test]
fn wirewood_elf_arrives_as_a_one_two_and_taps_for_one_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[wirewood_elf()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{G} off the two Forests, which is the only thing on the board that
    // can pay for it.
    cast_from_hand(&mut engine, p0, wirewood_elf());
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, p0, wirewood_elf()).expect("the Elf resolved");
    assert_eq!(pt(&engine, elf), (1, 2), "the printed 1/2 body");
    assert!(
        types(&engine, elf).contains(TypeSet::CREATURE),
        "and it is the creature the card prints: {:?}",
        types(&engine, elf)
    );
    assert!(!is_tapped(&engine, elf), "nothing has tapped it");

    // The control on the turn change below: the same untapped creature, the
    // same empty pool, and the line is still not there, because a `{T}` may
    // not be paid by a permanent that has not been under its controller's
    // control since their most recent turn began (CR 302.6).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(elf, 0)),
        "untapped and yet not on offer: the Elf arrived this turn \
         (CR 302.6): {:?}",
        legal.abilities
    );

    // Through the opponent's turn and back.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 takes another turn and reaches its own main"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a pool empties when a step or phase ends (CR 500.5), so anything in \
         it from here on has to be accounted for"
    );
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 2, "both Forests are still standing");
    assert!(
        forests.iter().all(|id| !is_tapped(&engine, *id)),
        "and untapped, so the green below has no other source on this board"
    );

    // The whole price is its own `{T}`, so the line is offered with nothing
    // floating — `can_afford` reads the *pool*, and an empty pool pays a tap.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(elf, 0)),
        "a printed mana ability has an index to name, so it is an ordinary \
         `(source, index)` entry in `abilities`: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, wirewood_elf(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the one color the card names, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, and nothing beside it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, elf), "the Elf paid its own {{T}}");
    assert!(
        forests.iter().all(|id| !is_tapped(&engine, *id)),
        "and neither Forest moved, so the green mana is the Elf's own"
    );

    // The other half of "its whole price is its own tap": the tap is spent,
    // so the line is no longer one the seat may take.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(elf, 0)),
        "a tapped Elf has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
