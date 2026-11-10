//! `cards/creatures/mv_3/wall_of_heat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Heat is `{2}{R}` for a printed 2/6 and one word of rules text:
/// "Defender (This creature can't attack.)". Casting it is what says the
/// body is real — a `(2, 6)` read off the projected characteristics, not off
/// the card file — and the attack declaration is what says the word is:
/// CR 508.1a leaves a defender out of `ChooseAttackers` entirely.
///
/// The control is the whole point of the scenario. An offer without the Wall
/// proves nothing on its own, because a combat step that never came, or a
/// creature the engine withheld for a reason of its own, would satisfy it
/// just as well; so an untapped Llanowar Elves stands under the same seat and
/// must be offered, which is what makes the missing Wall the defender rule
/// rather than an empty declaration. The Elves are named as the printing kept
/// back from `tap_all_mana_but` for the same reason: a mana creature tapped
/// to pay for the Wall is a creature with its own reason not to attack.
#[test]
fn wall_of_heat_lands_as_a_two_six_defender_the_attack_declaration_never_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[wall_of_heat()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{R} off the three Mountains, with the Elves kept standing: they are
    // the control the attack declaration below is read against.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, wall_of_heat());
    pass_until(&mut engine, stack_is_empty);
    let wall = on_battlefield(&engine, p0, wall_of_heat()).expect("the Wall resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");

    assert_eq!(pt(&engine, wall), (2, 6), "the printed 2/6 body");
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "Defender is the whole of the card's rules text"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 with no text of its own may attack, so the offer is \
         a real combat step and not an empty one: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "\"Defender (This creature can't attack.)\" — the 2/6 may not: {attackers:?}"
    );
}
