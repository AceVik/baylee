//! `cards/instants/mv_3/daring_leap.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Daring Leap — {1}{W}{U} instant: "Target creature gets +1/+1 and gains
/// flying and first strike until end of turn."
///
/// The word "creature" only shows in the menu, so the board carries three
/// lands, one Elf under the caster and one across the table: two options is
/// a claim about the CR 115.1 target the card prints, and a wildcard filter
/// would have offered the lands too. The pump and both keywords have to land
/// on the creature that was named and on no other — a grant that swept the
/// board would leave the bystander flying — and a whole turn cycle later the
/// 2/2 flier with first strike is a printed 1/1 again, which is the printed
/// duration read off the board rather than taken from the card file.
#[test]
fn daring_leap_pumps_and_arms_only_the_creature_it_names_and_only_for_the_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), island(), forest(), quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[daring_leap()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the spell");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "nothing has granted anything yet"
    );

    // Mana before the claim: the cast is a real {1}{W}{U}, and what is
    // castable is filtered through the pool rather than through the untapped
    // lands, so the three sources are tapped first.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, daring_leap());

    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and nothing else on this board is one — the three lands are permanents \
         and no creatures: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the spell named"
    );
    let granted = keywords(&engine, host);
    assert!(granted.contains(KeywordSet::FLYING), "and flying");
    assert!(
        granted.contains(KeywordSet::FIRST_STRIKE),
        "and first strike"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "nor does the grant: one target, one creature"
    );

    // "until end of turn": a turn cycle later the Elf is the 1/1 it was
    // printed as, so the body and the keywords were a duration and not a
    // permanent change.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "the creature is still standing, so the grant left rather than the creature"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the +1/+1 lasted the turn it was made in and no longer"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "and so did the flying"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "and the first strike, which a permanent grant would have kept"
    );
}
