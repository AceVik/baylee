//! `cards/instants/mv_1/unnatural_speed.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Unnatural Speed is one red instant — Arcane — reading "Target creature
/// gains haste until end of turn", and the keyword alone proves nothing: haste
/// exists to beat summoning sickness (CR 302.6), so the only board that reads
/// the card is one where a creature *arrived this turn*. Two identical Elves
/// are cast in the same first main phase and only one of them is aimed at, so
/// the pair has to part company in the attack declaration — the hasted Elf is
/// offered although it just entered, and the one beside it is not. The Elf
/// across the table is the control for the second half of the target line:
/// "target creature" is not "target creature you control".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn unnatural_speed_lets_a_creature_that_arrived_this_turn_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(197, forest())
        .battlefield(0, &[mountain(), mountain(), forest(), forest()])
        .hand(0, &[quiet_creature(), quiet_creature(), unnatural_speed()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    // Four lands and no creature of p0's yet, so the pool is exactly these
    // four and every number below is one of them being spent.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Mountains and two Forests, and nothing else on this board makes mana"
    );

    cast_with_floating(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);
    let fast = on_battlefield(&engine, p0, quiet_creature()).expect("the first Elf resolved");
    cast_with_floating(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);

    let pair = all_on_battlefield(&engine, p0, quiet_creature());
    assert_eq!(
        pair.len(),
        2,
        "two Elves, and both arrived in this same turn"
    );
    let slow = pair
        .iter()
        .copied()
        .find(|id| *id != fast)
        .expect("the second Elf is on the table too");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, fast).contains(KeywordSet::HASTE),
        "nothing has been aimed at yet"
    );

    cast_with_floating(&mut engine, p0, unnatural_speed());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert!(
        options.contains(&fast) && options.contains(&slow),
        "both Elves of mine are creatures: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is not \"target creature you control\": {options:?}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "the two Elves spent the green, so the {{R}} is still in the pool while \
         the target question stands (CR 601.2c before CR 601.2h)"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fast],
            },
        )
        .expect("the Elf the question offered is the one that was aimed at");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, unnatural_speed()).is_some(),
        "an instant that resolved is in its owner's graveyard"
    );
    assert!(
        keywords(&engine, fast).contains(KeywordSet::HASTE),
        "the Elf the spell named has haste until end of turn"
    );
    assert!(
        !keywords(&engine, slow).contains(KeywordSet::HASTE),
        "and the identical Elf beside it does not: the pump reaches one target \
         and no other"
    );

    // The functional half. Haste is only worth anything against summoning
    // sickness, and this is where the two Elves have to disagree.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&fast),
        "an untapped creature that arrived this turn may attack because the \
         spell gave it haste: {attackers:?}"
    );
    assert!(
        !attackers.contains(&slow),
        "while the Elf that arrived on the same turn and was not aimed at is \
         still sick (CR 302.6): {attackers:?}"
    );
}
