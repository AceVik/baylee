//! `cards/creatures/mv_2/kjeldoran_guard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn kjeldoran_guard() -> CardIndex {
    card_index("bd699cf8-56c7-4375-920e-8dd22c83a82c")
}

fn snow_covered_forest() -> CardIndex {
    card_index("5f0d3be8-e63e-4ade-ae58-6b0c14f2ce6d")
}

#[test]
fn kjeldoran_guard_requires_combat_and_no_snow_land_controlled_by_the_defender() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[kjeldoran_guard(), snow_covered_forest()])
        .battlefield(1, &[snow_covered_forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p0, kjeldoran_guard()).unwrap();

    assert!(
        !ability_offered(&engine, guard, 0),
        "the ability cannot be activated during a main phase"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::Combat);
    assert!(
        !ability_offered(&engine, guard, 0),
        "the defending player's Snow-Covered Forest prevents activation"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: guard,
                    ability_index: 0,
                },
            )
            .is_err(),
        "an illegally submitted activation is refused"
    );
}

#[test]
fn kjeldoran_guard_pumps_any_creature_and_sacrifices_itself_when_that_creature_leaves() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                kjeldoran_guard(),
                grizzly_bears(),
                snow_covered_forest(),
                island(),
            ],
        )
        .hand(0, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p0, kjeldoran_guard()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::Combat);
    assert!(
        ability_offered(&engine, guard, 0),
        "the controller's own snow land does not prevent activation"
    );
    activate(&mut engine, p0, kjeldoran_guard(), 0);
    let options = aim_at(&mut engine, p0, bears);
    assert!(
        options.contains(&guard) && options.contains(&bears),
        "any creature is a legal target, including the Guard itself"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (3, 3));

    // Returning the target to hand is leaving the battlefield, not dying.
    cast_from_hand(&mut engine, p0, unsummon());
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
    assert_eq!(
        engine.state().object(guard).unwrap().zone,
        Zone::Graveyard,
        "the linked delayed trigger sacrifices the Guard this turn"
    );
}

#[test]
fn kjeldoran_guard_pump_and_linked_sacrifice_expire_at_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[kjeldoran_guard(), grizzly_bears(), island()])
        .hand(0, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::Combat);
    activate(&mut engine, p0, kjeldoran_guard(), 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (3, 3));

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, bears), (2, 2), "the pump has expired");
    cast_from_hand(&mut engine, p0, unsummon());
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
    assert!(
        on_battlefield(&engine, p0, kjeldoran_guard()).is_some(),
        "a later turn's departure no longer sacrifices the Guard"
    );
}

/// "Defending player" is whoever is attacked, not "the opponent": on the
/// opponent's attack only the Guard's controller's snow lands matter.
#[test]
fn kjeldoran_guard_reads_the_snow_lands_of_the_player_being_attacked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (mine, theirs, offered) in [
        (vec![snow_covered_forest()], vec![], false),
        (vec![], vec![snow_covered_forest()], true),
    ] {
        let mut board = vec![kjeldoran_guard()];
        board.extend(mine);
        let mut opposing = vec![grizzly_bears()];
        opposing.extend(theirs);
        let mut engine = Duel::new(SEED, plains())
            .battlefield(0, &board)
            .battlefield(1, &opposing)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let guard = on_battlefield(&engine, p0, kjeldoran_guard()).unwrap();
        let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

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
            e.state().turn.phase == Phase::Combat
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
        });
        assert_eq!(ability_offered(&engine, guard, 0), offered);
    }
}
