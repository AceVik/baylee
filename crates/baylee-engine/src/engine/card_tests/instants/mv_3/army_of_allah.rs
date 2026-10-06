//! `cards/instants/mv_3/army_of_allah.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Army of Allah — {1}{W}{W} instant: "Attacking creatures get +2/+0 until
/// end of turn."
///
/// Two of p0's creatures and one of p1's stand on the board, and only the
/// one declared as an attacker is pumped: the Elf at home and the opponent's
/// Ogre are creatures and neither carries the bonus, so what the card reads
/// is "attacking" (CR 508.1) and not "creatures".
///
/// The bonus is read at three moments. It is not there before the spell
/// resolves; it is there after combat has ended and the pumped creature is
/// no longer attacking — the set a resolving spell's continuous effect
/// affects is fixed as the effect begins (CR 611.2c), so the +2/+0 lasts
/// the turn and not merely the attack; and it is gone by p0's next turn,
/// which is where "until end of turn" ends (CR 514.2).
#[test]
fn army_of_allah_pumps_only_attacking_creatures_until_the_turn_ends() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[army_of_allah()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let attacker = on_battlefield(&engine, p0, grizzly_bears()).expect("the attacker-to-be");
    let home = on_battlefield(&engine, p0, llanowar_elves()).expect("the creature at home");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("the opponent's creature");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .expect("the Bears attack");
    assert_eq!(
        pt(&engine, attacker),
        (2, 2),
        "a printed 2/2 before the pump: the spell is what this test is waiting on"
    );

    // The active player holds priority after the declaration, and the three
    // Plains pay {1}{W}{W} out of the declare attackers step. The Elf is kept
    // untapped so the creature at home is never one of the sources this cast
    // spent.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains, three white, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, army_of_allah());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, attacker), (4, 2), "the attacker has the +2/+0");
    assert_eq!(pt(&engine, home), (1, 1), "the Elf is not attacking");
    assert_eq!(pt(&engine, theirs), (2, 2), "nor is the opponent's Ogre");

    // Combat ends and the pumped creature stops attacking. The bonus is
    // still there: the resolving spell fixed the set it affects (CR 611.2c)
    // and the duration, not the attack, is what holds it.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        pt(&engine, attacker),
        (4, 2),
        "the +2/+0 outlasts the combat it was cast in"
    );
    assert_eq!(
        engine.state().players[1].life,
        16,
        "the pumped Bear went through unblocked for its 4 power"
    );

    assert!(walk_to_own_main(&mut engine, p0), "p0's next turn");
    assert_eq!(
        pt(&engine, attacker),
        (2, 2),
        "\"until end of turn\" ended at cleanup (CR 514.2)"
    );
    assert_eq!(
        pt(&engine, home),
        (1, 1),
        "and the Elf never changed at all"
    );
}
