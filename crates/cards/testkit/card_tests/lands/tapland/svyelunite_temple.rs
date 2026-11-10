//! `cards/lands/tapland/svyelunite_temple.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Svyelunite Temple is a land with three printed lines: it enters tapped,
/// "{T}: Add {U}", and "{T}, Sacrifice this land: Add {U}{U}".
///
/// It has to be *played* rather than seated on the battlefield, because
/// `starting_battlefield` places a permanent without an entry and a land that
/// prints "enters tapped" would arrive standing up — so the first turn reads
/// the tapped half and only the untap step after it reads the two mana lines.
/// Each line is activated off an asserted-empty pool, so the counts are claims
/// about this land's own tap and not about mana already floating, and the
/// sacrifice half is followed to its owner's graveyard, which is where a land
/// given up to pay a cost goes.
#[test]
fn svyelunite_temple_enters_tapped_then_taps_and_sacrifices_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(211, forest())
        .hand(0, &[svyelunite_temple()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let temple = play_land(&mut engine, p0, svyelunite_temple());
    assert!(
        on_battlefield(&engine, p0, svyelunite_temple()).is_some(),
        "the land reached the battlefield"
    );
    assert!(
        is_tapped(&engine, temple),
        "\"This land enters tapped\" — a real land drop, so the entry modifier runs"
    );

    // Across the opponent's turn and back: a permanent that entered tapped is
    // standing again only after its controller's untap step (CR 502.3).
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, temple),
        "the untap step stood it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing of the turn before is still floating"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(temple, 0)),
        "a printed mana ability is an ordinary (source, index) entry, and this \
         land has no basic land type to be the CR 305.6 shortcut: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(temple, 1)),
        "and the sacrifice line is offered beside it: {:?}",
        legal.abilities
    );

    // Ability 0: "{T}: Add {U}."
    activate(&mut engine, p0, svyelunite_temple(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "one blue, off the land's own tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing else came with it"
    );
    assert!(is_tapped(&engine, temple), "which tapped the land");
    assert!(
        on_battlefield(&engine, p0, svyelunite_temple()).is_some(),
        "and the first line costs the land nothing but its tap"
    );

    // Another turn, so the land is untapped and the pool is empty once more:
    // that empty pool is what makes the count below an exact claim about the
    // sacrifice line instead of one about the blue already sitting there.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, temple), "untapped again");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the turn"
    );

    // Ability 1: "{T}, Sacrifice this land: Add {U}{U}."
    activate(&mut engine, p0, svyelunite_temple(), 1);
    assert!(
        stack_is_empty(&engine),
        "the second line is a mana ability too, so nothing is waiting on a stack"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        2,
        "\"Add {{U}}{{U}}\" — two blue from one activation, not one"
    );
    assert_eq!(pool.total(), 2, "and nothing else in the pool");
    assert!(
        on_battlefield(&engine, p0, svyelunite_temple()).is_none(),
        "sacrificing the land is part of the price, so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, svyelunite_temple()).is_some(),
        "and a sacrificed land goes to its owner's graveyard"
    );
}
