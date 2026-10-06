//! `cards/creatures/mv_2/angelic_wall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Angelic Wall is a {1}{W} 0/4 Wall that prints Defender and Flying and
/// nothing else, so both keywords are the card and this plays them on the
/// battlefield it is cast onto. The attack declaration is where Defender is
/// read: a clean untapped Elf beside it is offered as an attacker and the
/// Wall is not, and a full turn is walked first so summoning sickness
/// (CR 302.6) cannot be what the missing offer means — a 0-power 0/4 meets
/// every other clause in CR 508.1a.
#[test]
fn angelic_wall_lands_as_a_flying_defender_that_no_attack_step_offers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), quiet_creature()])
        .hand(0, &[angelic_wall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{W} off the two Plains, with the Elf named as the one source kept
    // back: it is this test's control in the attack declaration below, and a
    // creature that had tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    cast_with_floating(&mut engine, p0, angelic_wall());
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, angelic_wall()).expect("the Wall resolved");
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is on the table");
    assert_eq!(pt(&engine, wall), (0, 4), "the body the card prints");
    let kw = keywords(&engine, wall);
    assert!(
        kw.contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );
    assert!(kw.contains(KeywordSet::DEFENDER), "and so does defender");

    // A turn away and back, so the Wall is not summoning sick: nothing but
    // Defender is left to keep it out of the attack declaration.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration");
    };
    assert!(
        attackers.contains(&elves),
        "an untapped, unsick 1/1 with no text of its own may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "and the 0/4 with flying is not: nothing but Defender (CR 702.3) \
         keeps it off this list, whose every other clause it meets — \
         untapped, unsick, and a creature that may attack with power 0"
    );
}
