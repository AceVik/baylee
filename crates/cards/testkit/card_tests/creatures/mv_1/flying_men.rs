//! `cards/creatures/mv_1/flying_men.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flying Men prints one line — flying — on a {U} 1/1 Human, so the whole
/// card is the distance between "the keyword is on the card" and "the keyword
/// is on the permanent that arrived". The test casts it for {U} off an Island
/// and reads flying off the layer projection, with an Elf beside it as the
/// control: a keyword that never reached the battlefield would show on
/// neither, and a board-wide one on both. The attack is the second half — a
/// 1/1 whose body has to be real for the one damage to land.
#[test]
fn flying_men_arrives_as_a_flying_one_one_and_deals_its_one_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), quiet_creature()])
        .hand(0, &[flying_men()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, flying_men());
    pass_until(&mut engine, stack_is_empty);

    let men = on_battlefield(&engine, p0, flying_men()).expect("the spell resolved");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf beside it is out");
    assert!(
        types(&engine, men).contains(TypeSet::CREATURE),
        "it arrives as a creature"
    );
    assert_eq!(pt(&engine, men), (1, 1), "with the printed 1/1 body");
    assert!(
        keywords(&engine, men).contains(KeywordSet::FLYING),
        "and flying, read off the permanent rather than off the card file"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "the Elf beside it is the control: the keyword belongs to one \
         creature and not to the board"
    );

    // A turn later the 1/1 is out of summoning sickness (CR 302.6), so the
    // combat step has to offer it.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        attackers.contains(&men),
        "an untapped 1/1 with no text but a keyword is offered as an \
         attacker: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(men, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the flier goes unblocked across an empty table, so its one power is \
         exactly the one life"
    );
}
