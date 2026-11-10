//! `cards/creatures/mv_5/jhovall_rider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jhovall Rider is a {4}{W} 3/3 Human Rebel whose entire printed rules text
/// is trample, so the only thing a board can hold it to is the cost and the
/// body it prints. Six Plains pay the {4}{W} down to a single white left in
/// the pool, and the permanent that arrives is a 3/3 carrying the keyword —
/// with the Llanowar Elves standing beside it as the control, because a
/// static that had lost "this creature" and granted trample to the board
/// would read exactly the same on the Rider alone.
#[test]
fn jhovall_rider_costs_five_and_a_white_for_the_trampling_three_three_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[jhovall_rider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Plains and no Elf: the Elk stays untapped, so the pool read here is
    // exactly the six mana a {4}{W} spell is paid out of and nothing else.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Plains tapped, and the Elf beside them kept out of the pool"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p0, jhovall_rider()).expect("the Rider is in hand");
    assert!(
        legal.castable.contains(&card),
        "{{4}}{{W}} is affordable off a pool of six white: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, jhovall_rider());
    pass_until(&mut engine, stack_is_empty);
    let rider = on_battlefield(&engine, p0, jhovall_rider()).expect("the Rider resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{4}}{{W}} came out of the pool and left the sixth white behind"
    );
    assert_eq!(pt(&engine, rider), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, rider).contains(KeywordSet::TRAMPLE),
        "the one line of rules text the card prints reaches the permanent"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "and belongs to the creature itself: the Elf beside it is untouched"
    );
}
