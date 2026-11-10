//! `cards/instants/mv_1/giant_growth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Berserk: "Target creature gains trample and gets +X/+0 until end of
/// turn, where X is its power." X is read as Berserk *resolves*, not as it
/// is cast: Berserk is cast at the Elf while it is still a 1/1, then Giant
/// Growth is cast on top of it and resolves first (last in, first out), so
/// by the time Berserk itself resolves the Elf is at 4 power. A cast-time X
/// would still be 1 and leave the Elf at 5/4; reading X at resolution
/// leaves it at 8/4 instead. A second Giant Growth cast afterwards adds its
/// own +3/+3 without inflating Berserk's now-fixed bonus further, and the
/// "+0" half never touches toughness. The bystander Goblin beside it is
/// never targeted and never moves. By the next turn every "until end of
/// turn" grant this test made — trample included — is gone.
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn berserk_pumps_by_the_targets_power_at_resolution_and_only_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                llanowar_elves(),
                festering_goblin(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[giant_growth(), giant_growth(), berserk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let goblin = on_battlefield(&engine, p0, festering_goblin()).expect("the Goblin is seated");
    assert_eq!(pt(&engine, elf), (1, 1), "printed 1/1");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "nothing has been granted anything yet"
    );

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, berserk());
    let options = aim_at(&mut engine, p0, elf);
    assert!(
        options.contains(&goblin),
        "\"target creature\" is not restricted to a boosted one: {options:?}"
    );

    // Still p0's own priority: Giant Growth stacks above Berserk and
    // resolves first, so the Elf is at 4 power — not its printed 1 — when
    // Berserk's own resolution reads X.
    cast_with_floating(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elf),
        (8, 4),
        "Giant Growth resolves first, to 4 power; Berserk then reads \
         X = 4 at its own resolution and adds +4/+0 — a cast-time X would \
         have frozen at 1 and left the Elf at 5/4, not 8/4 — and toughness \
         is untouched by \"+X/+0\""
    );
    assert_eq!(pt(&engine, goblin), (1, 1), "never targeted, never moved");
    assert!(
        keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "\"gains trample\""
    );
    assert!(
        !keywords(&engine, goblin).contains(KeywordSet::TRAMPLE),
        "the Goblin beside it was never Berserk's target"
    );

    cast_with_floating(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elf),
        (11, 7),
        "the second Giant Growth adds its own +3/+3; Berserk's own +4/+0 \
         does not grow along with the Elf's power after the fact"
    );
    assert!(
        keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "still this same turn"
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "every \"until end of turn\" bonus is gone, Berserk's included"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "the trample granted \"until end of turn\" does not outlast it"
    );
}
