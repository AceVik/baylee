//! `cards/instants/mv_1/engulfing_flames.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn engulfing_flames() -> CardIndex {
    card_index("88f935ee-7dbb-4c27-8485-f56b4193fa11")
}

#[track_caller]
fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, flames: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: flames })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// The one damage is dealt to the chosen creature, and its prohibition defeats
/// a regeneration shield that was activated before Engulfing Flames resolves.
#[test]
fn engulfing_flames_deals_one_and_prevents_regeneration() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), index::GRIZZLY_BEARS])
        .hand(0, &[engulfing_flames()])
        .battlefield(1, &[index::DRUDGE_SKELETONS, index::SWAMP])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let skeletons = on_battlefield(&engine, p1, index::DRUDGE_SKELETONS).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, engulfing_flames());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a creature target, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 2);
    assert!(options.contains(&bears) && options.contains(&skeletons));
    assert!(
        player_options.is_empty(),
        "only creatures are legal targets"
    );
    aim_at(&mut engine, p0, skeletons);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, index::DRUDGE_SKELETONS, 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, index::DRUDGE_SKELETONS).is_some());
    assert!(on_battlefield(&engine, p1, index::DRUDGE_SKELETONS).is_none());
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
    assert_eq!(life_of(&engine, p1), 20);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(in_graveyard(&engine, p0, engulfing_flames()).is_some());
}

/// Flashback costs exactly {3}{R}, deals the same one damage, and exiles the
/// spell after resolution instead of returning it to the graveyard.
#[test]
fn engulfing_flames_flashback_costs_four_deals_one_and_exiles() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 4])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[engulfing_flames()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let flames = hand_to_graveyard(&mut engine, p0, engulfing_flames());
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    start_flashback(&mut engine, p0, flames);
    aim_at(&mut engine, p0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(flames).unwrap().zone, Zone::Stack);

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);
    assert_eq!(life_of(&engine, p1), 20);
    assert_eq!(engine.state().object(flames).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, engulfing_flames()).is_none());
}

/// Three red mana are one short of flashback's {3}{R}; the ordinary {R} cost
/// does not make the graveyard incarnation castable.
#[test]
fn engulfing_flames_flashback_is_not_offered_with_only_three_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 3])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[engulfing_flames()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let flames = hand_to_graveyard(&mut engine, p0, engulfing_flames());
    let bears = on_battlefield(&engine, PlayerId::new(1), index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&flames));
    assert!(!legal.payable.contains(&flames));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: flames })
            .is_err()
    );
    assert_eq!(engine.state().object(flames).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
}
