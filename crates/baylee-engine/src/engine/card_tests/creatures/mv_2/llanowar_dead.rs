//! `cards/creatures/mv_2/llanowar_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Llanowar Dead — `{B}{G}` Creature — Zombie Elf, 2/2, whose entire printed
/// text is one line: "`{T}`: Add `{B}`."
///
/// Both halves are played, because neither shows the other. The cast proves the
/// two-colour cost is really paid off a Swamp and a Forest — the pool is empty
/// and the Swamp is tapped afterwards — and the mana ability proves the creature
/// adds *black* rather than the green it costs, as an ordinary `(source, index)`
/// entry in `LegalActions::abilities` and with nothing on the stack (CR 605.1,
/// 605.3b). It is activated in p0's **next** own turn, so that whether this
/// engine reads a creature's summoning sickness as reaching its mana ability is
/// not what the test decides; the 2/2 body is read off the board in between.
#[test]
fn llanowar_dead_costs_black_and_green_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), forest()])
        .hand(0, &[llanowar_dead()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // One Swamp and one Forest is exactly {B}{G}, so the cast is paid by the
    // two lands the test named and by nothing else.
    cast_from_hand(&mut engine, p0, llanowar_dead());
    pass_until(&mut engine, stack_is_empty);

    let dead = on_battlefield(&engine, p0, llanowar_dead()).expect("the {B}{G} spell resolved");
    assert_eq!(pt(&engine, dead), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{B}}{{G}} was paid, so nothing is left floating"
    );
    let land = on_battlefield(&engine, p0, swamp()).expect("the Swamp is still out");
    assert!(
        is_tapped(&engine, land),
        "the {{B}} half of the cost came off the Swamp"
    );

    // A turn cycle, so the {T} below is unambiguously payable.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 takes another turn and holds priority in its own main phase"
    );

    let dead = on_battlefield(&engine, p0, llanowar_dead()).expect("still on the battlefield");
    assert!(!is_tapped(&engine, dead), "nothing has tapped it yet");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a main phase begins with an empty pool"
    );

    // Ability 0 is the printed "{T}: Add {B}." Its whole price is its own tap,
    // so nothing is tapped or floated first.
    activate(&mut engine, p0, llanowar_dead(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "{{T}}: Add {{B}}");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Zombie Elf adds black, not the green it costs to cast"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, dead), "the creature paid its own {{T}}");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );
}
