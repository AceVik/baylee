//! `cards/creatures/mv_3/wall_of_granite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Granite prints `{2}{R}` for a 0/7 Wall whose entire rules text is
/// the defender keyword, so the body and the attack declaration are the whole
/// card and each is checked where it can be seen. A zero-power creature is
/// still a legal attacker (CR 508.1a asks nothing about power), which is what
/// makes its absence from the declaration evidence of the keyword and not of
/// the numbers. The untapped Elf beside it is the control: an untapped
/// creature under the same seat is offered, so an offer without the Wall is
/// defender and not a combat step that never came.
#[test]
fn wall_of_granite_lands_as_a_zero_seven_wall_the_combat_step_never_offers() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[wall_of_granite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Mountains pay the {2}{R}. The Elves are named as the printing kept
    // back: they are the control in the attack declaration below, and a
    // creature tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, wall_of_granite());
    pass_until(&mut engine, stack_is_empty);
    let wall = on_battlefield(&engine, p0, wall_of_granite()).expect("the Wall resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");

    assert_eq!(pt(&engine, wall), (0, 7), "the body the card prints");
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "and the printed keyword reaches the permanent on the battlefield"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 with no text of its own may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "the 0/7 may not. A creature with no power is still a legal attacker, \
         so its absence is defender (CR 702.3b) and not the body: {attackers:?}"
    );
}
