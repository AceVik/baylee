//! `cards/creatures/mv_2/kobold_overlord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kobold Overlord — {1}{R} — is a 1/2 Kobold printing `first strike` and
/// "Other Kobold creatures you control have first strike." The static is a
/// four-part filter and this board strikes two of its limbs: a Llanowar Elf
/// under the same seat is a creature you control and no Kobold, and a
/// Llanowar Elf across the table is a creature somebody else controls and no
/// Kobold — a filter that had lost `HasSubtype(KOBOLD)` arms the first, one
/// that had lost `ControlledByYou` arms the second, and the Sol Ring beside
/// them catches a filter that had lost `CREATURE`.
///
/// The Overlord's own first strike is the keyword it prints, so the positive
/// half of the static — another Kobold, which has none of its own — is the
/// one thing this scenario cannot reach: no handle in this crate names a
/// second Kobold, and nothing else on the board is one.
#[test]
fn kobold_overlord_arms_itself_and_no_creature_that_is_not_a_kobold() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), llanowar_elves(), quiet_artifact()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[kobold_overlord()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{R} off the two Mountains; every card on the board that is not a
    // Mountain makes mana too, so the cast is paid out of the pool the
    // helper filled rather than out of a claim about untapped lands.
    cast_from_hand(&mut engine, p0, kobold_overlord());
    pass_until(&mut engine, stack_is_empty);

    let overlord = on_battlefield(&engine, p0, kobold_overlord()).expect("the Overlord resolved");
    assert_eq!(pt(&engine, overlord), (1, 2), "the body the card prints");
    assert!(
        keywords(&engine, overlord).contains(KeywordSet::FIRST_STRIKE),
        "the Overlord's own first strike reaches it as the keyword it prints"
    );

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "\"other **Kobold** creatures you control\": a Llanowar Elf is a \
         creature you control and no Kobold"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "\"**you control**\" is not the whole table: the Elf across it stays a \
         keywordless 1/1"
    );
    assert!(
        !keywords(&engine, rock).contains(KeywordSet::FIRST_STRIKE),
        "and the Sol Ring under the same seat is no creature at all"
    );
}

/// Kobold Overlord: "First strike. Other Kobold creatures you control have
/// first strike."
#[test]
fn kobold_overlord_gives_first_strike_to_other_kobolds_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4608, plains())
        .battlefield(
            0,
            &[kobold_overlord(), kobold_taskmaster(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let kobold = on_battlefield(&engine, p0, kobold_taskmaster()).expect("the Taskmaster");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");
    assert!(keywords(&engine, kobold).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&engine, elves).contains(KeywordSet::FIRST_STRIKE));
}
