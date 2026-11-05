//! `cards/enchantments/mv_3/sulfuric_vortex.rs`, played from Oracle text.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sulfuric_vortex() -> CardIndex {
    card_index("7652f328-e142-494b-a869-772ced10c26a")
}

/// Casts Healing Salve's first mode for `target` and lets it resolve.
#[track_caller]
fn salve_gain(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    tap_all_mana(engine, P0);
    cast_with_floating(engine, P0, healing_salve());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("Healing Salve chooses a mode: {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|option| matches!(option.kind, CastModeKind::Mode(0)))
        .expect("the life-gain mode is offered");
    engine.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(P0, PlayerAction::ChoosePlayer(target))
        .expect("either player is a legal target");
    pass_until(engine, stack_is_empty);
}

#[test]
fn sulfuric_vortex_pays_its_printed_cost_and_resolves() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 3])
        .hand(0, &[sulfuric_vortex()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, sulfuric_vortex());
    assert!(on_stack(&engine, sulfuric_vortex()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, sulfuric_vortex()).is_some());
    assert!(in_hand(&engine, P0, sulfuric_vortex()).is_none());
}

#[test]
fn sulfuric_vortex_damages_each_player_in_that_players_upkeep() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sulfuric_vortex()])
        .start();
    keep_mulligans(&mut engine);
    let source = on_battlefield(&engine, P0, sulfuric_vortex()).expect("the Vortex is out");
    assert!(!stack_is_empty(&engine), "the first upkeep triggers");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, P0), 18);
    assert_eq!(
        life_of(&engine, P1),
        20,
        "only the active player was dealt 2"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        life_of(&engine, P0),
        18,
        "the opponent's trigger hurts nobody else"
    );
    assert_eq!(life_of(&engine, P1), 18);
    let triggers = engine
        .journal()
        .entries()
        .iter()
        .filter(|entry| {
            matches!(entry.event, GameEvent::AbilityTriggered { source: id, .. } if id == source)
        })
        .count();
    assert_eq!(triggers, 2, "one trigger in each of the two upkeeps");
}

#[test]
fn sulfuric_vortex_replaces_all_life_gains_until_it_leaves() {
    let mut engine = Duel::new(SEED, forest())
        .life(0, 10)
        .life(1, 10)
        .battlefield(
            0,
            &[
                sulfuric_vortex(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(
            0,
            &[
                healing_salve(),
                healing_salve(),
                healing_salve(),
                baylee_core::generated::index::DISENCHANT,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    assert_eq!(life_of(&engine, P0), 8, "the first upkeep dealt its 2");
    let since = engine.journal().last_seq();
    let library_before = engine.state().zones.list(ZoneLocation::Library(P0)).len();

    salve_gain(&mut engine, P0);
    salve_gain(&mut engine, P1);
    assert_eq!(life_of(&engine, P0), 8, "the controller gains no life");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        library_before,
        "no life gain may become a draw instead"
    );
    assert_eq!(
        life_of(&engine, P1),
        10,
        "the opponent gains no life either"
    );
    assert_eq!(engine.state().per_turn.life_gained_amount[0], 0);
    assert_eq!(engine.state().per_turn.life_gained_amount[1], 0);
    let mut new_entries = engine
        .journal()
        .entries()
        .iter()
        .skip(usize::try_from(since).expect("journal length fits usize"));
    assert!(!new_entries.any(|entry| matches!(entry.event, GameEvent::LifeChanged { .. })));

    let source = on_battlefield(&engine, P0, sulfuric_vortex()).expect("the Vortex is still out");
    cast_from_hand(&mut engine, P0, baylee_core::generated::index::DISENCHANT);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Disenchant chooses a target: {:?}", engine.pending())
    };
    assert!(options.contains(&source));
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![source],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, sulfuric_vortex()).is_some());

    salve_gain(&mut engine, P0);
    assert_eq!(
        life_of(&engine, P0),
        11,
        "with the Vortex gone, Salve gains 3"
    );
    assert_eq!(engine.state().per_turn.life_gained_amount[0], 3);
}
