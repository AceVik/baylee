//! `cards/creatures/mv_4/keening_banshee.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Keening Banshee is a 2/2 flier whose enters-trigger gives one target
/// creature -2/-2 until end of turn. A Wall of Roots is the body worth aiming
/// at, because its printed 0/5 makes the number legible where a 1/1 would only
/// say that something happened: `(-2, 3)` is exactly what the printed -2/-2
/// produces on it, while a -1/-1 would leave `(-1, 4)` and a -3/-3 `(-3, 2)`.
/// The Elf across the table is the counter-half of "target creature" — it is
/// offered to the trigger and must still be a printed 1/1 afterwards — and the
/// whole turn walked at the end is what tells the printed "until end of turn"
/// from a permanent shrink.
#[test]
fn keening_banshee_shrinks_what_it_targets_and_only_until_the_turn_ends() {
    fn wall_of_roots() -> CardIndex {
        card_index("3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f")
    }

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), wall_of_roots()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[keening_banshee()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p0, wall_of_roots()).expect("the Wall is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, wall),
        (0, 5),
        "a printed 0/5 before the Spirit arrives"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "and a printed 1/1 across the table"
    );

    // {2}{B}{B} out of four Swamps. The Wall is the only other permanent that
    // could make mana, and it is no route for `tap_all_mana`: its price is a
    // -0/-1 counter rather than its own {T} (#159), so it is still standing
    // untouched when the trigger asks what to aim at.
    cast_from_hand(&mut engine, p0, keening_banshee());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let banshee = on_battlefield(&engine, p0, keening_banshee()).expect("the Banshee resolved");
    assert_eq!(pt(&engine, banshee), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, banshee).contains(KeywordSet::FLYING),
        "and the printed flying reaches the permanent"
    );

    let options = aim_at(&mut engine, p0, wall);
    assert!(
        options.contains(&wall),
        "a creature under this seat's control is a legal target: {options:?}"
    );
    assert!(
        options.contains(&elf),
        "\"target creature\" is any creature, not only one of mine: {options:?}"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wall),
        (-2, 3),
        "-2/-2 on a printed 0/5: a -1/-1 would leave (-1, 4) and a -3/-3 (-3, 2)"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the creature the trigger did not name never moved"
    );

    // "until end of turn": the Wall is still standing a turn later and the
    // shrink is not, so none of this was a counter or a static.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, wall_of_roots()).is_some(),
        "the creature is still on the battlefield"
    );
    assert_eq!(
        pt(&engine, wall),
        (0, 5),
        "the printed body came back, so the grant was a duration and no more"
    );
}
