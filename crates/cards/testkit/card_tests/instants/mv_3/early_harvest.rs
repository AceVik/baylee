//! `cards/instants/mv_3/early_harvest.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::CITY_OF_BRASS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn early_harvest() -> CardIndex {
    card_index("32001382-3079-4bbe-8d86-05da654c0d1a")
}

fn target_yourself(engine: &mut Engine<RegistryLookup>) {
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { player, .. } if *player == P0),
        "the supported self-only subset still requires a real target"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![]
                }
            )
            .is_err(),
        "the target is mandatory"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1]
                }
            )
            .is_err(),
        "other recipients remain unsupported"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
}

/// Early Harvest (32001382-3079-4bbe-8d86-05da654c0d1a) — {1}{G}{G} Instant:
/// "Target player untaps all basic lands they control."
#[test]
fn early_harvest_untaps_all_basic_lands_controlled_by_caster() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), plains(), CITY_OF_BRASS])
        .hand(0, &[early_harvest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let brass = on_battlefield(&engine, P0, CITY_OF_BRASS).unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(brass)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, early_harvest());
    target_yourself(&mut engine);
    pass_until(&mut engine, stack_is_empty);

    for &id in engine.state().zones.list(ZoneLocation::Battlefield) {
        let obj = engine.state().object(id).unwrap();
        if obj.controller == P0
            && obj
                .characteristics()
                .supertypes
                .contains(SupertypeSet::BASIC)
        {
            assert!(!obj.status.contains(Status::TAPPED));
        }
    }
    assert!(
        engine
            .state()
            .object(brass)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

/// The card is Partial: it targets the caster and untaps their basic lands only. Another
/// player's tapped basic land stays tapped, so the spell never does more than
/// it prints for its caster's side.
#[test]
fn early_harvest_leaves_an_opponents_tapped_basic_lands_tapped() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), plains()])
        .battlefield(1, &[forest(), plains()])
        .hand(0, &[early_harvest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let theirs: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().controller == P1)
        .collect();
    assert_eq!(theirs.len(), 2);
    for &id in &theirs {
        engine
            .dev_state_mut(P0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }
    engine.refresh_offer();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, early_harvest());
    target_yourself(&mut engine);
    pass_until(&mut engine, stack_is_empty);

    for &id in &theirs {
        assert!(
            engine
                .state()
                .object(id)
                .unwrap()
                .status
                .contains(Status::TAPPED)
        );
    }
}
