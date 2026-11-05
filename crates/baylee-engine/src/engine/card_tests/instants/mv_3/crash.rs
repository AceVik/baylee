//! `cards/instants/mv_3/crash.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn subject_card() -> CardIndex {
    card_index("562af5da-1080-46d2-8a38-71f06b7e8457")
}

/// P0 holds Crash over `lands`; P1 controls an artifact.
fn crash_over(lands: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, lands)
        .battlefield(1, &[index::JAYEMDAE_TOME])
        .hand(0, &[subject_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Casts Crash for its alternative cost, aimed at P1's artifact, up to the
/// question of what to sacrifice, and returns what it offers.
fn offered_sacrifices(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    let spell = in_hand(engine, P0, subject_card()).unwrap();
    let tome = on_battlefield(engine, P1, index::JAYEMDAE_TOME).unwrap();
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    // Without the mana for its cost there is nothing to choose between.
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let alternative = choose_cast_kind(engine, CastModeKind::Alternative(0));
        engine
            .apply(P0, PlayerAction::ChooseMode(alternative))
            .unwrap();
    }
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![tome],
                players: vec![],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the sacrifice, got {:?}", engine.pending());
    };
    options
}

/// Crash — {2}{R} Instant: "Destroy target artifact", for its mana cost,
/// with no land sacrificed.
#[test]
fn crash_destroys_target_artifact_for_its_mana_cost() {
    let mut engine = crash_over(&[mountain(), mountain(), mountain()]);
    let tome = on_battlefield(&engine, P1, index::JAYEMDAE_TOME).unwrap();
    cast_from_hand(&mut engine, P0, subject_card());
    let normal = choose_cast_kind(&engine, CastModeKind::Normal);
    engine.apply(P0, PlayerAction::ChooseMode(normal)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, P0, tome);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::JAYEMDAE_TOME).is_none());
    assert!(in_graveyard(&engine, P1, index::JAYEMDAE_TOME).is_some());
    assert!(
        in_graveyard(&engine, P0, mountain()).is_none(),
        "the mana cost sacrifices nothing"
    );
}

/// "You may sacrifice a Mountain rather than pay this spell's mana cost":
/// one Mountain and no mana cast it, and a Forest is no Mountain.
#[test]
fn crash_sacrifices_a_mountain_instead_of_paying_mana() {
    let mut engine = crash_over(&[mountain(), forest()]);
    let land = on_battlefield(&engine, P0, mountain()).unwrap();
    let green = on_battlefield(&engine, P0, forest()).unwrap();
    let offered = offered_sacrifices(&mut engine);
    assert_eq!(offered, vec![land], "only the Mountain, not {green:?}");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no mana was needed"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, mountain()).is_some());
    assert!(in_graveyard(&engine, P1, index::JAYEMDAE_TOME).is_some());
    assert!(in_graveyard(&engine, P0, subject_card()).is_some());
    assert!(on_battlefield(&engine, P0, forest()).is_some());
}

/// "A Mountain" is the land type, so a land that merely has it pays too.
#[test]
fn crash_takes_any_land_with_the_mountain_type() {
    let mut engine = crash_over(&[index::TAIGA]);
    let taiga = on_battlefield(&engine, P0, index::TAIGA).unwrap();
    assert_eq!(offered_sacrifices(&mut engine), vec![taiga]);
}
