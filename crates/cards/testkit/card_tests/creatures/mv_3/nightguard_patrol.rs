//! `cards/creatures/mv_3/nightguard_patrol.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nightguard Patrol is `{2}{W}` for a 2/1 Human Soldier whose whole printed
/// text is two words: first strike and vigilance. Both are read on a real board
/// rather than out of the card file — the cast, the 2/1 body through the layer
/// system, and then a combat step. Vigilance (CR 702.20) is the half a board can
/// *contrast*, so an untapped Llanowar Elves attacks beside the Patrol and the
/// declaration that taps the Elves is the same one that leaves the Patrol
/// standing; the Elves are named as the printing `tap_all_mana_but` keeps back,
/// because a control creature tapped for the cast's mana would have had a reason
/// of its own to be down. Both attackers are unblocked, so the three damage the
/// defending player takes is the 2/1 and the 1/1 and nothing else — and the
/// combat has to be the *next* turn's, since a creature that entered this turn
/// may not attack at all (CR 302.6).
#[test]
fn nightguard_patrol_attacks_without_tapping_and_the_creature_beside_it_taps() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .hand(0, &[nightguard_patrol()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Plains pay {2}{W}; the Elves are kept back because they are the
    // control the combat below is measured against, and a source tapped for
    // mana has already changed status for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains tapped and no creature"
    );
    cast_with_floating(&mut engine, p0, nightguard_patrol());
    pass_until(&mut engine, stack_is_empty);

    let patrol = on_battlefield(&engine, p0, nightguard_patrol()).expect("the Patrol resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves stand");
    assert_eq!(pt(&engine, patrol), (2, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{W}} came out of the pool"
    );
    let granted = keywords(&engine, patrol);
    assert!(
        granted.contains(KeywordSet::FIRST_STRIKE),
        "printed first strike reaches the permanent: {granted:?}"
    );
    assert!(
        granted.contains(KeywordSet::VIGILANCE),
        "and printed vigilance beside it: {granted:?}"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::VIGILANCE),
        "the control creature prints no such word"
    );

    // A creature cast this turn may not attack (CR 302.6), so the combat the
    // two keywords are read off is p0's next one.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, patrol),
        "untapped, and no longer summoning sick"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&patrol),
        "the 2/1 may attack: {attackers:?}"
    );
    assert!(
        attackers.contains(&elves),
        "and so may the untapped Elf beside it: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (patrol, Defender::Player(p1)),
                    (elves, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    // Not `stack_is_empty`: the stack is already empty the moment attackers are
    // declared, so that predicate stops the walk *before* the combat damage step
    // and every life total still reads 20. The end step is past damage
    // (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        17,
        "two damage from the Patrol and one from the Elves, and nothing else"
    );
    assert!(
        !is_tapped(&engine, patrol),
        "\"vigilance\" (CR 702.20): attacking does not tap it"
    );
    assert!(
        is_tapped(&engine, elves),
        "while the very same declaration taps the creature without the word — \
         which is what tells the keyword from an attack that never happened"
    );
}
