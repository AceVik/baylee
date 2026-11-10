//! `cards/creatures/mv_4/banshee.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn banshee() -> CardIndex {
    card_index("438546b8-0b67-4bec-a43b-f2fc7449c90e")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn float(engine: &mut Engine<RegistryLookup>, seat: usize, mana: &[(ManaColor, u32)]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for (color, n) in mana {
        state.players[seat].mana_pool.add(*color, *n);
    }
    engine.refresh_offer();
}

/// Activates the Banshee for `x`, naming the target when asked, and lets it
/// resolve.
fn fire(engine: &mut Engine<RegistryLookup>, x: u32, objects: &[ObjectId], players: &[PlayerId]) {
    let p0 = PlayerId::new(0);
    float(engine, 0, &[(ManaColor::Colorless, 10)]);
    activate(engine, p0, banshee(), 0);
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine
                    .apply(p0, PlayerAction::ChooseNumber(x))
                    .expect("X is within bounds");
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: objects.to_vec(),
                            players: players.to_vec(),
                        },
                    )
                    .expect("a legal target");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected question: {other:?}"),
        }
    }
    pass_until(engine, stack_is_empty);
}

fn board() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[banshee()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, PlayerId::new(0)),
        "p0 reaches its own main"
    );
    engine
}

/// Banshee (438546b8-0b67-4bec-a43b-f2fc7449c90e) — "{X}, {T}: This creature
/// deals half X damage, rounded down, to any target, and half X damage,
/// rounded up, to you."
#[test]
fn banshee_with_an_odd_x_hurts_you_more_than_the_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let banshee_id = on_battlefield(&engine, p0, banshee()).expect("the Banshee is out");
    fire(&mut engine, 5, &[], &[p1]);
    assert_eq!(engine.state().players[1].life, 18, "half of 5 rounded down");
    assert_eq!(engine.state().players[0].life, 17, "half of 5 rounded up");
    assert!(is_tapped(&engine, banshee_id), "{{T}} is part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "X = 5 paid out of 10"
    );
}

#[test]
fn banshee_with_an_even_x_splits_evenly() {
    let p1 = PlayerId::new(1);
    let mut engine = board();
    fire(&mut engine, 4, &[], &[p1]);
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 18);
}

/// X = 1: nothing for the target, one for you.
#[test]
fn banshee_with_x_of_one_only_hurts_you() {
    let p1 = PlayerId::new(1);
    let mut engine = board();
    fire(&mut engine, 1, &[], &[p1]);
    assert_eq!(engine.state().players[1].life, 20, "half of 1 rounded down");
    assert_eq!(engine.state().players[0].life, 19, "half of 1 rounded up");
}

/// Any target: a creature takes the rounded-down half.
#[test]
fn banshee_can_aim_at_a_creature() {
    let p1 = PlayerId::new(1);
    let mut engine = board();
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant is out");
    fire(&mut engine, 5, &[giant], &[]);
    assert_eq!(
        engine
            .state()
            .object(giant)
            .expect("the Giant lives")
            .damage,
        2,
        "half of 5 rounded down on the 3/3"
    );
    assert_eq!(engine.state().players[0].life, 17, "you take the rest");
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn banshee_with_a_big_x_kills_the_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant is out");
    fire(&mut engine, 6, &[giant], &[]);
    assert!(
        in_graveyard(&engine, p1, hill_giant()).is_some(),
        "3 damage kills the 3/3"
    );
    assert_eq!(engine.state().players[0].life, 17);
    assert!(
        on_battlefield(&engine, p0, banshee()).is_some(),
        "the Banshee lives"
    );
}
