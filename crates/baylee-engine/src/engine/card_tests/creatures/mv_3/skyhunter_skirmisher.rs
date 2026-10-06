//! `cards/creatures/mv_3/skyhunter_skirmisher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn skyhunter_skirmisher_flies_and_strikes_twice_for_two_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        // {1}{W}{W}: three Plains, and nothing else on the board could pay a
        // white pip.
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[skyhunter_skirmisher()])
        // A ground 1/1 across the table: it is what "flying" is about, since
        // it is a legal blocker for anything that is not a flier.
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, skyhunter_skirmisher());
    pass_until(&mut engine, stack_is_empty);

    let knight = on_battlefield(&engine, p0, skyhunter_skirmisher())
        .expect("the Skirmisher resolved onto the battlefield");
    assert_eq!(pt(&engine, knight), (1, 1), "the printed 1/1 body");
    let projected = keywords(&engine, knight);
    assert!(
        projected.contains(KeywordSet::FLYING),
        "the printed flying reached the permanent through the layers"
    );
    assert!(
        projected.contains(KeywordSet::DOUBLE_STRIKE),
        "and so did double strike: {projected:?}"
    );

    // CR 302.6: a creature that entered this turn cannot attack, and this one
    // did. A whole turn cycle is what makes the attack below legal.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, knight),
        "nothing tapped it on the way round"
    );

    let life_before = engine.state().players[1].life;
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&knight),
        "an untapped, unsick Skirmisher may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(knight, Defender::Player(p1))],
            },
        )
        .unwrap();

    // The walk answers the blocker question with nobody blocking, and the
    // ground 1/1 across the table is not a pairing the engine could have
    // offered for a flier anyway. The end step is past both damage steps
    // (CR 510.2, CR 510.4).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        life_before - 2,
        "a printed 1/1 with double strike deals its damage in the first-strike \
         step and again in the regular one: two life, where a single strike \
         would have taken one"
    );
    assert!(
        on_battlefield(&engine, p0, skyhunter_skirmisher()).is_some(),
        "the attacker took no damage back, because nothing blocked it"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_some(),
        "and the creature that could not block is untouched"
    );
}
