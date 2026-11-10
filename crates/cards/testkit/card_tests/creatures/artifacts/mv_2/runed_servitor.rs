//! `cards/creatures/artifacts/mv_2/runed_servitor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn runed_servitor() -> CardIndex {
    card_index("4e5e81e1-ab47-4f08-9c42-dba0adc5cf9a")
}

/// Runed Servitor: a 2/2; "when this creature dies, each player draws a
/// card". Both draw exactly one.
#[test]
fn runed_servitor_makes_each_player_draw_when_it_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = table(&[], &[runed_servitor()], &[]);
    let servitor = on_battlefield(&engine, p0, runed_servitor()).expect("seated");
    assert_eq!(pt(&engine, servitor), (2, 2));
    let (mine, theirs) = (hand_count(&engine, 0), hand_count(&engine, 1));
    let (lib0, lib1) = (library_size(&engine, p0), library_size(&engine, p1));
    kill(&mut engine, servitor);
    assert!(in_graveyard(&engine, p0, runed_servitor()).is_some());
    assert_eq!(hand_count(&engine, 0), mine + 1);
    assert_eq!(hand_count(&engine, 1), theirs + 1);
    assert_eq!(library_size(&engine, p0), lib0 - 1);
    assert_eq!(library_size(&engine, p1), lib1 - 1);
}

/// Surviving is not dying: blocked by a 0/8 Wall, which deals no damage, the
/// Servitor lives and neither player draws.
#[test]
fn runed_servitor_surviving_combat_makes_nobody_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[runed_servitor()], &[wall_of_stone()]);
    let servitor = on_battlefield(&engine, p0, runed_servitor()).expect("seated");
    let wall = on_battlefield(&engine, p1, wall_of_stone()).expect("seated");
    let (mine, theirs) = (hand_count(&engine, 0), hand_count(&engine, 1));
    let options = attack_and_collect_blocks(&mut engine, servitor, p1);
    assert!(options.iter().any(|o| o.blocker == wall));
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, servitor)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, p0, runed_servitor()).is_some(),
        "a 0/8 Wall deals no damage: the Servitor lives"
    );
    assert_eq!(hand_count(&engine, 0), mine);
    assert_eq!(hand_count(&engine, 1), theirs);
}
