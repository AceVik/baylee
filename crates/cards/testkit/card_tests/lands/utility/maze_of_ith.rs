//! `cards/lands/utility/maze_of_ith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Maze of Ith prints one line: "{T}: Untap target attacking creature. Prevent
/// all combat damage that would be dealt to and dealt by that creature this
/// turn."
///
/// The card makes three claims and the scenario is built so each is load-bearing.
/// My Elf attacks, so declaring it *tapped* it (CR 508.1f) and it is the only
/// creature on the board that is attacking — the Elf across the table is a
/// creature and no legal target, which is what reads "attacking" and not
/// "creature". That attack is then blocked by the same Elf, and a printed 1/1
/// blocked by a printed 1/1 kills both: "both are still standing" is therefore a
/// sentence only two working prevention clauses satisfy, while the control game
/// at the foot of the test shows the pair dying when the Maze stays out of it.
#[test]
#[allow(clippy::too_many_lines)] // one card, both of its sentences, and the control that proves them
fn maze_of_ith_untaps_an_attacker_and_prevents_its_combat_damage_both_ways() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[maze_of_ith(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let maze = on_battlefield(&engine, p0, maze_of_ith()).expect("the Maze is on the table");
    let attacker = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        (pt(&engine, attacker), pt(&engine, blocker)),
        ((1, 1), (1, 1)),
        "two printed 1/1s, so one point of damage in either direction is lethal \
         and what survives the damage step says which damage was prevented"
    );
    assert!(!is_tapped(&engine, attacker), "nothing has attacked yet");
    assert!(
        !is_tapped(&engine, maze),
        "and the land has paid for nothing"
    );

    // The attack has to be a real one: the ability's whole target filter is the
    // attacking state, so a board with nobody attacking offers nothing.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the attacking seat is the one that declares");
    assert!(
        attackers.contains(&attacker),
        "an untapped 1/1 that has been on the battlefield since before the game \
         may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    assert!(
        is_tapped(&engine, attacker),
        "CR 508.1f: declaring it as an attacker tapped it, so the Maze has an \
         untap to do"
    );

    // CR 508.2: priority comes back to the active player once attackers are
    // declared, and that window between the attack and the blocks is where the
    // Maze is played in.
    let mut activated = false;
    for _ in 0..20 {
        if at_rest(&engine, p0) {
            activate(&mut engine, p0, maze_of_ith(), 0);
            activated = true;
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "expected priority between the attack and the blocks, got {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    assert!(
        activated,
        "the attacking seat is offered priority before blockers are declared"
    );

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the untap asks for an attacking creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        (player, min, max),
        (p0, 1, 1),
        "one target, and it asks once"
    );
    assert!(
        !is_tapped(&engine, maze),
        "CR 601.2h pays last: the {{T}} is still unpaid while the question stands"
    );
    assert!(
        options.contains(&attacker),
        "the creature that is attacking is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&blocker),
        "\"target attacking creature\" is not \"target creature\": the Elf \
         across the table is a creature and no attacker, and the land is no \
         creature at all: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .expect("the attacking creature was one of the options");
    assert!(
        is_tapped(&engine, maze),
        "{{T}} is the other half of the price"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, attacker),
        "\"Untap target attacking creature\" — the creature it named is standing \
         back up in the middle of the combat it is in"
    );

    // Untapping an attacker is not removing it from combat (CR 506.4), which is
    // what the whole card rests on: the creature it untapped is still the
    // creature the blocks are declared against, and still the one whose damage
    // the second sentence is about.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the block declaration")
    };
    assert_eq!(player, p1, "the defending seat is the one that declares");
    let offer = blockers
        .iter()
        .find(|o| o.blocker == blocker)
        .expect("the Elf that is not attacking is offered as a blocker");
    assert!(
        offer.attackers.contains(&attacker),
        "the attacker is still attacking after the untap, so it is still the \
         thing the blocks are declared against: {offer:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .expect("the pairing came out of the offer the engine published");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "\"prevent all combat damage that would be dealt to ... that creature\": \
         one point from a printed 1/1 would have been lethal, and the attacker \
         took none of it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and \"... that would be dealt by that creature\": the same one point \
         would have killed the blocker, and the blocker took none of it either"
    );

    // The control: the identical board and the identical attack and block, with
    // the Maze left out of it. Without the activation the two printed 1/1s kill
    // each other, which is what makes the pair above a statement about the card
    // rather than about a damage step that never happened.
    let mut control = Duel::new(SEED, forest())
        .battlefield(0, &[maze_of_ith(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut control);
    assert!(
        walk_to_own_main(&mut control, p0),
        "p0 reaches its own main"
    );
    let plain_attacker = on_battlefield(&control, p0, llanowar_elves()).expect("my Elf is out");
    let plain_blocker = on_battlefield(&control, p1, llanowar_elves()).expect("their Elf is out");
    let offered = attack_and_collect_blocks(&mut control, plain_attacker, p1);
    assert!(
        offered
            .iter()
            .any(|o| o.blocker == plain_blocker && o.attackers.contains(&plain_attacker)),
        "the same pairing is offered on the same board: {offered:?}"
    );
    control
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(plain_blocker, plain_attacker)],
            },
        )
        .expect("the pairing came out of the offer");
    pass_until(&mut control, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert!(
        on_battlefield(&control, p0, llanowar_elves()).is_none()
            && on_battlefield(&control, p1, llanowar_elves()).is_none(),
        "with no Maze activation the blocked 1/1 and the 1/1 that blocked it \
         trade their one damage each and both die, so the survivors above \
         survived because the damage was prevented"
    );
}
