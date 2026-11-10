//! `cards/instants/mv_3/spore_cloud.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn spore_cloud() -> CardIndex {
    card_index("36d55084-4696-4d16-ab3a-b00f1456603f")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Passes priority until `seat` holds it.
#[track_caller]
fn priority_for(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if player == seat => return,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("expected priority on the way to {seat:?}, got {other:?}"),
        }
    }
    panic!("{seat:?} never got priority");
}

fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .expect("the creature is on the battlefield")
        .status
        .contains(Status::TAPPED)
}

/// Spore Cloud (36d55084-4696-4d16-ab3a-b00f1456603f) — "Tap all blocking
/// creatures. Prevent all combat damage that would be dealt this turn. Each
/// attacking creature and each blocking creature doesn't untap during its
/// controller's next untap step."
///
/// p1's Giant attacks, my Bears block, and I cast the Cloud: the Bears are
/// tapped, nobody is hurt, and neither the Bears nor the Giant untap in
/// their controller's next untap step. My Lions, which took no part, do.
#[test]
fn spore_cloud_taps_blockers_prevents_combat_damage_and_holds_the_combatants_down() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                grizzly_bears(),
                savannah_lions(),
            ],
        )
        .hand(0, &[spore_cloud()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions are out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");

    attack_and_collect_blocks(&mut engine, giant, p0);
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, giant)],
            },
        )
        .expect("the Bears block the Giant");
    priority_for(&mut engine, p0);
    assert!(!is_tapped(&engine, bears), "a blocker is untapped");

    cast_from_hand(&mut engine, p0, spore_cloud());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, bears),
        "all blocking creatures are tapped"
    );
    assert!(!is_tapped(&engine, lions), "the Lions blocked nothing");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    let bears_object = engine.state().object(bears).expect("the Bears survived");
    assert_eq!(bears_object.zone, Zone::Battlefield);
    assert_eq!(bears_object.damage, 0, "combat damage was prevented");
    assert_eq!(
        engine
            .state()
            .object(giant)
            .expect("the Giant is there")
            .damage,
        0,
        "in both directions"
    );
    assert_eq!(engine.state().players[0].life, 20);

    // My next untap step: the Bears stay down.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        is_tapped(&engine, bears),
        "the Bears did not untap in their controller's next untap step"
    );
    // The Giant attacked, so it stays down through p1's next untap step.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        is_tapped(&engine, giant),
        "the Giant did not untap in its controller's next untap step"
    );
}
