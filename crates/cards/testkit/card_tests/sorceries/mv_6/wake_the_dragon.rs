//! Wake the Dragon and its combat-damage trigger, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_engine::turn::Phase;

#[test]
fn wake_the_dragon_steals_the_damaged_players_artifact_and_flashes_back() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let wake = card_index("cfa5b168-d112-4b92-ac42-8bba30f15d77");
    let mut battlefield = vec![swamp(), mountain(), sol_ring()];
    battlefield.extend([plains(); 6]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &battlefield)
        .battlefield(1, &[sol_ring(), plains()])
        .hand(0, &[wake])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ours = on_battlefield(&engine, p0, sol_ring()).expect("our Ring is on the battlefield");
    let theirs = on_battlefield(&engine, p1, sol_ring()).expect("their Ring is on the battlefield");
    cast_from_hand(&mut engine, p0, wake);
    pass_until(&mut engine, stack_is_empty);
    let [dragon] = tokens_of(&engine, p0)[..] else {
        panic!("the first cast makes exactly one Dragon");
    };
    assert_eq!(pt(&engine, dragon), (6, 6));
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING.union(KeywordSet::MENACE)));
    let used = in_graveyard(&engine, p0, wake).expect("the spell went to the graveyard");

    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dragon, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(options.as_slice(), &[theirs]);
    assert!(!options.contains(&ours));
    assert_eq!(super::life_of(&engine, p1), 14);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(theirs).unwrap().controller, p0);

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: used })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let mode = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(tokens_of(&engine, p0).len(), 2);
    assert_eq!(engine.state().object(used).unwrap().zone, Zone::Exile);
}
