//! `cards/creatures/mv_3/skyhunter_prowler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skyhunter Prowler prints a 1/3 Cat Knight for {2}{W} whose whole text is
/// flying and vigilance. A body alone cannot show vigilance — "attacking
/// doesn't cause this creature to tap" is only readable by attacking with it
/// and finding it still standing afterwards — and flying needs a witness that
/// is not itself printed on the card, which is the ground Elf across the
/// table: a 1/1 with no flying and no reach is never offered as its blocker.
/// The one point of life p1 loses says the attack really connected, so the
/// untapped Prowler afterwards is vigilance and not a declaration that was
/// rolled back.
#[test]
fn skyhunter_prowler_attacks_without_tapping_and_flies_over_the_ground() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[skyhunter_prowler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Plains pay {2}{W}, and the mana is tapped before the spell is
    // claimed as castable because `can_afford` reads the pool.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the spell is paid for"
    );
    cast_from_hand(&mut engine, p0, skyhunter_prowler());
    pass_until(&mut engine, stack_is_empty);
    let prowler = on_battlefield(&engine, p0, skyhunter_prowler()).expect("the Prowler resolved");
    assert_eq!(pt(&engine, prowler), (1, 3), "a printed 1/3");
    let granted = keywords(&engine, prowler);
    assert!(
        granted.contains(KeywordSet::FLYING),
        "flying is printed on it"
    );
    assert!(
        granted.contains(KeywordSet::VIGILANCE),
        "and so is vigilance"
    );

    // Summoning sickness (CR 302.6) keeps it out of this turn's combat, so the
    // attack that reads the vigilance is a full turn cycle later — and by then
    // the Plains that paid for it have untapped with it.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, prowler), "untapped and ready to attack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&prowler),
        "an untapped 1/3 flier may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(prowler, Defender::Player(p1))],
            },
        )
        .expect("the Prowler was one of the offered attackers");

    assert!(
        !is_tapped(&engine, prowler),
        "\"attacking doesn't cause this creature to tap\" — vigilance leaves it \
         standing the moment it is declared"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        blockers
            .iter()
            .all(|option| !option.attackers.contains(&prowler)),
        "the Elf across the table has neither flying nor reach, so it is no \
         legal blocker for the Prowler: {blockers:?}"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declaring no blockers is always legal");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the one point of combat damage got through, so the attack was real"
    );
    assert!(
        !is_tapped(&engine, prowler),
        "and it is still untapped after damage (CR 510.2), which is vigilance \
         and not an untap step that has come round"
    );
}
