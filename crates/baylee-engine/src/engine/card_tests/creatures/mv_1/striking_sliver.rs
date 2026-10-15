//! Oracle checklist: your Sliver creatures gain first strike, including the source; own non-Sliver and opposing Sliver do not; first strike kills a 1/1 blocker before it deals return damage.
#[allow(clippy::wildcard_imports)]
use super::*;

fn striking_sliver() -> CardIndex {
    card_index("6a15599d-8b00-4e0f-ac0c-d05f07c1e50d")
}

#[test]
fn striking_sliver_grants_first_strike_to_your_slivers_only() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[striking_sliver(), muscle_sliver(), llanowar_elves()])
        .battlefield(1, &[muscle_sliver(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let source = on_battlefield(&e, p0, striking_sliver()).unwrap();
    let own_sliver = on_battlefield(&e, p0, muscle_sliver()).unwrap();
    let own_elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let enemy_sliver = on_battlefield(&e, p1, muscle_sliver()).unwrap();
    assert!(keywords(&e, source).contains(KeywordSet::FIRST_STRIKE));
    assert!(keywords(&e, own_sliver).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&e, own_elf).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&e, enemy_sliver).contains(KeywordSet::FIRST_STRIKE));

    let blocker = on_battlefield(&e, p1, llanowar_elves()).unwrap();
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
    assert!(
        on_battlefield(&e, p0, striking_sliver()).is_some(),
        "the 2/2 survived combat"
    );
    assert!(
        in_graveyard(&e, p1, llanowar_elves()).is_some(),
        "first-strike damage removed the 1/1 before normal damage"
    );
    assert_eq!(e.state().object(source).unwrap().damage, 0);
}
