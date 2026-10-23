#[allow(clippy::wildcard_imports)]
use super::*;

fn wall_of_nets() -> CardIndex {
    card_index("cd112962-b297-403c-bede-c8b587f379b3")
}

#[test]
fn wall_of_nets_exiles_blocked_creature_at_end_of_combat() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[wall_of_nets()])
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, wall_of_nets());
    pass_until(&mut engine, stack_is_empty);
    let wall = on_battlefield(&engine, p0, wall_of_nets()).expect("seated");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .any(|&id| {
                engine
                    .state()
                    .object(id)
                    .unwrap()
                    .card
                    .is_some_and(|c| c.index == grizzly_bears())
            })
    );
}
