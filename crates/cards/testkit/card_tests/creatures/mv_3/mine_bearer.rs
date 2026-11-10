//! `cards/creatures/mv_3/mine_bearer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn mine_bearer() -> CardIndex {
    card_index("19fba3c8-3363-4f2f-a2fb-01e59a77447c")
}

/// Mine Bearer ({2}{W} 1/1 Creature — Human Soldier):
/// "{T}, Sacrifice this creature: Destroy target attacking creature."
#[test]
fn mine_bearer_sacrifices_itself_to_destroy_target_attacking_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = mine_bearer();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[subject])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );

    let attacker = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    activate(&mut engine, p0, subject, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, subject).is_none());
    assert!(in_graveyard(&engine, p0, subject).is_some());
    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
}

/// Mine Bearer cannot activate its ability when no creature is attacking.
#[test]
fn mine_bearer_cannot_activate_without_an_attacking_creature() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = mine_bearer();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[subject])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bearer = on_battlefield(&engine, p0, subject).unwrap();
    assert!(
        !priority_offer(&engine).abilities.contains(&(bearer, 0)),
        "Mine Bearer should not be legal to activate when no creature is attacking"
    );
}
