//! `cards/creatures/artifacts/mv_3/soldier_replica.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn soldier_replica() -> CardIndex {
    card_index("702693f0-09f6-4f19-8dc9-6b31641e3347")
}

/// Soldier Replica — "{1}{W}, Sacrifice this creature: It deals 3 damage to
/// target attacking or blocking creature." In seat 1's attack the menu holds
/// the attacker and not the creature that stayed home.
#[test]
fn soldier_replica_deals_three_to_an_attacking_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soldier_replica(), plains(), forest()])
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let replica = on_battlefield(&engine, p0, soldier_replica()).expect("seated");
    assert_eq!(pt(&engine, replica), (1, 3));
    let theirs = all_on_battlefield(&engine, p1, quiet_creature());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1)
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(theirs[0], Defender::Player(p0))],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "seat 0 holds priority with the attacker declared"
    );
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, soldier_replica(), 0);
    let options = aim_at(&mut engine, p0, theirs[0]);
    assert!(options.contains(&theirs[0]));
    assert!(!options.contains(&theirs[1]), "not the creature that stayed home");
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, soldier_replica()).is_some(), "sacrificed");
    assert!(
        in_graveyard(&engine, p1, quiet_creature()).is_some(),
        "3 damage killed the attacker"
    );
    assert_eq!(all_on_battlefield(&engine, p1, quiet_creature()).len(), 1);
}
