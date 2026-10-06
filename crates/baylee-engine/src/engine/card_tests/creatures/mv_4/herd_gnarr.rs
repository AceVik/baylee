//! `cards/creatures/mv_4/herd_gnarr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "89cf4d99-64e9-4e5c-9edc-ea077a193332"

/// Herd Gnarr prints a 2/2 body and one triggered ability: "Whenever another
/// creature you control enters, this creature gets +2/+2 until end of turn."
/// Three words of that sentence need three different boards, and one main
/// phase holds two of them. A Llanowar Elves cast beside it makes it 4/4, a
/// *second* Herd Gnarr then proves "another" — the copy that enters is a
/// printed 2/2 while the copy already standing grows again to 6/6 — and the
/// opponent's turn afterwards is where "until end of turn" is read, with both
/// copies back to the body they print while both are still on the battlefield.
#[test]
fn herd_gnarr_grows_for_your_creatures_and_only_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                herd_gnarr(),
            ],
        )
        .hand(0, &[llanowar_elves(), herd_gnarr()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let first = on_battlefield(&engine, p0, herd_gnarr()).expect("the Gnarr was seated");
    assert_eq!(
        pt(&engine, first),
        (2, 2),
        "the body the card prints, before anything else arrives"
    );

    // Five Forests pay the {G} the Elves cost and leave exactly the {3}{G} a
    // second Gnarr costs floating beside it, because a pool survives until the
    // step ends (CR 500.5) and this whole scenario lives in one main phase.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elves resolved onto the battlefield"
    );
    assert_eq!(
        pt(&engine, first),
        (4, 4),
        "\"whenever another creature you control enters, this creature gets \
         +2/+2 until end of turn\" — the Elf arriving is the whole trigger"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "one green paid for the Elves; the other four are still floating"
    );

    // The second Gnarr is where "another" is read: the copy that enters is no
    // other creature to its own trigger, and the copy already standing is.
    cast_with_floating(&mut engine, p0, herd_gnarr());
    pass_until(&mut engine, stack_is_empty);
    let gnarrs = all_on_battlefield(&engine, p0, herd_gnarr());
    assert_eq!(gnarrs.len(), 2, "both copies are on the battlefield");
    let second = *gnarrs
        .iter()
        .find(|id| **id != first)
        .expect("one of the two is not the copy this test started with");
    assert_eq!(
        pt(&engine, second),
        (2, 2),
        "a creature entering is not \"another\" creature to its own trigger"
    );
    assert_eq!(
        pt(&engine, first),
        (6, 6),
        "and the Gnarr that was already there grew for it as well"
    );

    // "until end of turn": the pump belongs to the turn it was made in, and
    // the opponent's first main phase is past the cleanup that ended it.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, first),
        (2, 2),
        "the two +2/+2 the first Gnarr collected lasted p0's turn and no longer"
    );
    assert_eq!(
        pt(&engine, second),
        (2, 2),
        "and the copy that never grew reads the printed body on the same board"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, herd_gnarr()).len(),
        2,
        "both Gnarrs are still standing, so the pump expired rather than the creatures"
    );
}
