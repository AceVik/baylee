//! `cards/creatures/artifacts/mv_2/perilous_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn perilous_myr() -> CardIndex {
    card_index("f1e8f746-1837-4ea8-9a07-e57946883353")
}

/// Destroys `id` as a state-based action would, then runs to the target
/// question of whatever it triggered.
#[track_caller]
fn die_and_ask(
    engine: &mut Engine<RegistryLookup>,
    id: ObjectId,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    let p0 = PlayerId::new(0);
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::sba::destroy(state, id);
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets { .. } | Pending::ChoosePlayer { .. }
        )
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the Myr's controller chooses");
    (options, player_options)
}

/// Perilous Myr: a 1/1; "when this creature dies, it deals 2 damage to any
/// target". The target question offers players and creatures; naming the
/// opponent costs them two life.
#[test]
fn perilous_myr_deals_two_damage_to_a_player_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[perilous_myr()], &[grizzly_bears()]);
    let myr = on_battlefield(&engine, p0, perilous_myr()).expect("seated");
    assert_eq!(pt(&engine, myr), (1, 1));
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let (objects, players) = die_and_ask(&mut engine, myr);
    assert!(objects.contains(&bears), "any target: a creature");
    assert!(
        players.contains(&p1) && players.contains(&p0),
        "any target: a player"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent is on the menu");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 1), 18);
    assert_eq!(life_of(&engine, 0), 20);
}

/// Two damage kills a 2/2 Bears.
#[test]
fn perilous_myr_kills_a_two_two_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[perilous_myr()], &[grizzly_bears()]);
    let myr = on_battlefield(&engine, p0, perilous_myr()).expect("seated");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    die_and_ask(&mut engine, myr);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .expect("the Bears are on the menu");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(life_of(&engine, 1), 20);
}
