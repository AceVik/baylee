//! `cards/instants/mv_1/emerald_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::WIND_DRAKE;

fn emerald_charm() -> CardIndex {
    card_index("904b3db5-70bc-4d49-9887-f504aff8666f")
}

#[test]
fn emerald_charm_untaps_permanent_or_removes_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), grizzly_bears()])
        .battlefield(1, &[WIND_DRAKE])
        .hand(0, &[emerald_charm(), emerald_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let drake = on_battlefield(&engine, p1, WIND_DRAKE).unwrap();
    engine.state.set_tapped(bear, true);

    tap_all_mana(&mut engine, p0);

    // Mode 0: Untap target permanent.
    cast_with_floating(&mut engine, p0, emerald_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let slot0 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(slot0)).unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, bear));

    // Mode 2: Target creature loses flying until end of turn.
    cast_with_floating(&mut engine, p0, emerald_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    let slot2 = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(2)))
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseMode(slot2)).unwrap();
    aim_at(&mut engine, p0, drake);
    pass_until(&mut engine, stack_is_empty);
    assert!(!keywords(&engine, drake).contains(KeywordSet::FLYING));
    reach_their_main_phase(&mut engine, p1);
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "\"until end of turn\": the Drake flies again on the next turn"
    );
}

/// Picks mode `n` of the Charm that was just cast.
#[track_caller]
fn choose_mode(engine: &mut Engine<RegistryLookup>, seat: PlayerId, n: usize) {
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("a mode question, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(m) if m == n))
        .expect("the mode is offered");
    engine.apply(seat, PlayerAction::ChooseMode(slot)).unwrap();
}

/// "Destroy target non-Aura enchantment." An Aura on the battlefield and a creature are not on
/// the menu and are refused; the non-Aura enchantment is destroyed and the Aura stays put.
#[test]
fn emerald_charm_destroys_a_non_aura_enchantment_but_never_an_aura_or_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), grizzly_bears()])
        .battlefield(1, &[sylvan_library()])
        .hand(0, &[rancor(), emerald_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let library = on_battlefield(&engine, p1, sylvan_library()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, rancor());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let rancor_aura = on_battlefield(&engine, p0, rancor()).expect("the Aura is on the Bears");

    cast_with_floating(&mut engine, p0, emerald_charm());
    choose_mode(&mut engine, p0, 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target question, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&library),
        "a non-Aura enchantment is a target"
    );
    assert!(
        !options.contains(&rancor_aura),
        "an Aura enchantment is not"
    );
    assert!(!options.contains(&bear), "a creature is not an enchantment");
    for illegal in [rancor_aura, bear] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![illegal],
                        players: vec![],
                    },
                )
                .is_err(),
            "{illegal:?} cannot be named"
        );
    }
    aim_at(&mut engine, p0, library);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, sylvan_library()).is_some(),
        "the enchantment was destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, rancor()).is_some(),
        "the Aura was left alone"
    );
    assert_eq!(
        pt(&engine, bear),
        (4, 2),
        "so the Bears keeps Rancor's +2/+0"
    );
}
