//! `cards/creatures/mv_4/vile_deacon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "123147b4-57d0-44cd-bdd5-a449ac86c1cb"

/// Vile Deacon — {2}{B}{B} 2/2 Human Cleric: "Whenever this creature attacks,
/// it gets +X/+X until end of turn, where X is the number of Clerics on the
/// battlefield."
///
/// "Clerics on the battlefield" is the whole card, and the board is built so
/// that three readings of it give three different bodies: the Deacon itself
/// and the Deacon across the table are two, so +2/+2; a filter that had read
/// "Clerics you control" would have found one and left a 3/3, and one that
/// counted creatures would have found three — the Llanowar Elves beside it
/// included — and left a 5/5. The Elves and the second Deacon are also the
/// control for the other half of the sentence: the pump names `Filter::This`,
/// so neither may grow. The turn is walked to its end because the printed
/// duration is part of the card, and to the end step rather than to an empty
/// stack because the stack is empty the moment attackers are declared and
/// every life total still reads twenty there.
#[test]
fn vile_deacon_pumps_itself_for_the_clerics_on_both_sides_of_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vile_deacon(), llanowar_elves()])
        .battlefield(1, &[vile_deacon()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let deacon = on_battlefield(&engine, p0, vile_deacon()).expect("the Deacon is out");
    let theirs = on_battlefield(&engine, p1, vile_deacon()).expect("and one across the table");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert_eq!(
        pt(&engine, deacon),
        (2, 2),
        "a printed 2/2 before it attacks"
    );
    assert_eq!(pt(&engine, elves), (1, 1), "and a printed 1/1 beside it");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&deacon),
        "an untapped creature that was not cast this turn may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(deacon, Defender::Player(p1))],
            },
        )
        .expect("the Deacon came out of the list that offered it");

    // The trigger goes on the stack the moment the declaration is made and
    // resolves before combat damage (CR 509.1), so the life total read at the
    // end step belongs to the pumped body and not to the printed one.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        pt(&engine, deacon),
        (4, 4),
        "+2/+2 for the two Clerics on the battlefield — the Deacon itself and \
         the one across the table. \"Clerics you control\" would read one and \
         leave a 3/3; counting creatures would read three and leave a 5/5"
    );
    assert_eq!(
        engine.state().players[1].life,
        16,
        "and the pump was live when the damage happened: an unblocked 4/4 is \
         four life"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the effect names `Filter::This`: the Elves are no Cleric, so they are \
         neither counted nor granted anything"
    );
    assert_eq!(
        pt(&engine, theirs),
        (2, 2),
        "and the Deacon across the table is a Cleric — counted — and not the \
         attacker — unpumped"
    );

    // The printed duration, on the same board a turn later: a static or an
    // indefinite grant would still be on the creature here.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, deacon),
        (2, 2),
        "\"until end of turn\": the pump lasted the turn it was made in and no \
         longer"
    );
    assert_eq!(
        engine.state().players[1].life,
        16,
        "and the damage it dealt is not undone with it"
    );
}
