//! `cards/artifacts/mv_3/ebony_horse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ebony Horse prints one line: "{2}, {T}: Untap target attacking creature
/// you control. Prevent all combat damage that would be dealt to and dealt
/// by that creature this turn."
///
/// The scenario is the card's three claims in order. Declaring the Elf
/// attacked and tapped it (CR 508.1f); the Horse's first sentence stands it
/// back up in the middle of the combat it is in, and untapping does not
/// remove it from combat (CR 506.4), so the blocks are still declared
/// against it and its own point of damage is still due. The damage step then
/// has the two printed prevention clauses to satisfy: a printed 1/1 blocked
/// by a printed 1/1 normally trades, and both survivors are one claim each.
/// The target menu is read where it is published — the bystander under the
/// same seat is a creature and no attacker, the Elf across the table is a
/// creature this seat does not control, and the Horse itself is an artifact
/// — so the single entry is the whole of "target attacking creature you
/// control".
///
/// The control at the foot is the identical attack and block with no
/// activation, where the pair does trade: that is what makes the two
/// survivors above a statement about the prevention clauses rather than
/// about a damage step that never happened.
#[allow(clippy::too_many_lines)] // one card, both of its sentences, and the control that proves them
#[test]
fn ebony_horse_untaps_an_attacker_and_stops_its_combat_damage_both_ways() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ebony_horse(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let horse = on_battlefield(&engine, p0, ebony_horse()).expect("the Horse is on the table");
    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "the attacker and the creature that must stay home"
    );
    let (attacker, bystander) = (elves[0], elves[1]);
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        (pt(&engine, attacker), pt(&engine, blocker)),
        ((1, 1), (1, 1)),
        "two printed 1/1s, so one point of damage in either direction is \
         lethal and what survives the damage step says which damage was \
         prevented"
    );
    assert!(!is_tapped(&engine, attacker), "nothing has attacked yet");

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
        "an untapped Elf may attack: {attackers:?}"
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
        "CR 508.1f: declaring it as an attacker tapped it, so the Horse has \
         an untap to do"
    );

    // CR 508.2: priority comes back to the active player once attackers are
    // declared, and that window between the attack and the blocks is where
    // the Horse is activated. The {2} is floated first, from the Forests;
    // the bystander Elf may pay too, so the attacker is not the only thing
    // that moved.
    let mut activated = false;
    for _ in 0..20 {
        if at_rest(&engine, p0) {
            tap_mana_except(&mut engine, p0, horse);
            activate(&mut engine, p0, ebony_horse(), 0);
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
        options.contains(&attacker),
        "\"target attacking creature you control\" names the attacker: {options:?}"
    );
    assert!(
        !options.contains(&bystander),
        "the bystander Elf is a creature I control and no attacker: {options:?}"
    );
    assert!(
        !options.contains(&blocker),
        "their Elf is a creature this seat does not control: {options:?}"
    );
    assert!(
        !options.contains(&horse),
        "the Horse itself is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 1, "and the attacker is the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .expect("the only creature on the menu is a legal answer");
    assert!(
        is_tapped(&engine, horse),
        "the {{T}} is the other half of the price"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, attacker),
        "\"Untap target attacking creature\": it stands back up in the middle \
         of the combat it is in (CR 506.4)"
    );

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
        .expect("their untapped Elf is offered as a blocker");
    assert!(
        offer.attackers.contains(&attacker),
        "untapping an attacker does not remove it from combat (CR 506.4), so \
         the block is still declared against it: {offer:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .expect("the pairing came out of the offer the engine published");

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::Ending);
    assert_eq!(
        engine.state().object(attacker).map(|o| o.zone),
        Some(Zone::Battlefield),
        "\"prevent all combat damage that would be dealt to ... that \
         creature\": one point from a printed 1/1 would have been lethal, and \
         the attacker took none of it"
    );
    assert_eq!(
        engine.state().object(blocker).map(|o| o.zone),
        Some(Zone::Battlefield),
        "and \"... that would be dealt by that creature\": the same one point \
         would have killed the blocker, and the blocker took none of it either"
    );

    // The control: the identical board, attack and block with the Horse left
    // out of the line that matters. Without the activation the two printed
    // 1/1s kill each other, which is what makes the pair above a statement
    // about the card rather than about a damage step that never happened.
    let mut control = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ebony_horse(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut control);
    assert!(
        walk_to_own_main(&mut control, p0),
        "p0 reaches its own main"
    );
    let plain_attacker = all_on_battlefield(&control, p0, llanowar_elves())[0];
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
    pass_until(&mut control, |e| e.state().turn.phase == Phase::Ending);
    assert_eq!(
        (
            control.state().object(plain_attacker).map(|o| o.zone),
            control.state().object(plain_blocker).map(|o| o.zone),
        ),
        (Some(Zone::Graveyard), Some(Zone::Graveyard)),
        "with no Horse activation the blocked 1/1 and the 1/1 that blocked it \
         trade their one damage each and both die, so the survivors above \
         survived because the damage was prevented"
    );
}
