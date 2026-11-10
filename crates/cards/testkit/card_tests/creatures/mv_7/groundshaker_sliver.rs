//! Oracle checklist: own Sliver creatures, including Groundshaker, gain trample; own non-Sliver and opposing Sliver do not; blocked excess reaches the defending player.
#[allow(clippy::wildcard_imports)]
use super::*;

fn groundshaker_sliver() -> CardIndex {
    card_index("106e343f-c6f4-4155-ad01-63e482d4d38e")
}

/// Galerider Sliver, the other Sliver on the board: a plain Sliver body for
/// the anthem to reach.
fn galerider_sliver() -> CardIndex {
    card_index("1ad78038-44f9-4599-84db-fc1a87d9ed45")
}

#[test]
fn groundshaker_grants_trample_to_your_slivers_and_pushes_excess_through_a_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[groundshaker_sliver(), muscle_sliver(), llanowar_elves()],
        )
        .battlefield(1, &[muscle_sliver(), fangren_hunter()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let source = on_battlefield(&e, p0, groundshaker_sliver()).unwrap();
    let own_sliver = on_battlefield(&e, p0, muscle_sliver()).unwrap();
    let own_elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let enemy_sliver = on_battlefield(&e, p1, muscle_sliver()).unwrap();
    assert!(keywords(&e, source).contains(KeywordSet::TRAMPLE));
    assert!(keywords(&e, own_sliver).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&e, own_elf).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&e, enemy_sliver).contains(KeywordSet::TRAMPLE));

    let blocker = on_battlefield(&e, p1, fangren_hunter()).unwrap();
    reach_their_main_phase(&mut e, p1);
    pass_until(&mut e, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(source, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = e.pending().clone()
    else {
        panic!("defender chooses blocks");
    };
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == blocker && b.attackers.contains(&source))
    );
    e.apply(
        player,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, source)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        e.state().players[1].life,
        17,
        "both Muscle Slivers give Groundshaker +2/+2, so it assigns four lethal and three excess"
    );
    assert!(in_graveyard(&e, p1, fangren_hunter()).is_some());
}

/// Groundshaker Sliver — a 5/5 for {6}{G} that gives Slivers you control
/// trample. It is a Sliver itself, so it has it too; so has my other Sliver;
/// the opposing Sliver and my Ogre have not.
#[test]
fn groundshaker_sliver_gives_trample_to_slivers_i_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let engine = table7(
        &[groundshaker_sliver(), galerider_sliver(), gray_ogre()],
        &[],
        &[galerider_sliver()],
    );
    let ground = on_battlefield(&engine, p0, groundshaker_sliver()).expect("out");
    let mine = on_battlefield(&engine, p0, galerider_sliver()).expect("out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    let theirs = on_battlefield(&engine, p1, galerider_sliver()).expect("out");
    assert_eq!(pt(&engine, ground), (5, 5));
    assert!(
        keywords(&engine, ground).contains(KeywordSet::TRAMPLE),
        "itself"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "my Sliver"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "their Sliver is not mine"
    );
    assert!(
        !keywords(&engine, ogre).contains(KeywordSet::TRAMPLE),
        "an Ogre is not a Sliver"
    );
}

/// Cast off {6}{G}, a 5/5; and the trample is real in combat: 3 of its 5
/// tramples over a 2/2.
#[test]
fn groundshaker_sliver_costs_seven() {
    let mut engine = table7(&lots(forest(), 7), &[groundshaker_sliver()], &[]);
    let sliver = cast_and_resolve(&mut engine, groundshaker_sliver());
    assert_body(&engine, sliver, (5, 5), KeywordSet::TRAMPLE);
}

#[test]
fn groundshaker_sliver_tramples_over_a_blocker() {
    let (mut engine, sliver, ogre) = duel_vs_ogre(groundshaker_sliver(), &[]);
    fight(&mut engine, &[sliver], &[(ogre, sliver)]);
    assert_eq!(life_of(&engine, 1), 17);
}
