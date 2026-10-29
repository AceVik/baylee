//! Nefashu — `{4}{B}{B}` 5/3 Creature — Zombie Mutant (SCG).
//! Oracle: "Whenever this creature attacks, up to five target creatures each get -1/-1 until end of turn."

use super::*;

fn nefashu() -> CardIndex {
    card_index("9391f8ee-3435-400c-8a50-79a75de18a90")
}

#[test]
fn nefashu_attack_trigger_shrinks_up_to_five_target_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = nefashu();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[llanowar_elves(), grizzly_bears(), gray_ogre()])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);

    let nefashu_id = on_battlefield(&engine, p0, card).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    // Advance to p0's next combat so Nefashu is no longer summoning-sick.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0 && e.state().turn.number >= 3),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(nefashu_id, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Choose two of the three opposing creatures for the up-to-five target prompt.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves, bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 1/1 Llanowar Elves dies to SBA; 2/2 Grizzly Bears is now 1/1; untargeted Gray Ogre stays 2/2.
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert_eq!(pt(&engine, bears), (1, 1));
    assert_eq!(pt(&engine, ogre), (2, 2));
}
