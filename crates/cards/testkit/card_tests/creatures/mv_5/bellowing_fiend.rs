//! `cards/creatures/mv_5/bellowing_fiend.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bellowing_fiend() -> CardIndex {
    card_index("37331d0e-edda-4791-8bcc-fb457f984877")
}

/// Dealing combat damage to a flying blocker deals 3 damage to that creature's
/// controller and 3 damage to Bellowing Fiend's controller, even when the
/// blocker dies from lethal combat damage.
#[test]
fn bellowing_fiend_deals_three_to_both_players_when_damaging_an_opposing_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 5])
        .battlefield(1, &[storm_crow()])
        .hand(0, &[bellowing_fiend()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, bellowing_fiend());
    pass_until(&mut engine, stack_is_empty);
    let fiend = on_battlefield(&engine, p0, bellowing_fiend()).expect("resolved");
    assert_eq!(pt(&engine, fiend), (3, 3));
    assert!(keywords(&engine, fiend).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let crow = on_battlefield(&engine, p1, storm_crow()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, fiend, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(crow, fiend)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_graveyard(&engine, p1, storm_crow()).is_some());
    assert!(on_battlefield(&engine, p0, bellowing_fiend()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        17,
        "opponent controlled the damaged Storm Crow"
    );
    assert_eq!(
        engine.state().players[0].life,
        17,
        "Bellowing Fiend's controller also takes 3"
    );
}

/// Unblocked combat damage hits a player rather than a creature, so the
/// creature-damage trigger does not fire.
#[test]
fn bellowing_fiend_does_not_trigger_when_dealing_damage_to_a_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[bellowing_fiend()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fiend = on_battlefield(&engine, p0, bellowing_fiend()).unwrap();
    let blocks = attack_and_collect_blocks(&mut engine, fiend, p1);
    assert!(blocks.iter().all(|opt| !opt.attackers.contains(&fiend)));
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 17, "only 3 combat damage");
    assert_eq!(engine.state().players[0].life, 20, "no self-damage trigger");
}
