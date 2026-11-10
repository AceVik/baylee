//! `cards/creatures/mv_5/ambush_party.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ambush Party is `{4}{R}` for a 3/1 Human Rogue whose whole text is two
/// keywords: first strike and haste. Both are read off the board rather than
/// off the card file, and each needs its own event. Haste is the attack
/// declaration in the very turn the creature arrived — an untapped 3/1 that
/// entered this main phase is otherwise not offered as an attacker at all.
/// First strike is the block: a 1/1 Elf standing across the table kills a 3/1
/// in the *regular* damage step and dies to it in the same instant, so the
/// Party being alive afterwards with the Elf in its owner's graveyard is what
/// tells a first-strike damage step from a simultaneous one.
#[test]
fn ambush_party_attacks_the_turn_it_arrives_and_kills_its_blocker_with_first_strike() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[ambush_party()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Five Mountains pay `{4}{R}` and nothing else on the board is tapped:
    // the Party is cast the way the card is cast, so the keywords asserted
    // below are on a permanent that arrived through the stack.
    cast_from_hand(&mut engine, p0, ambush_party());
    pass_until(&mut engine, stack_is_empty);
    let party = on_battlefield(&engine, p0, ambush_party()).expect("the Party resolved");
    assert_eq!(pt(&engine, party), (3, 1), "the body the card prints");
    let granted = keywords(&engine, party);
    assert!(
        granted.contains(KeywordSet::FIRST_STRIKE),
        "the printed first strike reaches the permanent"
    );
    assert!(granted.contains(KeywordSet::HASTE), "and so does its haste");

    // The Elf is the blocker this scenario turns on: a 1/1 across the table
    // whose one damage is lethal to a 3/1, so it can only survive the combat
    // if it strikes first.
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the active seat declares its attackers");
    assert!(
        attackers.contains(&party),
        "haste: the Party entered this turn and is still offered as an attacker: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(party, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p1, "the defending seat is the one asked to block");
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&party)),
        "the Elf may block the Party: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, party)],
            },
        )
        .expect("the pairing came out of the offer");

    // Past both damage steps and the rest of the turn, so the board read
    // below is the one combat left behind.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three damage killed the blocker"
    );
    assert!(
        on_battlefield(&engine, p0, ambush_party()).is_some(),
        "and the Party survived a blocker that trades with it in a regular \
         damage step — the Elf's one damage is lethal to a 3/1, so only a \
         first-strike step could have killed the Elf before it dealt any"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that blocked and never to the player"
    );
}
