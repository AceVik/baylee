//! `cards/creatures/mv_8/bloodfire_colossus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn bloodfire_colossus() -> CardIndex {
    card_index("cc59e28a-eda3-468c-a31b-73e4b615f953")
}

/// Bloodfire Colossus is a 6/6 for {6}{R}{R}.
#[test]
fn bloodfire_colossus_is_cast_for_eight_as_a_six_six() {
    let mut engine = at_main(&[mountain(); 8], &[bloodfire_colossus()]);
    let colossus = cast_and_settle(&mut engine, bloodfire_colossus());
    assert_eq!(pt(&engine, colossus), (6, 6));
}

/// "{R}, Sacrifice this creature: It deals 6 damage to each creature and
/// each player": the Colossus goes to its owner's graveyard as the cost, every
/// creature left (the opponent's Bears, and the controller's own) takes 6
/// and dies, and both players lose 6, the controller included.
#[test]
fn bloodfire_colossus_sacrificed_hits_every_creature_and_every_player_for_six() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bloodfire_colossus(), mountain(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let (mine, theirs) = (life_of(&engine, 0), life_of(&engine, 1));
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, bloodfire_colossus(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, bloodfire_colossus()).is_some(),
        "sacrificed as a cost"
    );
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_none(),
        "its controller's own creature takes the 6"
    );
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "so does the opponent's"
    );
    assert_eq!(
        life_of(&engine, 0),
        mine - 6,
        "each player, the controller too"
    );
    assert_eq!(life_of(&engine, 1), theirs - 6);
}

/// The Colossus does nothing without the {R}: the ability is not offered
/// with an empty pool.
#[test]
fn bloodfire_colossus_needs_red_mana_to_be_sacrificed() {
    let p0 = PlayerId::new(0);
    let engine = at_main(&[bloodfire_colossus()], &[]);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let colossus = on_battlefield(&engine, p0, bloodfire_colossus()).expect("it is out");
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == colossus),
        "no mana, no activation: {:?}",
        legal.abilities
    );
}
