//! `cards/creatures/mv_1/wall_of_wood.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Wood is `{G}` for a 0/3 Wall whose entire text is Defender
/// (CR 702.3), "This creature can't attack."
///
/// The card is cast rather than placed so the `{G}` is really paid, and the
/// keyword's other side is played too: a defender is barred from attacking
/// and from nothing else, so the Wall blocks on the opponent's next turn.
/// The attack declaration one turn later is where the printed line has to
/// show — the Wall is untapped and has been under its controller's control
/// since the turn began (CR 302.6), so Defender is the only thing left that
/// can keep it off the list, while the untapped 1/1 standing beside it is
/// offered, which is what stops an empty list from passing for a combat step
/// that never arrived.
#[test]
fn wall_of_wood_blocks_but_is_never_offered_as_an_attacker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), quiet_creature()])
        .hand(0, &[wall_of_wood()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // One Forest pays the {G}. The Elf is kept standing because it is this
    // test's control in the attack declaration below, and a creature tapped
    // for mana is no longer offered as an attacker.
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    cast_with_floating(&mut engine, p0, wall_of_wood());
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, wall_of_wood()).expect("the Wall resolved");
    assert_eq!(pt(&engine, wall), (0, 3), "the body the card prints");
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "the one line the card prints reaches the permanent"
    );

    // The opponent's turn, where the Wall is asked for the thing a defender
    // is actually for. Blocking is not attacking (CR 702.3), so a keyword
    // that read "this creature can't be declared as an attacker" rather than
    // "can't attack" would fail here and nowhere else.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&their_elf),
        "their untapped 1/1 may attack: {attackers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(their_elf, Defender::Player(p0))],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p0),
    );
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the block declaration")
    };
    let option = blockers
        .iter()
        .find(|b| b.blocker == wall)
        .expect("the Wall may block — Defender bars attacking and nothing else");
    assert!(
        option.attackers.contains(&their_elf),
        "and it may block the creature that is attacking: {option:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, their_elf)],
            },
        )
        .unwrap();

    // Past that damage step (CR 510.2) and on into p0's own combat, where
    // the Wall has now been under its controller's control since before the
    // turn began.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    assert!(
        on_battlefield(&engine, p0, wall_of_wood()).is_some(),
        "the 0/3 blocked the 1/1 and survived the one damage it dealt back"
    );
    assert!(
        !is_tapped(&engine, wall),
        "and stands untapped for this declaration"
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&elves),
        "the untapped 1/1 beside it is offered, so this combat step is real: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "\"This creature can't attack\" (CR 702.3) — untapped, past summoning \
         sickness (CR 302.6), and still not on the list: {attackers:?}"
    );
}
