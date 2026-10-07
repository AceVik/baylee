//! `cards/creatures/mv_2/wall_of_razors.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Razors is `{1}{R}` for a 4/1 with Defender and First Strike —
/// two printed keywords that pull in opposite directions
/// and both must be read on the permanent. The scenario casts it from
/// two Mountains and reads body and both keywords on the battlefield,
/// and then goes to the attacker declaration: the Llanowar Elf next to it is
/// untapped and is offered, so an offer without the 4/1 is the
/// Defender line and not a combat step that never came.
#[test]
fn wall_of_razors_lands_as_a_four_one_with_defender_and_first_strike_and_never_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .hand(0, &[wall_of_razors()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert!(!is_tapped(&engine, elves), "and untapped, so it may attack");

    // {1}{R} from the two Mountains, with the Elf as a held-back
    // printing: it is the control against which the attacker
    // declaration below is read, and a creature tapped for mana no
    // longer attacks.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains and no Elf: two red mana for the {{1}}{{R}}"
    );
    cast_with_floating(&mut engine, p0, wall_of_razors());
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, wall_of_razors()).expect("die Wall ist gelandet");
    assert_eq!(pt(&engine, wall), (4, 1), "der gedruckte Körper");
    let granted = keywords(&engine, wall);
    assert!(granted.contains(KeywordSet::DEFENDER), "Defender");
    assert!(
        granted.contains(KeywordSet::FIRST_STRIKE),
        "First strike, aus demselben projizierten Satz"
    );

    // The gap that the Defender line opens, and its control in the same
    // offer: the engine counts the Elf and not the Wall.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until hält nur bei der Angreifer-Erklärung")
    };
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 without its own text may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "the 4/1 may not: its first strike is printed, and so is the defender \
         that keeps it out of every attack declaration: {attackers:?}"
    );
}
